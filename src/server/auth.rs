use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use deadpool_postgres::{GenericClient, Pool};
use jsonwebtoken::{EncodingKey, Header, encode};
use rand::random;
use rocket::State;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::serde::json::Json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::guards::{AuthenticatedUser, check_permission};
use crate::models::{
    AuthError, Claims, CreateRoleRequest, CreateUserRequest, LoginRequest, LoginResponse,
    LogoutResponse, MoveDirection, MoveRoleRequest, PaginationParams, RoleWithPermissions,
    UpdateRoleRequest, UpdateUserPasswordRequest, UpdateUserRoleRequest, User,
};
use crate::shared::{
    assign_role_permissions, bad_request, conflict, db_error, forbidden, get_client,
    make_access_cookie, make_refresh_cookie, not_found, row_to_role_with_permissions, row_to_user,
    server_error, unauthorized,
};

const ROLE_POSITION_LOCK: i64 = 1_976_101;

async fn lock_role_positions(
    transaction: &deadpool_postgres::Transaction<'_>,
) -> Result<(), tokio_postgres::Error> {
    transaction
        .execute("SELECT pg_advisory_xact_lock($1)", &[&ROLE_POSITION_LOCK])
        .await?;
    Ok(())
}

// Password hashing

fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::HashingError)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::HashingError)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// JWT generation

fn generate_jwt(
    user_id: &Uuid,
    username: &str,
    role: &str,
    secret: &str,
    expiration_hours: i64,
) -> Result<String, AuthError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiration_hours))
        .ok_or(AuthError::TokenGenerationError)?
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::TokenGenerationError)
}

// Refresh token helpers

fn generate_refresh_token() -> String {
    let bytes: [u8; 32] = random();
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// Handlers

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

/// POST /api/users — Create a new user (admin only)
#[post("/users", data = "<create>")]
pub async fn create_user(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    create: Json<CreateUserRequest>,
) -> Result<Json<LoginResponse>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "create_user")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if create.username.is_empty() || create.password.is_empty() {
        return Err(bad_request("Username and password are required"));
    }

    let client = get_client(pool).await?;
    let role_name = create
        .role_name
        .clone()
        .unwrap_or_else(|| "viewer".to_string());

    let role_row = client
        .query_opt("SELECT id, name FROM roles WHERE name = $1", &[&role_name])
        .await
        .map_err(db_error)?
        .ok_or_else(|| bad_request(&format!("Role '{}' not found", role_name)))?;

    let role_id: i32 = role_row.get("id");

    let existing = client
        .query_opt(
            "SELECT id FROM users WHERE username = $1",
            &[&create.username],
        )
        .await
        .map_err(db_error)?;

    if existing.is_some() {
        return Err(conflict("Username already exists"));
    }

    let password_hash = match hash_password(&create.password) {
        Ok(h) => h,
        Err(_) => return Err(server_error()),
    };

    let user_id = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO users (id, username, password_hash, role_id) VALUES ($1, $2, $3, $4)",
            &[&user_id, &create.username, &password_hash, &role_id],
        )
        .await
        .map_err(|e| {
            eprintln!("Failed to create user: {e}");
            server_error()
        })?;

    let new_user = User {
        id: user_id,
        username: create.username.clone(),
        password_hash: String::new(),
        role_id,
        role_name: role_name.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(LoginResponse { user: new_user }))
}

/// GET /api/users — List all users (admin only)
#[get("/users?<pagination..>")]
pub async fn list_users(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    pagination: PaginationParams,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "view_users")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    let client = get_client(pool).await?;
    let limit = pagination.effective_limit();
    let offset = pagination.effective_offset();
    let search_pattern = pagination.search.as_ref().map(|s| format!("%{}%", s));
    let username_pattern = pagination.username.as_ref().map(|s| format!("%{}%", s));

    let mut conditions: Vec<String> = Vec::new();
    let mut param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();

    if let Some(ref sp) = search_pattern {
        conditions.push(format!("u.username ILIKE ${}", param_refs.len() + 1));
        param_refs.push(sp);
    }
    if let Some(ref up) = username_pattern {
        conditions.push(format!("u.username ILIKE ${}", param_refs.len() + 1));
        param_refs.push(up);
    }
    if let Some(ref rn) = pagination.role_name {
        conditions.push(format!("r.name = ${}", param_refs.len() + 1));
        param_refs.push(rn);
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM users u JOIN roles r ON u.role_id = r.id {}",
        where_clause
    );
    let total: i64 = client
        .query_one(&count_sql, &param_refs)
        .await
        .map_err(db_error)?
        .get::<_, i64>(0);

    let data_sql = format!(
        "SELECT u.id, u.username, u.password_hash, u.role_id, r.name as role_name,
                u.created_at, u.updated_at
         FROM users u
         JOIN roles r ON u.role_id = r.id
         {}
         ORDER BY u.created_at ASC
         LIMIT ${} OFFSET ${}",
        where_clause,
        param_refs.len() + 1,
        param_refs.len() + 2,
    );

    let mut all_params = param_refs;
    all_params.push(&limit);
    all_params.push(&offset);

    let rows = client
        .query(&data_sql, &all_params)
        .await
        .map_err(db_error)?;

    let users: Vec<User> = rows.iter().map(row_to_user).collect();

    Ok(Json(serde_json::json!({"data": users, "total": total})))
}

/// GET /api/roles — List all roles with their permissions
#[get("/roles?<pagination..>")]
pub async fn list_roles(
    pool: &State<Pool>,
    _user: AuthenticatedUser,
    pagination: PaginationParams,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;
    let limit = pagination.effective_limit();
    let offset = pagination.effective_offset();
    let search_pattern = pagination.search.as_ref().map(|s| format!("%{}%", s));
    let name_pattern = pagination.name.as_ref().map(|s| format!("%{}%", s));
    let display_name_pattern = pagination.display_name.as_ref().map(|s| format!("%{}%", s));

    let mut conditions: Vec<String> = Vec::new();
    let mut param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();

    if let Some(ref sp) = search_pattern {
        conditions.push(format!(
            "(r.name ILIKE ${0} OR r.display_name ILIKE ${0})",
            param_refs.len() + 1
        ));
        param_refs.push(sp);
    }
    if let Some(ref np) = name_pattern {
        conditions.push(format!("r.name ILIKE ${}", param_refs.len() + 1));
        param_refs.push(np);
    }
    if let Some(ref dp) = display_name_pattern {
        conditions.push(format!("r.display_name ILIKE ${}", param_refs.len() + 1));
        param_refs.push(dp);
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM roles r {}", where_clause);
    let total: i64 = client
        .query_one(&count_sql, &param_refs)
        .await
        .map_err(db_error)?
        .get::<_, i64>(0);

    let data_sql = format!(
        "SELECT id, name, display_name, position, color, created_at, updated_at
         FROM roles r
         {}
         ORDER BY position ASC
         LIMIT ${} OFFSET ${}",
        where_clause,
        param_refs.len() + 1,
        param_refs.len() + 2,
    );

    let mut all_params = param_refs;
    all_params.push(&limit);
    all_params.push(&offset);

    let rows = client
        .query(&data_sql, &all_params)
        .await
        .map_err(db_error)?;

    let mut roles: Vec<RoleWithPermissions> = Vec::new();
    for row in &rows {
        let role_id: i32 = row.get("id");

        let perm_rows = client
            .query(
                "SELECT p.name FROM permissions p
                 JOIN role_permissions rp ON p.id = rp.permission_id
                 WHERE rp.role_id = $1 ORDER BY p.group_name, p.name",
                &[&role_id],
            )
            .await
            .map_err(db_error)?;

        let permissions: Vec<String> = perm_rows.iter().map(|r| r.get("name")).collect();

        roles.push(row_to_role_with_permissions(row, permissions));
    }

    Ok(Json(serde_json::json!({"data": roles, "total": total})))
}

/// GET /api/roles/<id> — Get a single role with permissions
#[get("/roles/<id>")]
pub async fn get_role(
    pool: &State<Pool>,
    _user: AuthenticatedUser,
    id: i32,
) -> Result<Json<RoleWithPermissions>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;

    let row = client
        .query_opt(
            "SELECT id, name, display_name, position, color, created_at, updated_at
             FROM roles WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Role not found"))?;

    let role_id: i32 = row.get("id");

    let perm_rows = client
        .query(
            "SELECT p.name FROM permissions p
             JOIN role_permissions rp ON p.id = rp.permission_id
             WHERE rp.role_id = $1 ORDER BY p.group_name, p.name",
            &[&role_id],
        )
        .await
        .map_err(db_error)?;

    let permissions: Vec<String> = perm_rows.iter().map(|r| r.get("name")).collect();

    Ok(Json(row_to_role_with_permissions(&row, permissions)))
}

/// POST /api/roles — Create a new role at the final position
#[post("/roles", data = "<create>")]
pub async fn create_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    create: Json<CreateRoleRequest>,
) -> Result<Json<RoleWithPermissions>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "manage_roles")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if create.name.is_empty() || create.display_name.is_empty() {
        return Err(bad_request("Name and display_name are required"));
    }

    let color = create.color.as_deref().unwrap_or("gray");
    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    lock_role_positions(&transaction).await.map_err(db_error)?;

    if transaction
        .query_opt("SELECT id FROM roles WHERE name = $1", &[&create.name])
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Err(conflict("Role already exists"));
    }

    let role = transaction
        .query_one(
            "INSERT INTO roles (name, display_name, position, color)
             SELECT $1, $2, COALESCE(MAX(position), 0) + 1, $3 FROM roles
             RETURNING id, name, display_name, position, color, created_at, updated_at",
            &[&create.name, &create.display_name, &color],
        )
        .await
        .map_err(db_error)?;

    let role_id: i32 = role.get("id");
    assign_role_permissions(&transaction, role_id, &create.permissions)
        .await
        .map_err(db_error)?;

    let response = RoleWithPermissions {
        id: role_id,
        name: role.get("name"),
        display_name: role.get("display_name"),
        position: role.get("position"),
        color: role.get("color"),
        permissions: create.permissions.clone(),
        created_at: role.get("created_at"),
        updated_at: role.get("updated_at"),
    };

    transaction.commit().await.map_err(db_error)?;
    Ok(Json(response))
}

/// PUT /api/roles/<id> — Update a role without changing its position
#[put("/roles/<id>", data = "<update>")]
pub async fn update_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: i32,
    update: Json<UpdateRoleRequest>,
) -> Result<Json<RoleWithPermissions>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "manage_roles")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if update.display_name.is_empty() {
        return Err(bad_request("display_name is required"));
    }

    let color = update.color.as_deref().unwrap_or("gray");
    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;

    let row = transaction
        .query_opt(
            "UPDATE roles SET display_name = $1, color = $2
             WHERE id = $3
             RETURNING id, name, display_name, position, color, created_at, updated_at",
            &[&update.display_name, &color, &id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Role not found"))?;

    transaction
        .execute("DELETE FROM role_permissions WHERE role_id = $1", &[&id])
        .await
        .map_err(db_error)?;
    assign_role_permissions(&transaction, id, &update.permissions)
        .await
        .map_err(db_error)?;

    let response = RoleWithPermissions {
        id: row.get("id"),
        name: row.get("name"),
        display_name: row.get("display_name"),
        position: row.get("position"),
        color: row.get("color"),
        permissions: update.permissions.clone(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };

    transaction.commit().await.map_err(db_error)?;
    Ok(Json(response))
}

/// POST /api/roles/<id>/move — Exchange a role with its adjacent position
#[post("/roles/<id>/move", data = "<move_request>")]
pub async fn move_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: i32,
    move_request: Json<MoveRoleRequest>,
) -> Result<Json<RoleWithPermissions>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "manage_roles")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    lock_role_positions(&transaction).await.map_err(db_error)?;

    let role = transaction
        .query_opt(
            "SELECT id, name, position FROM roles WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Role not found"))?;
    let name: String = role.get("name");
    if name == "admin" {
        return Err(bad_request("Cannot move the admin role"));
    }

    let position: i32 = role.get("position");
    let neighbor_position = match &move_request.direction {
        MoveDirection::Up if position == 1 => return Err(conflict("Role is already first")),
        MoveDirection::Up => position - 1,
        MoveDirection::Down => position + 1,
    };
    let neighbor = transaction
        .query_opt(
            "SELECT id, position FROM roles WHERE position = $1 FOR UPDATE",
            &[&neighbor_position],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| match &move_request.direction {
            MoveDirection::Up => conflict("Role is already first"),
            MoveDirection::Down => conflict("Role is already last"),
        })?;
    let neighbor_id: i32 = neighbor.get("id");

    transaction
        .batch_execute("SET CONSTRAINTS roles_position_key DEFERRED")
        .await
        .map_err(db_error)?;
    transaction
        .execute(
            "UPDATE roles
             SET position = CASE
                 WHEN id = $1 THEN $2::INTEGER
                 WHEN id = $3 THEN $4::INTEGER
             END
             WHERE id = $1 OR id = $3",
            &[&id, &neighbor_position, &neighbor_id, &position],
        )
        .await
        .map_err(db_error)?;

    let row = transaction
        .query_one(
            "SELECT id, name, display_name, position, color, created_at, updated_at
             FROM roles WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(db_error)?;
    let permission_rows = transaction
        .query(
            "SELECT p.name FROM permissions p
             JOIN role_permissions rp ON p.id = rp.permission_id
             WHERE rp.role_id = $1 ORDER BY p.group_name, p.name",
            &[&id],
        )
        .await
        .map_err(db_error)?;
    let permissions = permission_rows.iter().map(|row| row.get("name")).collect();
    let response = row_to_role_with_permissions(&row, permissions);

    transaction.commit().await.map_err(db_error)?;
    Ok(Json(response))
}

/// DELETE /api/roles/<id> — Delete a role and close the position gap
#[delete("/roles/<id>")]
pub async fn delete_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: i32,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "manage_roles")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    lock_role_positions(&transaction).await.map_err(db_error)?;

    let role = transaction
        .query_opt(
            "SELECT name, position FROM roles WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Role not found"))?;
    let name: String = role.get("name");
    if name == "admin" {
        return Err(bad_request("Cannot delete the admin role"));
    }
    if name == "viewer" {
        return Err(bad_request("Cannot delete the viewer role"));
    }
    let position: i32 = role.get("position");

    let viewer = transaction
        .query_opt("SELECT id FROM roles WHERE name = 'viewer' FOR UPDATE", &[])
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Viewer role not found"))?;
    let viewer_id: i32 = viewer.get("id");

    transaction
        .execute(
            "UPDATE users SET role_id = $1 WHERE role_id = $2",
            &[&viewer_id, &id],
        )
        .await
        .map_err(db_error)?;
    transaction
        .execute("DELETE FROM roles WHERE id = $1", &[&id])
        .await
        .map_err(db_error)?;
    transaction
        .batch_execute("SET CONSTRAINTS roles_position_key DEFERRED")
        .await
        .map_err(db_error)?;
    transaction
        .execute(
            "UPDATE roles SET position = position - 1 WHERE position > $1",
            &[&position],
        )
        .await
        .map_err(db_error)?;

    transaction.commit().await.map_err(db_error)?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// PUT /api/users/<id>/role — Update user role
#[put("/users/<id>/role", data = "<update>")]
pub async fn update_user_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: String,
    update: Json<UpdateUserRoleRequest>,
) -> Result<Json<User>, (Status, Json<serde_json::Value>)> {
    let user_id = Uuid::parse_str(&id).map_err(|_| bad_request("Invalid user ID"))?;

    if !check_permission(pool, user.id, "edit_user")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if user_id == user.id {
        return Err(bad_request("Cannot change your own role"));
    }

    let client = get_client(pool).await?;

    let role_row = client
        .query_opt("SELECT id FROM roles WHERE name = $1", &[&update.role])
        .await
        .map_err(db_error)?
        .ok_or_else(|| bad_request(&format!("Role '{}' not found", update.role)))?;

    let role_id: i32 = role_row.get("id");

    let updated = client
        .execute(
            "UPDATE users SET role_id = $1 WHERE id = $2",
            &[&role_id, &user_id],
        )
        .await
        .map_err(db_error)?;

    if updated == 0 {
        return Err(not_found("User not found"));
    }

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
        .ok_or_else(|| not_found("User not found after update"))?;

    Ok(Json(row_to_user(&row)))
}

/// PUT /api/users/<id>/password — Update user password
#[put("/users/<id>/password", data = "<update>")]
pub async fn update_user_password(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: String,
    update: Json<UpdateUserPasswordRequest>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let user_id = Uuid::parse_str(&id).map_err(|_| bad_request("Invalid user ID"))?;

    if !check_permission(pool, user.id, "edit_user")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if update.password.len() < 4 {
        return Err(bad_request("Password must be at least 4 characters"));
    }

    let client = get_client(pool).await?;

    let password_hash = match hash_password(&update.password) {
        Ok(h) => h,
        Err(_) => return Err(server_error()),
    };

    let updated = client
        .execute(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            &[&password_hash, &user_id],
        )
        .await
        .map_err(db_error)?;

    if updated == 0 {
        return Err(not_found("User not found"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// DELETE /api/users/<id> — Delete a user
#[delete("/users/<id>")]
pub async fn delete_user(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: String,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let user_id = Uuid::parse_str(&id).map_err(|_| bad_request("Invalid user ID"))?;

    if !check_permission(pool, user.id, "delete_user")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    if user_id == user.id {
        return Err(bad_request("Cannot delete yourself"));
    }

    let client = get_client(pool).await?;

    if let Ok(row) = client
        .query_one("SELECT username FROM users WHERE id = $1", &[&user_id])
        .await
    {
        let username: String = row.get("username");
        if username == "admin" {
            return Err(bad_request("Cannot delete the admin user"));
        }
    }

    let deleted = client
        .execute("DELETE FROM users WHERE id = $1", &[&user_id])
        .await
        .map_err(db_error)?;

    if deleted == 0 {
        return Err(not_found("User not found"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

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

/// GET /api/permissions — List all permissions
#[get("/permissions")]
pub async fn list_permissions(
    pool: &State<Pool>,
    _user: AuthenticatedUser,
) -> Result<Json<Vec<crate::models::Permission>>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;

    let rows = client
        .query(
            "SELECT id, name, display_name, group_name FROM permissions ORDER BY group_name, name",
            &[],
        )
        .await
        .map_err(db_error)?;

    let perms: Vec<crate::models::Permission> = rows
        .iter()
        .map(|row| crate::models::Permission {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            group_name: row.get("group_name"),
        })
        .collect();

    Ok(Json(perms))
}

// Admin bootstrap

/// Create the default admin user if no users exist.
pub async fn create_default_admin(pool: &Pool) {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to get DB connection for admin bootstrap: {e}");
            return;
        }
    };

    let count: i64 = match client
        .query_one("SELECT COUNT(*)::int8 FROM users", &[])
        .await
    {
        Ok(row) => row.get(0),
        Err(e) => {
            eprintln!("Failed to check user count: {e}. Tables may not exist yet.");
            return;
        }
    };

    if count > 0 {
        println!("Users already exist -- skipping admin bootstrap.");
        return;
    }

    let default_password =
        std::env::var("DEFAULT_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    let password_hash = match hash_password(&default_password) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to hash admin password: {e:?}");
            return;
        }
    };

    let role_id: Option<i32> = match client
        .query_opt("SELECT id FROM roles WHERE name = 'admin'", &[])
        .await
    {
        Ok(Some(row)) => Some(row.get("id")),
        Ok(None) => {
            eprintln!("Admin role not found in database. Run dbinit/0001_seed.sql first.");
            return;
        }
        Err(e) => {
            eprintln!("Failed to query admin role: {e}");
            return;
        }
    };

    let role_id = match role_id {
        Some(id) => id,
        None => return,
    };

    let admin_id = Uuid::new_v4();

    match client
        .execute(
            "INSERT INTO users (id, username, password_hash, role_id) VALUES ($1, $2, $3, $4)",
            &[&admin_id, &"admin", &password_hash, &role_id],
        )
        .await
    {
        Ok(_) => {
            println!("Default admin user created (username: admin, password: {default_password})")
        }
        Err(e) => eprintln!("Failed to create default admin: {e}"),
    }
}
