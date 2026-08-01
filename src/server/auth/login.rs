use super::*;

use crate::models::{AuthError, Claims, LoginRequest, LoginResponse, LogoutResponse};
use crate::shared::{make_access_cookie, make_refresh_cookie};
use rocket::http::{Cookie, CookieJar, Status};
use uuid::Uuid;

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

    let user_id: Uuid = row.get("id");
    let username: String = row.get("username");
    let role_name: String = row.get("role_name");
    let created_at = row.get("created_at");
    let updated_at = row.get("updated_at");

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    let exp_hours: i64 = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);

    let access_token = match generate_jwt(&user_id, &username, &role_name, &jwt_secret, exp_hours) {
        Ok(t) => t,
        Err(_) => return Err(server_error()),
    };

    let refresh_token = generate_refresh_token();
    let token_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::hours(exp_hours * 24);

    if let Err(e) = client
        .execute(
            "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            &[&user_id, &token_hash, &expires_at],
        )
        .await
    {
        eprintln!("Failed to store refresh token: {e}");
        return Err(server_error());
    }

    jar.add(make_access_cookie(access_token, exp_hours));
    jar.add(make_refresh_cookie(refresh_token, exp_hours));

    let user = User {
        id: user_id,
        username,
        password_hash: String::new(),
        role_id: row.get("role_id"),
        role_name,
        created_at,
        updated_at,
    };

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
        let token_hash = hash_token(refresh_token.value());

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

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.password_hash, u.role_id, r.name as role_name,
                    u.created_at, u.updated_at
             FROM users u
             JOIN roles r ON u.role_id = r.id
             WHERE u.id = $1",
            &[&user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| unauthorized("User not found"))?;

    let user_obj = row_to_user(&row);

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
    let token_hash = hash_token(&refresh_token);

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

    let row = client
        .query_opt(
            "SELECT u.id, u.username, u.password_hash, u.role_id, r.name as role_name,
                    u.created_at, u.updated_at
             FROM users u
             JOIN roles r ON u.role_id = r.id
             WHERE u.id = $1",
            &[&user_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| unauthorized("User not found"))?;

    let username: String = row.get("username");
    let role_name: String = row.get("role_name");
    let created_at = row.get("created_at");
    let updated_at = row.get("updated_at");

    if let Err(e) = client
        .execute(
            "UPDATE sessions SET revoked = TRUE WHERE token_hash = $1",
            &[&token_hash],
        )
        .await
    {
        eprintln!("Failed to revoke session: {e}");
    }

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    let exp_hours: i64 = std::env::var("JWT_EXPIRATION_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .unwrap_or(24);

    let access_token = match generate_jwt(&user_id, &username, &role_name, &jwt_secret, exp_hours) {
        Ok(t) => t,
        Err(_) => return Err(server_error()),
    };

    let new_refresh_token = generate_refresh_token();
    let new_token_hash = hash_token(&new_refresh_token);
    let expires_at_new = Utc::now() + Duration::hours(exp_hours * 24);

    if let Err(e) = client
        .execute(
            "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            &[&user_id, &new_token_hash, &expires_at_new],
        )
        .await
    {
        eprintln!("Failed to store new refresh token: {e}");
    }

    jar.add(make_access_cookie(access_token, exp_hours));
    jar.add(make_refresh_cookie(new_refresh_token, exp_hours));

    let user_obj = User {
        id: user_id,
        username,
        password_hash: String::new(),
        role_id: row.get("role_id"),
        role_name,
        created_at,
        updated_at,
    };

    Ok(Json(LoginResponse { user: user_obj }))
}

// User management

/// GET /api/check — Check if current user is authenticated
#[get("/check")]
pub async fn check_auth(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;

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
            "id": user.id,
            "username": user.username,
            "role_name": user.role,
            "permissions": permissions,
        }
    })))
}
