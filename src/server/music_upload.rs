use deadpool_postgres::Pool;
use rocket::State;
use rocket::data::{Data, ToByteUnit};
use std::io::SeekFrom;
use std::path::Path;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::guards::{AuthenticatedUser, check_permission};
use crate::shared::{
    MUSIC_ROOT, bad_request, conflict, db_error, forbidden, get_client, not_found, server_error,
};

use super::tus::{
    ApiError, TusHeaders, TusResponse, cleanup_expired_uploads, destination_from_metadata,
    parse_upload_id, temporary_path,
};

const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024; // 10 GiB per file
const TUS_CHUNK_SIZE: u64 = 8 * 1024 * 1024;

const TEMP_DIRECTORY: &str = ".uploads";

async fn require_music_upload_permission(
    pool: &Pool,
    user: &AuthenticatedUser,
) -> Result<(), ApiError> {
    if !check_permission(pool, user.id, "music_upload")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }
    Ok(())
}

// ── TUS endpoints ──

#[options("/music/uploads")]
pub(crate) fn music_tus_options() -> TusResponse {
    TusResponse::options()
}

#[post("/music/uploads")]
pub(crate) async fn create_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    require_music_upload_permission(pool, &user).await?;
    cleanup_expired_uploads(pool).await;

    let length = headers.upload_length()?;
    if length > MAX_UPLOAD_SIZE {
        return Err(bad_request("Upload exceeds the maximum size"));
    }
    let (target_path, destination) = destination_from_metadata(MUSIC_ROOT, headers.metadata()?)?;

    match fs::metadata(&destination).await {
        Ok(_) => return Err(conflict("A file with this name already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(server_error()),
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }
    let temp_directory = Path::new(MUSIC_ROOT).join(TEMP_DIRECTORY);
    fs::create_dir_all(&temp_directory)
        .await
        .map_err(|_| server_error())?;

    let id = Uuid::new_v4();
    let temporary = temporary_path(MUSIC_ROOT, id);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;

    let length = as_i64(length)?;
    let client = get_client(pool).await?;
    let inserted = client
        .execute(
            "INSERT INTO upload_sessions (id, user_id, target_path, upload_length, expires_at)
             VALUES ($1, $2, $3, $4, NOW() + INTERVAL '24 hours')",
            &[&id, &user.id, &target_path, &length],
        )
        .await;

    if let Err(error) = inserted {
        fs::remove_file(&temporary).await.ok();
        if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
            return Err(conflict(
                "An upload to this destination is already in progress",
            ));
        }
        return Err(db_error(error));
    }

    Ok(TusResponse::created(format!("/api/music/uploads/{id}")))
}

#[head("/music/uploads/<id>")]
pub(crate) async fn head_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_music_upload_permission(pool, &user).await?;
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;

    Ok(TusResponse::head(
        as_u64(row.get::<_, i64>("upload_offset"))?,
        as_u64(row.get::<_, i64>("upload_length"))?,
    ))
}

#[patch("/music/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_music_upload_permission(pool, &user).await?;
    if !headers.is_offset_octet_stream() {
        return Err(bad_request(
            "PATCH requires application/offset+octet-stream",
        ));
    }

    let requested_offset = headers.upload_offset()?;
    let content_length = headers.content_length()?;
    if content_length > TUS_CHUNK_SIZE {
        return Err(bad_request("Upload chunk exceeds the maximum size"));
    }

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let row = transaction
        .query_opt(
            "SELECT target_path, upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()
             FOR UPDATE",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;

    let target_path: String = row.get("target_path");
    let length = as_u64(row.get::<_, i64>("upload_length"))?;
    let offset = as_u64(row.get::<_, i64>("upload_offset"))?;
    if offset == length {
        return Err(conflict("Upload is already complete"));
    }
    if requested_offset != offset {
        return Err(conflict("Upload offset does not match the server offset"));
    }
    let remaining = length.checked_sub(offset).ok_or_else(server_error)?;
    if content_length > remaining {
        return Err(bad_request("Upload chunk exceeds the declared file size"));
    }

    let temporary = temporary_path(MUSIC_ROOT, id);
    let metadata = fs::metadata(&temporary).await.map_err(|_| server_error())?;
    if metadata.len() != offset {
        eprintln!("Upload session {id} has an inconsistent temporary file offset");
        return Err(server_error());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;
    file.seek(SeekFrom::Start(offset))
        .await
        .map_err(|_| server_error())?;
    let written = *data
        .open(content_length.bytes())
        .stream_to(&mut file)
        .await
        .map_err(|_| server_error())?;
    if written != content_length {
        return Err(server_error());
    }
    file.flush().await.map_err(|_| server_error())?;
    file.sync_data().await.map_err(|_| server_error())?;

    let next_offset = offset.checked_add(written).ok_or_else(server_error)?;
    transaction
        .execute(
            "UPDATE upload_sessions SET upload_offset = $1 WHERE id = $2 AND user_id = $3",
            &[&as_i64(next_offset)?, &id, &user.id],
        )
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;

    if next_offset == length {
        finalize_music_upload(pool, id, user.id, &target_path).await?;
    }

    Ok(TusResponse::patched(next_offset))
}

#[delete("/music/uploads/<id>")]
pub(crate) async fn terminate_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_music_upload_permission(pool, &user).await?;
    let client = get_client(pool).await?;
    client
        .query_opt(
            "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2 RETURNING id",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    fs::remove_file(temporary_path(MUSIC_ROOT, id)).await.ok();

    Ok(TusResponse::terminated())
}

// ── Music-specific finalization with tag scanning ──

async fn finalize_music_upload(
    pool: &Pool,
    id: Uuid,
    user_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    let temporary = temporary_path(MUSIC_ROOT, id);
    let destination = Path::new(MUSIC_ROOT).join(target_path);

    match fs::hard_link(&temporary, &destination).await {
        Ok(()) => {
            fs::remove_file(&temporary)
                .await
                .map_err(|_| server_error())?;
            let client = get_client(pool).await?;
            if let Err(error) = client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2",
                    &[&id, &user_id],
                )
                .await
            {
                eprintln!(
                    "Completed music upload {id} could not be removed from the session table: {error}"
                );
            }

            // Scan ID3/Vorbis tags and insert into songs table
            if let Err(error) = super::music::scan_and_insert_song(pool, target_path).await {
                eprintln!("Failed to scan tags for {target_path}: {error}");
                // File still exists on disk; scan can be retried later
            }

            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(&temporary).await.ok();
            let client = get_client(pool).await?;
            client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2",
                    &[&id, &user_id],
                )
                .await
                .map_err(db_error)?;
            Err(conflict("A file with this name already exists"))
        }
        Err(_) => Err(server_error()),
    }
}

// ── Helpers ──

fn as_i64(value: u64) -> Result<i64, ApiError> {
    i64::try_from(value).map_err(|_| bad_request("Upload is too large"))
}

fn as_u64(value: i64) -> Result<u64, ApiError> {
    u64::try_from(value).map_err(|_| server_error())
}
