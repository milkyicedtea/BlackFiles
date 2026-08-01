use deadpool_postgres::Pool;
use rand::Rng;
use rocket::State;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Deserialize, Serialize, json::Json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::guards::AuthenticatedUser;
use crate::shared::{db_error, forbidden, get_client, not_found};

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

// ── Error helper ──

fn api_err(status: Status, msg: &str) -> (Status, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({"error": msg})))
}

// ── Key crypto ──

fn generate_api_key() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill(&mut bytes);
    hex::encode(bytes)
}

pub(crate) fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

fn row_to_key(row: &tokio_postgres::Row) -> ApiKeyResponse {
    ApiKeyResponse {
        id: row.get("id"),
        label: row.get("label"),
        last_used_at: row
            .try_get::<_, chrono::DateTime<chrono::Utc>>("last_used_at")
            .ok()
            .map(|d| d.to_rfc3339()),
        created_at: row
            .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
            .to_rfc3339(),
    }
}

fn row_to_admin_key(row: &tokio_postgres::Row) -> AdminApiKeyResponse {
    AdminApiKeyResponse {
        id: row.get("id"),
        user_id: row.get("user_id"),
        username: row.get("username"),
        label: row.get("label"),
        last_used_at: row
            .try_get::<_, chrono::DateTime<chrono::Utc>>("last_used_at")
            .ok()
            .map(|d| d.to_rfc3339()),
        created_at: row
            .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
            .to_rfc3339(),
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
    let keys: Vec<ApiKeyResponse> = rows.iter().map(|r| row_to_key(r)).collect();
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
        let key = generate_api_key();
        let hash = hash_api_key(&key);
        if !client
            .query_opt("SELECT 1 FROM api_keys WHERE key_hash = $1", &[&hash])
            .await
            .map_err(db_error)?
            .is_some()
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
    let keys: Vec<AdminApiKeyResponse> = rows.iter().map(|r| row_to_admin_key(r)).collect();
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

// ── API Key Auth Guard ──

#[derive(Debug, Clone)]
pub struct ApiKeyUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKeyUser {
    type Error = Json<serde_json::Value>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let api_key = request.uri().query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (k, v) = pair.as_str().split_once('=')?;
                if k == "apiKey" {
                    Some(url_decode(v))
                } else {
                    None
                }
            })
        });

        let api_key = match api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => return Outcome::Error(api_err(Status::Unauthorized, "Missing apiKey parameter")),
        };

        let pool = match request.guard::<&State<Pool>>().await {
            Outcome::Success(p) => p,
            _ => {
                return Outcome::Error(api_err(
                    Status::InternalServerError,
                    "Server configuration error",
                ));
            }
        };

        let key_hash = hash_api_key(&api_key);

        let client = match pool.get().await {
            Ok(c) => c,
            Err(_) => {
                return Outcome::Error(api_err(Status::InternalServerError, "Database error"));
            }
        };

        let row = match client.query_opt(
            "SELECT u.id, u.username, r.name as role_name FROM api_keys ak JOIN users u ON ak.user_id = u.id JOIN roles r ON u.role_id = r.id WHERE ak.key_hash = $1",
            &[&key_hash],
        ).await {
            Ok(Some(r)) => r,
            Ok(None) => return Outcome::Error(api_err(Status::Unauthorized, "Invalid API key")),
            Err(_) => return Outcome::Error(api_err(Status::InternalServerError, "Database error")),
        };

        let id: Uuid = row.get("id");
        let username: String = row.get("username");
        let role: String = row.get("role_name");

        if let Ok(c) = pool.get().await {
            let _ = c
                .execute(
                    "UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1",
                    &[&key_hash],
                )
                .await;
        }

        Outcome::Success(ApiKeyUser { id, username, role })
    }
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    out.push(b as char);
                } else {
                    out.push('%');
                    out.push_str(&hex);
                }
            }
            '+' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}
