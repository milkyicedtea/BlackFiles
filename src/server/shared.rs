use crate::models::{PaginationParams, RoleWithPermissions, User};
use deadpool_postgres::GenericClient;
use tokio_postgres::Row;

use rocket::http::{Cookie, Header, Status};
use rocket::response::Responder;
use rocket::serde::{Serialize, json::Json};
use rocket::{Request, Response};
use std::path::Path;
use std::path::PathBuf;
use tokio::io::AsyncRead;

pub const STORAGE_ROOT: &str = "storage";
pub const BUILD_ROOT: &str = "dist";

// Error response helpers
// These reduce repeated `(Status::X, Json(json!({"error": "..."})))` patterns.

pub fn server_error() -> (Status, Json<serde_json::Value>) {
    (
        Status::InternalServerError,
        Json(serde_json::json!({"error": "Server error"})),
    )
}
#[catch(default)]
pub fn api_error(status: Status, _: &Request<'_>) -> (Status, Json<serde_json::Value>) {
    let message = if status == Status::InternalServerError {
        "Server error".to_owned()
    } else {
        status.reason().unwrap_or("Request failed").to_owned()
    };

    (status, Json(serde_json::json!({"error": message})))
}

pub fn bad_request(msg: &str) -> (Status, Json<serde_json::Value>) {
    (Status::BadRequest, Json(serde_json::json!({"error": msg})))
}

pub fn unauthorized(msg: &str) -> (Status, Json<serde_json::Value>) {
    (
        Status::Unauthorized,
        Json(serde_json::json!({"error": msg})),
    )
}

pub fn forbidden() -> (Status, Json<serde_json::Value>) {
    (
        Status::Forbidden,
        Json(serde_json::json!({"error": "Insufficient permissions"})),
    )
}

pub fn not_found(msg: &str) -> (Status, Json<serde_json::Value>) {
    (Status::NotFound, Json(serde_json::json!({"error": msg})))
}

pub fn conflict(msg: &str) -> (Status, Json<serde_json::Value>) {
    (Status::Conflict, Json(serde_json::json!({"error": msg})))
}

/// Wraps a database error log + server error response.
pub fn db_error<E: std::fmt::Display>(e: E) -> (Status, Json<serde_json::Value>) {
    eprintln!("DB error: {e}");
    server_error()
}

/// Acquires a database connection from the pool, returning a Server error on failure.
pub async fn get_client(
    pool: &deadpool_postgres::Pool,
) -> Result<deadpool_postgres::Object, (Status, Json<serde_json::Value>)> {
    pool.get().await.map_err(|e| {
        eprintln!("DB pool error: {e}");
        server_error()
    })
}

/// Construct a User from a database row (standard user query columns).
pub fn row_to_user(row: &Row) -> User {
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

/// Construct a RoleWithPermissions from a database row and a pre-fetched permissions list.
pub fn row_to_role_with_permissions(row: &Row, permissions: Vec<String>) -> RoleWithPermissions {
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

/// Assign permissions to a role, ignoring duplicate request values.
pub async fn assign_role_permissions(
    client: &impl GenericClient,
    role_id: i32,
    permissions: &[String],
) -> Result<(), tokio_postgres::Error> {
    for perm_name in permissions {
        client
            .execute(
                "INSERT INTO role_permissions (role_id, permission_id)
                 SELECT $1, id FROM permissions WHERE name = $2
                 ON CONFLICT DO NOTHING",
                &[&role_id, &perm_name],
            )
            .await?;
    }

    Ok(())
}

/// Build an access-token cookie set for the whole site.
pub fn make_access_cookie(token: String, exp_hours: i64) -> Cookie<'static> {
    Cookie::build(("accessToken", token))
        .path("/")
        .http_only(true)
        .same_site(rocket::http::SameSite::Lax)
        .max_age(rocket::time::Duration::hours(exp_hours))
        .into()
}

/// Build a refresh-token cookie restricted to `/api/auth`.
pub fn make_refresh_cookie(token: String, exp_hours: i64) -> Cookie<'static> {
    Cookie::build(("refreshToken", token))
        .path("/api/auth")
        .http_only(true)
        .same_site(rocket::http::SameSite::Lax)
        .max_age(rocket::time::Duration::hours(exp_hours * 24))
        .into()
}
// Shared types

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
    pub size: u64,
    pub modified: u64, // unix time
}

pub struct FileResponse {
    pub stream: Box<dyn AsyncRead + Send + Unpin>,
    pub size: u64,
}

impl<'r> Responder<'r, 'static> for FileResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .header(Header::new("Content-Length", self.size.to_string()))
            .streamed_body(self.stream)
            .ok()
    }
}

pub fn sanitize_path(path: PathBuf) -> Option<PathBuf> {
    // allow empty paths for root directory
    if path.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }

    let mut clean = PathBuf::new();

    for part in path.components() {
        match part {
            std::path::Component::Normal(p) => {
                // additional check to reject null bytes or looks sus
                let s = p.to_string_lossy();
                if s.contains('\0') || s == "." || s == ".." {
                    return None;
                }
                clean.push(p);
            }
            _ => return None,
        }
    }

    // reject empty paths
    if clean.as_os_str().is_empty() {
        return None;
    }

    Some(clean)
}

// Convert a Path to a web-friendly path string using forward slashes.
// This ensures the API returns paths with '/' separators even on Windows.
pub fn path_to_web_string(path: &Path) -> String {
    path.iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn filter_by_search_term(pagination: &PaginationParams, entries: &mut Vec<FileEntry>) {
    if let Some(ref search) = pagination.search
        && !search.is_empty()
    {
        let lower = search.to_lowercase();
        entries.retain(|e| e.name.to_lowercase().contains(&lower));
    }
}

