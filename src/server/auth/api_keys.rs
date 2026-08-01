use deadpool_postgres::Pool;
use rocket::State;
use rocket::http::Status;
use rocket::serde::{Deserialize, Serialize, json::Json};
use uuid::Uuid;

use super::guards::AuthenticatedUser;
use crate::shared::{db_error, forbidden, get_client, not_found, random_hex, sha256_hex};

// ── Response types ──

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub label: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiKeyCreatedResponse {
    pub id: Uuid,
    pub label: Option<String>,
    pub key: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ApiKeyListResponse {
    pub keys: Vec<ApiKeyResponse>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AdminApiKeyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub label: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AdminApiKeyListResponse {
    pub keys: Vec<AdminApiKeyResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateApiKeyRequest {
    pub label: Option<String>,
}

fn key_timestamps(row: &tokio_postgres::Row) -> (Option<String>, String) {
    let last_used_at = row
        .try_get::<_, chrono::DateTime<chrono::Utc>>("last_used_at")
        .ok()
        .map(|date| date.to_rfc3339());
    let created_at = row
        .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
        .to_rfc3339();
    (last_used_at, created_at)
}

fn row_to_key(row: &tokio_postgres::Row) -> ApiKeyResponse {
    let (last_used_at, created_at) = key_timestamps(row);
    ApiKeyResponse {
        id: row.get("id"),
        label: row.get("label"),
        last_used_at,
        created_at,
    }
}

fn row_to_admin_key(row: &tokio_postgres::Row) -> AdminApiKeyResponse {
    let (last_used_at, created_at) = key_timestamps(row);
    AdminApiKeyResponse {
        id: row.get("id"),
        user_id: row.get("user_id"),
        username: row.get("username"),
        label: row.get("label"),
        last_used_at,
        created_at,
    }
}

// ── User endpoints ──

#[get("/music/api-keys")]
pub(crate) async fn list_my_api_keys(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<ApiKeyListResponse>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;
    let rows = client.query(
        "SELECT id, label, last_used_at, created_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        &[&user.id],
    ).await.map_err(db_error)?;
    let keys: Vec<ApiKeyResponse> = rows.iter().map(row_to_key).collect();
    Ok(Json(ApiKeyListResponse { keys }))
}

#[post("/music/api-keys", data = "<req>")]
pub(crate) async fn create_api_key(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    req: Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyCreatedResponse>, (Status, Json<serde_json::Value>)> {
    let client = get_client(pool).await?;
    let (key, key_hash) = loop {
        let key = random_hex::<24>();
        let hash = sha256_hex(&key);
        if client
            .query_opt("SELECT 1 FROM api_keys WHERE key_hash = $1", &[&hash])
            .await
            .map_err(db_error)?
            .is_none()
        {
            break (key, hash);
        }
    };
    let id = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO api_keys (id, user_id, key_hash, label) VALUES ($1,$2,$3,$4)",
            &[&id, &user.id, &key_hash, &req.label],
        )
        .await
        .map_err(db_error)?;
    let row = client
        .query_one("SELECT created_at FROM api_keys WHERE id = $1", &[&id])
        .await
        .map_err(db_error)?;
    Ok(Json(ApiKeyCreatedResponse {
        id,
        label: req.label.clone(),
        key,
        created_at: row
            .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
            .to_rfc3339(),
    }))
}

#[delete("/music/api-keys/<id>")]
pub(crate) async fn revoke_api_key(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let key_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid key ID"))?;
    let client = get_client(pool).await?;
    if client
        .execute(
            "DELETE FROM api_keys WHERE id = $1 AND user_id = $2",
            &[&key_id, &user.id],
        )
        .await
        .map_err(db_error)?
        == 0
    {
        return Err(not_found("API key not found"));
    }
    Ok(Json(serde_json::json!({"message": "API key revoked"})))
}

// ── Admin endpoints ──

#[get("/admin/api-keys")]
pub(crate) async fn list_all_api_keys(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<AdminApiKeyListResponse>, (Status, Json<serde_json::Value>)> {
    if user.role != "admin" {
        return Err(forbidden());
    }
    let client = get_client(pool).await?;
    let rows = client.query(
        "SELECT ak.id, ak.user_id, u.username, ak.label, ak.last_used_at, ak.created_at FROM api_keys ak JOIN users u ON ak.user_id = u.id ORDER BY ak.created_at DESC",
        &[],
    ).await.map_err(db_error)?;
    let keys: Vec<AdminApiKeyResponse> = rows.iter().map(row_to_admin_key).collect();
    Ok(Json(AdminApiKeyListResponse { keys }))
}

#[delete("/admin/api-keys/<id>")]
pub(crate) async fn admin_revoke_api_key(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    if user.role != "admin" {
        return Err(forbidden());
    }
    let key_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid key ID"))?;
    let client = get_client(pool).await?;
    if client
        .execute("DELETE FROM api_keys WHERE id = $1", &[&key_id])
        .await
        .map_err(db_error)?
        == 0
    {
        return Err(not_found("API key not found"));
    }
    Ok(Json(serde_json::json!({"message": "API key revoked"})))
}
