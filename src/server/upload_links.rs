use crate::guards::{AuthenticatedUser, check_permission};
use crate::models::{
    CreateUploadLinkRequest, CreatedUploadLink, PublicUploadLinkStatus, UploadLink,
};
use crate::shared::{
    bad_request, conflict, db_error, forbidden, get_client, not_found, path_to_web_string,
    sanitize_path, server_error,
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use rand::random;
use rocket::State;
use rocket::data::{Data, ToByteUnit};
use rocket::http::Status;
use rocket::serde::json::Json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

struct ActorRole {
    is_admin: bool,
    position: i32,
}

struct LinkOwner {
    user_id: Uuid,
    role_position: i32,
}

fn generate_upload_token() -> String {
    hex::encode(random::<[u8; 32]>())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn normalize_target_path(target_path: &str) -> Result<String, (Status, Json<serde_json::Value>)> {
    let target_path = target_path.trim();
    let safe_path = if target_path.is_empty() {
        PathBuf::new()
    } else {
        sanitize_path(PathBuf::from(target_path)).ok_or(bad_request("Invalid destination path"))?
    };

    Ok(path_to_web_string(&safe_path))
}

fn sanitize_filename(filename: &str) -> Result<String, (Status, Json<serde_json::Value>)> {
    if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
        return Err(bad_request("Invalid file name"));
    }

    let safe_path =
        sanitize_path(PathBuf::from(filename)).ok_or(bad_request("Invalid file name"))?;
    if safe_path.components().count() != 1 {
        return Err(bad_request("Invalid file name"));
    }

    safe_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| bad_request("Invalid file name"))
}

async fn actor_role(
    client: &deadpool_postgres::Object,
    user_id: Uuid,
) -> Result<ActorRole, (Status, Json<serde_json::Value>)> {
    let row = client
        .query_opt(
            "SELECT r.name, r.position
             FROM users u
             JOIN roles r ON r.id = u.role_id
             WHERE u.id = $1",
            &[&user_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("User not found"))?;

    Ok(ActorRole {
        is_admin: row.get::<_, String>("name") == "admin",
        position: row.get("position"),
    })
}

fn row_to_upload_link(row: &tokio_postgres::Row, can_delete: bool) -> UploadLink {
    UploadLink {
        id: row.get("id"),
        target_path: row.get("target_path"),
        created_by_user_id: row.get("created_by_user_id"),
        created_by_username: row.get("created_by_username"),
        created_at: row.get::<_, DateTime<Utc>>("created_at"),
        used_at: row.get::<_, Option<DateTime<Utc>>>("used_at"),
        can_delete,
    }
}

/// POST /api/upload-links — Create a one-time upload link for a destination directory.
#[post("/upload-links", data = "<request>")]
pub async fn create_upload_link(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    request: Json<CreateUploadLinkRequest>,
) -> Result<Json<CreatedUploadLink>, (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, "create_upload_links")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }

    let target_path = normalize_target_path(&request.target_path)?;
    let token = generate_upload_token();
    let token_hash = hash_token(&token);
    let client = get_client(pool).await?;
    let row = client
        .query_one(
            "INSERT INTO upload_links (token_hash, target_path, created_by_user_id)
             VALUES ($1, $2, $3)
             RETURNING id, target_path, created_by_user_id, created_at, used_at",
            &[&token_hash, &target_path, &user.id],
        )
        .await
        .map_err(db_error)?;

    Ok(Json(CreatedUploadLink {
        link: UploadLink {
            id: row.get("id"),
            target_path: row.get("target_path"),
            created_by_user_id: row.get("created_by_user_id"),
            created_by_username: user.username,
            created_at: row.get("created_at"),
            used_at: row.get("used_at"),
            can_delete: true,
        },
        token,
    }))
}

/// GET /api/upload-links — List links. Creators can see their own; view permission sees all.
#[get("/upload-links")]
pub async fn list_upload_links(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<UploadLink>>, (Status, Json<serde_json::Value>)> {
    let can_view_all = check_permission(pool, user.id, "view_upload_links")
        .await
        .unwrap_or(false);
    let can_create = check_permission(pool, user.id, "create_upload_links")
        .await
        .unwrap_or(false);
    if !can_view_all && !can_create {
        return Err(forbidden());
    }
    let can_delete_others = check_permission(pool, user.id, "delete_upload_links")
        .await
        .unwrap_or(false);

    let client = get_client(pool).await?;
    let actor = actor_role(&client, user.id).await?;
    let rows = if can_view_all {
        client
            .query(
                "SELECT l.id, l.target_path, l.created_by_user_id, u.username AS created_by_username,
                        l.created_at, l.used_at, r.position AS creator_role_position
                 FROM upload_links l
                 JOIN users u ON u.id = l.created_by_user_id
                 JOIN roles r ON r.id = u.role_id
                 ORDER BY l.created_at DESC",
                &[],
            )
            .await
            .map_err(db_error)?
    } else {
        client
            .query(
                "SELECT l.id, l.target_path, l.created_by_user_id, u.username AS created_by_username,
                        l.created_at, l.used_at, r.position AS creator_role_position
                 FROM upload_links l
                 JOIN users u ON u.id = l.created_by_user_id
                 JOIN roles r ON r.id = u.role_id
                 WHERE l.created_by_user_id = $1
                 ORDER BY l.created_at DESC",
                &[&user.id],
            )
            .await
            .map_err(db_error)?
    };

    let links = rows
        .iter()
        .map(|row| {
            let owner = LinkOwner {
                user_id: row.get("created_by_user_id"),
                role_position: row.get("creator_role_position"),
            };
            let can_delete = owner.user_id == user.id
                || actor.is_admin
                || (can_delete_others && actor.position < owner.role_position);
            row_to_upload_link(row, can_delete)
        })
        .collect();

    Ok(Json(links))
}

/// DELETE /api/upload-links/<id> — Delete own link, or link created by lower role with permission.
#[delete("/upload-links/<id>")]
pub async fn delete_upload_link(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: String,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let id = Uuid::parse_str(&id).map_err(|_| bad_request("Invalid upload link ID"))?;
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT l.created_by_user_id, r.position AS creator_role_position
             FROM upload_links l
             JOIN users u ON u.id = l.created_by_user_id
             JOIN roles r ON r.id = u.role_id
             WHERE l.id = $1",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload link not found"))?;
    let owner = LinkOwner {
        user_id: row.get("created_by_user_id"),
        role_position: row.get("creator_role_position"),
    };

    if owner.user_id != user.id {
        let actor = actor_role(&client, user.id).await?;
        let can_delete_others = check_permission(pool, user.id, "delete_upload_links")
            .await
            .unwrap_or(false);
        if !actor.is_admin && (!can_delete_others || actor.position >= owner.role_position) {
            return Err(forbidden());
        }
    }

    client
        .execute("DELETE FROM upload_links WHERE id = $1", &[&id])
        .await
        .map_err(db_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// GET /api/public/upload-links/<token> — Validate a one-time upload link.
#[get("/public/upload-links/<token>")]
pub async fn get_public_upload_link(
    pool: &State<Pool>,
    token: &str,
) -> Result<Json<PublicUploadLinkStatus>, (Status, Json<serde_json::Value>)> {
    let token_hash = hash_token(token);
    let client = get_client(pool).await?;
    let exists = client
        .query_opt(
            "SELECT 1 FROM upload_links WHERE token_hash = $1 AND used_at IS NULL",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .is_some();

    if !exists {
        return Err(not_found("Upload link is invalid or has already been used"));
    }

    Ok(Json(PublicUploadLinkStatus { ready: true }))
}

/// PUT /api/public/upload-links/<token>?filename=<name> — Consume link and upload one file.
#[put("/public/upload-links/<token>?<filename>", data = "<data>")]
pub async fn upload_with_link(
    pool: &State<Pool>,
    token: &str,
    filename: String,
    data: Data<'_>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let filename = sanitize_filename(&filename)?;
    let token_hash = hash_token(token);
    let client = get_client(pool).await?;

    // Consume before writing. Concurrent requests can never both use this link.
    let row = client
        .query_opt(
            "UPDATE upload_links
             SET used_at = NOW()
             WHERE token_hash = $1 AND used_at IS NULL
             RETURNING target_path",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload link is invalid or has already been used"))?;
    let target_path: String = row.get("target_path");
    let full_path = Path::new(crate::shared::STORAGE_ROOT)
        .join(target_path)
        .join(filename);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }

    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full_path)
        .await
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(conflict("A file with this name already exists"));
        }
        Err(_) => return Err(server_error()),
    };

    if data
        .open(10.gibibytes())
        .stream_to(&mut file)
        .await
        .is_err()
        || file.flush().await.is_err()
    {
        drop(file);
        fs::remove_file(&full_path).await.ok();
        return Err(server_error());
    }

    Ok(Json(serde_json::json!({"success": true})))
}
