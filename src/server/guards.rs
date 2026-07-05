use deadpool_postgres::Pool;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use rocket::State;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use uuid::Uuid;

use crate::models::{AuthError, Claims};

/// Authenticated user attached to every protected request.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let jwt_secret = match std::env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(_) => {
                eprintln!("JWT_SECRET not set");
                return Outcome::Error((Status::InternalServerError, AuthError::InvalidToken));
            }
        };

        // Try cookie first, then Authorization header
        let token = request
            .cookies()
            .get("accessToken")
            .map(|c| c.value().to_string())
            .or_else(|| {
                request
                    .headers()
                    .get_one("Authorization")
                    .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()))
            });

        let token = match token {
            Some(t) => t,
            None => return Outcome::Error((Status::Unauthorized, AuthError::MissingToken)),
        };

        match decode_jwt(&token, &jwt_secret) {
            Ok(claims) => {
                let user_id = match Uuid::parse_str(&claims.sub) {
                    Ok(id) => id,
                    Err(_) => {
                        return Outcome::Error((Status::Unauthorized, AuthError::InvalidToken));
                    }
                };

                // Verify the user still exists in the database
                let valid = match request.guard::<&State<Pool>>().await {
                    Outcome::Success(pool) => match pool.get().await {
                        Ok(client) => client
                            .query_one("SELECT 1 FROM users WHERE id = $1", &[&user_id])
                            .await
                            .is_ok(),
                        Err(_) => false,
                    },
                    _ => false,
                };

                if !valid {
                    return Outcome::Error((Status::Unauthorized, AuthError::InvalidToken));
                }

                Outcome::Success(AuthenticatedUser {
                    id: user_id,
                    username: claims.username,
                    role: claims.role,
                })
            }
            Err(AuthError::ExpiredToken) => {
                Outcome::Error((Status::Unauthorized, AuthError::ExpiredToken))
            }
            Err(_) => Outcome::Error((Status::Unauthorized, AuthError::InvalidToken)),
        }
    }
}

/// Decode and validate a JWT token.
pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|err| match err.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
        _ => AuthError::InvalidToken,
    })?;

    Ok(token_data.claims)
}

// Permission check at runtime
//
// For per-route permission checks, use `AuthenticatedUser` as a guard and
// call `check_permission()` inside the handler.

/// Check if a user has a specific permission by querying the database.
/// Admin users bypass the check (always have permission).
pub async fn check_permission(
    pool: &Pool,
    user_id: Uuid,
    permission: &str,
) -> Result<bool, String> {
    let client = pool
        .get()
        .await
        .map_err(|e| format!("DB pool error: {e}"))?;

    // First check if user is admin — they have all permissions
    let role_row = client
        .query_one(
            "SELECT r.name FROM users u JOIN roles r ON u.role_id = r.id WHERE u.id = $1",
            &[&user_id],
        )
        .await
        .map_err(|e| format!("DB query error: {e}"))?;

    let role_name: String = role_row.get("name");
    if role_name == "admin" {
        return Ok(true);
    }

    let row = client
        .query_one(
            "SELECT COUNT(*)::int8 AS cnt
             FROM users u
             JOIN role_permissions rp ON u.role_id = rp.role_id
             JOIN permissions p ON rp.permission_id = p.id
             WHERE u.id = $1 AND p.name = $2",
            &[&user_id, &permission],
        )
        .await
        .map_err(|e| format!("DB query error: {e}"))?;

    let count: i64 = row.get("cnt");
    Ok(count > 0)
}
