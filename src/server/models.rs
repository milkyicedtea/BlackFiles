use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

// User

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub password_hash: String,
    pub role_id: i32,
    pub role_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginResponse {
    pub user: User,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LogoutResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateUserRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateUserPasswordRequest {
    pub password: String,
}

// JWT Claims

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

// Roles & Permissions

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[allow(dead_code)]
pub struct Role {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub hierarchy: i32,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct RoleWithPermissions {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub hierarchy: i32,
    pub color: String,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateRoleRequest {
    pub name: String,
    pub display_name: String,
    pub hierarchy: i32,
    pub color: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateRoleRequest {
    pub display_name: String,
    pub hierarchy: i32,
    pub color: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Permission {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub group_name: String,
}

// Pagination

#[derive(Debug, Deserialize, rocket::form::FromForm)]
#[serde(crate = "rocket::serde")]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub username: Option<String>,
    pub role_name: Option<String>,
    pub name: Option<String>,
    pub display_name: Option<String>,
}

impl PaginationParams {
    /// Returns the limit clamped to [1, 50]. Defaults to 25.
    pub fn effective_limit(&self) -> i64 {
        self.limit.map(|l| l.clamp(1, 50)).unwrap_or(25)
    }

    /// Returns the offset. Defaults to 0.
    pub fn effective_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
// Auth error

#[derive(Debug)]
pub enum AuthError {
    HashingError,
    TokenGenerationError,
    MissingToken,
    InvalidToken,
    ExpiredToken,
}
