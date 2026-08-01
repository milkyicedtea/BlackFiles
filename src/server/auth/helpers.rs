use deadpool_postgres::GenericClient;
use rocket::http::{Cookie, SameSite};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::models::{RoleWithPermissions, User};
use crate::shared::{ApiError, bad_request, forbidden};

use super::guards::check_permission;

pub(crate) async fn require_permission(
    pool: &deadpool_postgres::Pool,
    user_id: Uuid,
    permission: &str,
) -> Result<(), ApiError> {
    if has_permission(pool, user_id, permission).await {
        Ok(())
    } else {
        Err(forbidden())
    }
}

pub(crate) async fn has_permission(
    pool: &deadpool_postgres::Pool,
    user_id: Uuid,
    permission: &str,
) -> bool {
    check_permission(pool, user_id, permission)
        .await
        .unwrap_or(false)
}

pub(crate) fn parse_user_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| bad_request("Invalid user ID"))
}

pub(crate) async fn find_user_by_id(
    client: &impl GenericClient,
    user_id: Uuid,
) -> Result<Option<Row>, tokio_postgres::Error> {
    client
        .query_opt(
            "SELECT u.id, u.username, u.password_hash, u.role_id, r.name as role_name,
                    u.created_at, u.updated_at
             FROM users u
             JOIN roles r ON u.role_id = r.id
             WHERE u.id = $1",
            &[&user_id],
        )
        .await
}

pub(crate) fn row_to_user(row: &Row) -> User {
    User {
        id: row.get("id"),
        username: row.get("username"),
        password_hash: String::new(),
        role_id: row.get("role_id"),
        role_name: row.get("role_name"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub(crate) fn row_to_role(row: &Row, permissions: Vec<String>) -> RoleWithPermissions {
    RoleWithPermissions {
        id: row.get("id"),
        name: row.get("name"),
        display_name: row.get("display_name"),
        position: row.get("position"),
        color: row.get("color"),
        permissions,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub(crate) async fn role_permissions(
    client: &impl GenericClient,
    role_id: i32,
) -> Result<Vec<String>, tokio_postgres::Error> {
    let rows = client
        .query(
            "SELECT p.name FROM permissions p
             JOIN role_permissions rp ON p.id = rp.permission_id
             WHERE rp.role_id = $1 ORDER BY p.group_name, p.name",
            &[&role_id],
        )
        .await?;
    Ok(rows.iter().map(|row| row.get("name")).collect())
}

pub(crate) async fn assign_role_permissions(
    client: &impl GenericClient,
    role_id: i32,
    permissions: &[String],
) -> Result<(), tokio_postgres::Error> {
    for permission in permissions {
        client
            .execute(
                "INSERT INTO role_permissions (role_id, permission_id)
                 SELECT $1, id FROM permissions WHERE name = $2
                 ON CONFLICT DO NOTHING",
                &[&role_id, permission],
            )
            .await?;
    }
    Ok(())
}

pub(crate) fn make_access_cookie(token: String, expiration_hours: i64) -> Cookie<'static> {
    make_auth_cookie("accessToken", token, "/", expiration_hours)
}

pub(crate) fn make_refresh_cookie(token: String, expiration_hours: i64) -> Cookie<'static> {
    make_auth_cookie("refreshToken", token, "/api/auth", expiration_hours * 24)
}

fn make_auth_cookie(
    name: &'static str,
    token: String,
    path: &'static str,
    expiration_hours: i64,
) -> Cookie<'static> {
    Cookie::build((name, token))
        .path(path)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(rocket::time::Duration::hours(expiration_hours))
        .into()
}
