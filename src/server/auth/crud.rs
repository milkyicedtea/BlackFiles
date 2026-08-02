use super::*;

use crate::models::{
    CreateRoleRequest, CreateUserRequest, LoginResponse, MoveDirection, MoveRoleRequest,
    PaginationParams, RoleWithPermissions, UpdateRoleRequest, UpdateUserPasswordRequest,
    UpdateUserRoleRequest, User,
};
use rocket::http::Status;
use uuid::Uuid;

type SqlParam<'a> = &'a (dyn tokio_postgres::types::ToSql + Sync);

fn like_pattern(value: Option<&String>) -> Option<String> {
    value.map(|value| format!("%{value}%"))
}

fn add_filter<'a>(
    conditions: &mut Vec<String>,
    params: &mut Vec<SqlParam<'a>>,
    value: Option<&'a String>,
    condition: impl FnOnce(usize) -> String,
) {
    if let Some(value) = value {
        conditions.push(condition(params.len() + 1));
        params.push(value);
    }
}

fn where_clause(conditions: &[String]) -> String {
    if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    }
}

async fn query_page<'a>(
    client: &deadpool_postgres::Object,
    sql: &str,
    mut params: Vec<SqlParam<'a>>,
    limit: &'a i64,
    offset: &'a i64,
) -> Result<Vec<tokio_postgres::Row>, tokio_postgres::Error> {
    params.push(limit);
    params.push(offset);
    client.query(sql, &params).await
}

/// POST /api/users — Create a new user (admin only)
#[post("/users", data = "<create>")]
pub async fn create_user(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    create: Json<CreateUserRequest>,
) -> Result<Json<LoginResponse>, (Status, Json<serde_json::Value>)> {
    require_permission(pool, user.id, "create_user").await?;

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
    require_permission(pool, user.id, "view_users").await?;

    let client = get_client(pool).await?;
    let limit = pagination.effective_limit();
    let offset = pagination.effective_offset();
    let search_pattern = like_pattern(pagination.search.as_ref());
    let username_pattern = like_pattern(pagination.username.as_ref());
    let mut conditions = Vec::new();
    let mut param_refs: Vec<SqlParam<'_>> = Vec::new();
    add_filter(
        &mut conditions,
        &mut param_refs,
        search_pattern.as_ref(),
        |index| format!("u.username ILIKE ${index}"),
    );
    add_filter(
        &mut conditions,
        &mut param_refs,
        username_pattern.as_ref(),
        |index| format!("u.username ILIKE ${index}"),
    );
    add_filter(
        &mut conditions,
        &mut param_refs,
        pagination.role_name.as_ref(),
        |index| format!("r.name = ${index}"),
    );
    let where_clause = where_clause(&conditions);

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

    let rows = query_page(&client, &data_sql, param_refs, &limit, &offset)
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
    let search_pattern = like_pattern(pagination.search.as_ref());
    let name_pattern = like_pattern(pagination.name.as_ref());
    let display_name_pattern = like_pattern(pagination.display_name.as_ref());
    let mut conditions = Vec::new();
    let mut param_refs: Vec<SqlParam<'_>> = Vec::new();
    add_filter(
        &mut conditions,
        &mut param_refs,
        search_pattern.as_ref(),
        |index| format!("(r.name ILIKE ${index} OR r.display_name ILIKE ${index})"),
    );
    add_filter(
        &mut conditions,
        &mut param_refs,
        name_pattern.as_ref(),
        |index| format!("r.name ILIKE ${index}"),
    );
    add_filter(
        &mut conditions,
        &mut param_refs,
        display_name_pattern.as_ref(),
        |index| format!("r.display_name ILIKE ${index}"),
    );
    let where_clause = where_clause(&conditions);

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

    let rows = query_page(&client, &data_sql, param_refs, &limit, &offset)
        .await
        .map_err(db_error)?;

    let mut roles: Vec<RoleWithPermissions> = Vec::new();
    for row in &rows {
        let role_id = row.get("id");
        let permissions = role_permissions(&client, role_id).await.map_err(db_error)?;
        roles.push(row_to_role(row, permissions));
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

    let role_id = row.get("id");
    let permissions = role_permissions(&client, role_id).await.map_err(db_error)?;
    Ok(Json(row_to_role(&row, permissions)))
}

/// POST /api/roles — Create a new role at the final position
#[post("/roles", data = "<create>")]
pub async fn create_role(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    create: Json<CreateRoleRequest>,
) -> Result<Json<RoleWithPermissions>, (Status, Json<serde_json::Value>)> {
    require_permission(pool, user.id, "manage_roles").await?;

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

    let response = row_to_role(&role, create.permissions.clone());

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
    require_permission(pool, user.id, "manage_roles").await?;

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

    let response = row_to_role(&row, update.permissions.clone());

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
    require_permission(pool, user.id, "manage_roles").await?;

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
    let permissions = role_permissions(&transaction, id).await.map_err(db_error)?;
    let response = row_to_role(&row, permissions);

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
    require_permission(pool, user.id, "manage_roles").await?;

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
    let user_id = parse_user_id(&id)?;
    require_permission(pool, user.id, "edit_user").await?;

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

    let row = find_user_by_id(&client, user_id)
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
    let user_id = parse_user_id(&id)?;
    if user_id != user.id {
        require_permission(pool, user.id, "edit_user").await?;
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
    let user_id = parse_user_id(&id)?;
    require_permission(pool, user.id, "delete_user").await?;

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
