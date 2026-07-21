use crate::guards::{AuthenticatedUser, check_permission};
use crate::shared::{
    FileResponse, STORAGE_ROOT, bad_request, forbidden, not_found, sanitize_path, server_error,
};
use deadpool_postgres::Pool;
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::File;


#[get("/files/<path..>")]
pub async fn download(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<FileResponse, Status> {
    let has_perm = check_permission(pool, user.id, "download_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(Status::Forbidden);
    }
    let safe_path = sanitize_path(path).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(safe_path);

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    if !metadata.is_file() {
        return Err(Status::NotFound);
    }

    let canonical = fs::canonicalize(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !canonical.starts_with(&canonical_root) {
        return Err(Status::Forbidden);
    }

    if let Some(filename) = canonical.file_name()
        && filename.to_string_lossy().starts_with('.')
    {
        return Err(Status::Forbidden);
    }

    let file = File::open(&canonical).await.map_err(|_| Status::NotFound)?;
    let file_size = metadata.len();

    Ok(FileResponse {
        stream: Box::new(file),
        size: file_size,
    })
}


/// Delete a file or directory — DELETE /api/files/<path..>
#[delete("/files/<path..>")]
pub async fn delete_path(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let has_perm = check_permission(pool, user.id, "delete_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(forbidden());
    }

    let safe_path = sanitize_path(path).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }

    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    let canonical = fs::canonicalize(&full_path)
        .await
        .map_err(|_| not_found("File not found"))?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| server_error())?;

    if canonical == canonical_root {
        return Err(bad_request("Cannot delete storage root"));
    }
    if !canonical.starts_with(&canonical_root) {
        return Err(forbidden());
    }

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|_| not_found("File not found"))?;

    if metadata.is_dir() {
        fs::remove_dir_all(&canonical)
            .await
            .map_err(|_| server_error())?;
    } else {
        fs::remove_file(&canonical)
            .await
            .map_err(|_| server_error())?;
    }

    Ok(Json(serde_json::json!({"success": true})))
}

