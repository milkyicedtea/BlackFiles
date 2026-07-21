use crate::guards::{AuthenticatedUser, check_permission};
use crate::models::{
    CreateUploadLinkRequest, CreatedUploadLink, PublicTusUpload, PublicUploadLinkStatus, UploadLink,
};
use crate::shared::{
    bad_request, conflict, db_error, forbidden, get_client, not_found, path_to_web_string,
    sanitize_path,
};
use crate::tus::cleanup_expired_uploads;
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use rand::random;
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
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

pub(crate) fn hash_token(token: &str) -> String {
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
    cleanup_expired_uploads(pool).await;

    let id = Uuid::parse_str(&id).map_err(|_| bad_request("Invalid upload link ID"))?;
    let mut client = get_client(pool).await?;
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

    let transaction = client.transaction().await.map_err(db_error)?;
    let locked = transaction
        .query_opt(
            "SELECT id FROM upload_links WHERE id = $1 FOR UPDATE",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .is_some();
    if !locked {
        return Err(not_found("Upload link not found"));
    }
    if transaction
        .query_opt(
            "SELECT 1 FROM upload_sessions WHERE upload_link_id = $1",
            &[&id],
        )
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Err(conflict(
            "Cannot delete an upload link while its upload is in progress",
        ));
    }
    transaction
        .execute("DELETE FROM upload_links WHERE id = $1", &[&id])
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// GET /api/public/upload-links/<token> — Validate a one-time upload link and resume state.
#[get("/public/upload-links/<token>")]
pub async fn get_public_upload_link(
    pool: &State<Pool>,
    token: &str,
) -> Result<Json<PublicUploadLinkStatus>, (Status, Json<serde_json::Value>)> {
    cleanup_expired_uploads(pool).await;

    let token_hash = hash_token(token);
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT s.id, s.target_path, s.upload_length, s.upload_offset
             FROM upload_links l
             LEFT JOIN upload_sessions s
               ON s.upload_link_id = l.id AND s.expires_at > NOW()
             WHERE l.token_hash = $1 AND l.used_at IS NULL",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload link is invalid or has already been used"))?;
    let session = row.get::<_, Option<Uuid>>("id").map(|id| PublicTusUpload {
        id,
        target_path: row.get("target_path"),
        upload_length: row.get("upload_length"),
        upload_offset: row.get("upload_offset"),
    });

    Ok(Json(PublicUploadLinkStatus {
        ready: true,
        session,
    }))
}
