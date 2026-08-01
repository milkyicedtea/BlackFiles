use deadpool_postgres::Pool;
use rocket::State;
use rocket::data::Data;
use rocket::serde::json::Json;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::auth::{AuthenticatedUser, require_permission};
use crate::shared::*;

#[options("/uploads")]
pub(crate) fn tus_options() -> TusResponse {
    TusResponse::options()
}

#[get("/uploads")]
pub(crate) async fn list_tus_uploads(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<PendingTusUpload>>, ApiError> {
    require_permission(pool, user.id, "upload_files").await?;
    cleanup_expired_uploads(pool).await;
    let client = get_client(pool).await?;
    let rows = client
        .query(
            "SELECT id, target_path, upload_length, upload_offset
             FROM upload_sessions
             WHERE user_id = $1 AND expires_at > NOW()
             ORDER BY created_at",
            &[&user.id],
        )
        .await
        .map_err(db_error)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| PendingTusUpload {
                id: row.get("id"),
                target_path: row.get("target_path"),
                upload_length: row.get("upload_length"),
                upload_offset: row.get("upload_offset"),
            })
            .collect(),
    ))
}

#[post("/uploads")]
pub(crate) async fn create_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    require_permission(pool, user.id, "upload_files").await?;
    cleanup_expired_uploads(pool).await;
    create_user_upload(pool, user.id, STORAGE_ROOT, &headers, "/api/uploads").await
}

#[head("/uploads/<id>")]
pub(crate) async fn head_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "upload_files").await?;
    head_user_upload(pool, user.id, id).await
}

#[patch("/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "upload_files").await?;
    let (response, completed) =
        patch_user_upload(pool, user.id, STORAGE_ROOT, &headers, id, data).await?;
    if let Some(target_path) = completed {
        finalize_user_upload(pool, STORAGE_ROOT, id, user.id, &target_path).await?;
    }
    Ok(response)
}

#[delete("/uploads/<id>")]
pub(crate) async fn terminate_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "upload_files").await?;
    terminate_user_upload(pool, STORAGE_ROOT, user.id, id).await
}

#[options("/public/upload-links/<_token>/uploads")]
pub(crate) fn public_tus_options(_token: &str) -> TusResponse {
    TusResponse::options()
}

#[post("/public/upload-links/<token>/uploads")]
pub(crate) async fn create_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    cleanup_expired_uploads(pool).await;
    let length = headers.upload_length()?;
    let filename = filename_from_metadata(headers.metadata()?)?;
    let token_hash = sha256_hex(token);

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let link = transaction
        .query_opt(
            "SELECT id, target_path FROM upload_links
             WHERE token_hash = $1 AND used_at IS NULL
             FOR UPDATE",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload link is invalid or has already been used"))?;
    let link_id: Uuid = link.get("id");
    let target_directory = PathBuf::from(link.get::<_, String>("target_path"));
    let relative_path = target_directory.join(filename);
    let target_path = path_to_web_string(&relative_path);
    let destination = Path::new(STORAGE_ROOT).join(relative_path);
    let (id, temporary, length) =
        prepare_temporary_upload(STORAGE_ROOT, &destination, length).await?;

    let inserted = transaction
        .execute(
            "INSERT INTO upload_sessions (id, upload_link_id, target_path, upload_length, expires_at)
             VALUES ($1, $2, $3, $4, NOW() + INTERVAL '24 hours')",
            &[&id, &link_id, &target_path, &length],
        )
        .await;
    if let Err(error) = inserted {
        fs::remove_file(&temporary).await.ok();
        if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
            return Err(conflict(
                "An upload for this link or destination is already in progress",
            ));
        }
        return Err(db_error(error));
    }
    transaction.commit().await.map_err(db_error)?;
    Ok(TusResponse::created(format!(
        "/api/public/upload-links/{token}/uploads/{id}"
    )))
}

#[head("/public/upload-links/<token>/uploads/<id>")]
pub(crate) async fn head_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    let token_hash = sha256_hex(token);
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT s.upload_length, s.upload_offset
             FROM upload_sessions s
             JOIN upload_links l ON l.id = s.upload_link_id
             WHERE s.id = $1 AND l.token_hash = $2 AND l.used_at IS NULL
               AND s.expires_at > NOW()",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    head_response(&row)
}

#[patch("/public/upload-links/<token>/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    let (requested_offset, content_length) = patch_headers(&headers)?;
    let token_hash = sha256_hex(token);
    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let row = transaction
        .query_opt(
            "SELECT s.upload_link_id, s.target_path, s.upload_length, s.upload_offset
             FROM upload_sessions s
             JOIN upload_links l ON l.id = s.upload_link_id
             WHERE s.id = $1 AND l.token_hash = $2 AND l.used_at IS NULL
               AND s.expires_at > NOW()
             FOR UPDATE OF s, l",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    let link_id: Uuid = row.get("upload_link_id");
    let progress = upload_progress(&row, requested_offset, content_length)?;
    let next_offset =
        write_upload_chunk(STORAGE_ROOT, id, progress.offset, content_length, data).await?;
    transaction
        .execute(
            "UPDATE upload_sessions SET upload_offset = $1
             WHERE id = $2 AND upload_link_id = $3",
            &[&as_i64(next_offset)?, &id, &link_id],
        )
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;

    if next_offset == progress.length {
        finalize_public_upload(pool, id, link_id, &progress.target_path).await?;
    }
    Ok(TusResponse::patched(next_offset))
}

async fn finalize_public_upload(
    pool: &Pool,
    id: Uuid,
    link_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    let temporary = temporary_path(STORAGE_ROOT, id);
    let destination = Path::new(STORAGE_ROOT).join(target_path);
    match fs::hard_link(&temporary, &destination).await {
        Ok(()) => {
            let mut client = get_client(pool).await?;
            let transaction = client.transaction().await.map_err(db_error)?;
            let marked = transaction
                .execute(
                    "UPDATE upload_links SET used_at = NOW()
                     WHERE id = $1 AND used_at IS NULL",
                    &[&link_id],
                )
                .await
                .map_err(db_error)?;
            if marked != 1 {
                return Err(conflict("Upload link is no longer available"));
            }
            transaction
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND upload_link_id = $2",
                    &[&id, &link_id],
                )
                .await
                .map_err(db_error)?;
            transaction.commit().await.map_err(db_error)?;
            fs::remove_file(&temporary)
                .await
                .map_err(|_| server_error())?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(&temporary).await.ok();
            let client = get_client(pool).await?;
            client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND upload_link_id = $2",
                    &[&id, &link_id],
                )
                .await
                .map_err(db_error)?;
            Err(conflict("A file with this name already exists"))
        }
        Err(_) => Err(server_error()),
    }
}

#[delete("/public/upload-links/<token>/uploads/<id>")]
pub(crate) async fn terminate_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    let token_hash = sha256_hex(token);
    let client = get_client(pool).await?;
    client
        .query_opt(
            "DELETE FROM upload_sessions s
             USING upload_links l
             WHERE s.id = $1 AND s.upload_link_id = l.id
               AND l.token_hash = $2 AND l.used_at IS NULL
             RETURNING s.id",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    fs::remove_file(temporary_path(STORAGE_ROOT, id)).await.ok();
    Ok(TusResponse::terminated())
}
