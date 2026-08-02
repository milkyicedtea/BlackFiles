use super::*;

use crate::models::{LoginRequest, LoginResponse, LogoutResponse};
use rocket::http::{Cookie, CookieJar, Status};
use uuid::Uuid;

async fn issue_tokens(
    client: &deadpool_postgres::Object,
    jar: &CookieJar<'_>,
    user: &User,
    fail_on_session_error: bool,
) -> Result<(), ApiError> {
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    let expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);
    let access_token = generate_jwt(
        &user.id,
        &user.username,
        &user.role_name,
        &jwt_secret,
        expiration_hours,
    )
    .map_err(|_| server_error())?;
    let refresh_token = generate_refresh_token();
    let token_hash = sha256_hex(&refresh_token);
    let expires_at = Utc::now() + Duration::hours(expiration_hours * 24);

    if let Err(error) = client
        .execute(
            "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            &[&user.id, &token_hash, &expires_at],
        )
        .await
    {
        eprintln!("Failed to store refresh token: {error}");
        if fail_on_session_error {
            return Err(server_error());
        }
    }

    jar.add(make_access_cookie(access_token, expiration_hours));
    jar.add(make_refresh_cookie(refresh_token, expiration_hours));
    Ok(())
}

/// POST /api/auth/login
#[post("/auth/login", data = "<login>")]
pub async fn login(
    pool: &State<Pool>,
    jar: &CookieJar<'_>,
    login: Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (Status, Json<serde_json::Value>)> {
    if login.username.is_empty() || login.password.is_empty() {
        return Err(bad_request("Username and password are required"));
    }

    let client = get_client(pool).await?;

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.password_hash, u.role_id, r.name as role_name,
                    u.created_at, u.updated_at
             FROM users u
             JOIN roles r ON u.role_id = r.id
             WHERE u.username = $1",
            &[&login.username],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| unauthorized("Invalid credentials"))?;

    let password_hash: String = row.get("password_hash");

    match verify_password(&login.password, &password_hash) {
        Ok(true) => {}
        Ok(false) => return Err(unauthorized("Invalid credentials")),
        Err(_) => return Err(server_error()),
    }

    let user = row_to_user(&row);
    issue_tokens(&client, jar, &user, true).await?;

    Ok(Json(LoginResponse { user }))
}

/// POST /api/auth/logout
#[post("/auth/logout")]
pub async fn logout(
    pool: &State<Pool>,
    jar: &CookieJar<'_>,
    user: AuthenticatedUser,
) -> Json<LogoutResponse> {
    if let Some(refresh_token) = jar.get("refresh_token") {
        let token_hash = sha256_hex(refresh_token.value());

        match get_client(pool).await {
            Ok(client) => {
                if let Err(e) = client
                    .execute(
                        "DELETE FROM sessions WHERE user_id = $1 AND token_hash = $2",
                        &[&user.id, &token_hash],
                    )
                    .await
                {
                    eprintln!("Failed to delete session during logout: {e}");
                }
            }
            Err(_) => eprintln!("Failed to get DB connection during logout"),
        }
    }

    jar.remove(Cookie::build("accessToken").path("/"));
    jar.remove(Cookie::build("refreshToken").path("/api/auth"));

    Json(LogoutResponse {
        message: "Logged out".to_string(),
    })
}

/// GET /api/auth/me
#[get("/auth/me")]
pub async fn me(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<LoginResponse>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;

    let user_obj = find_user_by_id(&client, user.id)
        .await
        .map_err(db_error)?
        .map(|row| row_to_user(&row))
        .ok_or_else(|| unauthorized("User not found"))?;

    Ok(Json(LoginResponse { user: user_obj }))
}

/// POST /api/auth/refresh
#[post("/auth/refresh")]
pub async fn refresh(
    pool: &State<Pool>,
    jar: &CookieJar<'_>,
) -> Result<Json<LoginResponse>, (Status, Json<serde_json::Value>)> {
    let refresh_token = jar
        .get("refreshToken")
        .map(|c| c.value().to_string())
        .ok_or_else(|| unauthorized("Refresh token not found"))?;

    let client = get_client(pool).await?;
    let token_hash = sha256_hex(&refresh_token);

    let row = client
        .query_opt(
            "SELECT s.user_id, s.revoked, s.expires_at
             FROM sessions s
             WHERE s.token_hash = $1",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| unauthorized("Invalid refresh token"))?;

    let revoked: bool = row.get("revoked");
    let expires_at: chrono::DateTime<Utc> = row.get("expires_at");

    if revoked {
        return Err(unauthorized("Refresh token has been revoked"));
    }

    if Utc::now() > expires_at {
        return Err(unauthorized("Refresh token has expired"));
    }

    let user_id: Uuid = row.get("user_id");

    let user_obj = find_user_by_id(&client, user_id)
        .await
        .map_err(db_error)?
        .map(|row| row_to_user(&row))
        .ok_or_else(|| unauthorized("User not found"))?;

    if let Err(e) = client
        .execute(
            "UPDATE sessions SET revoked = TRUE WHERE token_hash = $1",
            &[&token_hash],
        )
        .await
    {
        eprintln!("Failed to revoke session: {e}");
    }

    issue_tokens(&client, jar, &user_obj, false).await?;

    Ok(Json(LoginResponse { user: user_obj }))
}

// User management

/// GET /api/check - Check if current user is authenticated
#[get("/check")]
pub async fn check_auth(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;
    let profile = find_user_by_id(&client, user.id)
        .await
        .map_err(db_error)?
        .map(|row| row_to_user(&row))
        .ok_or_else(|| not_found("User not found"))?;

    let rows = client
        .query(
            "SELECT p.name FROM role_permissions rp
             JOIN permissions p ON rp.permission_id = p.id
             JOIN users u ON u.role_id = rp.role_id
             WHERE u.id = $1",
            &[&user.id],
        )
        .await
        .map_err(db_error)?;

    let permissions: Vec<String> = rows.iter().map(|row| row.get("name")).collect();

    Ok(Json(serde_json::json!({
        "user": {
            "id": profile.id,
            "username": profile.username,
            "role_id": profile.role_id,
            "role_name": profile.role_name,
            "created_at": profile.created_at,
            "updated_at": profile.updated_at,
            "permissions": permissions,
        }
    })))
}
