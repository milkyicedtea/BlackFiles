use std::path::{Path, PathBuf};
use rocket::http::Status;
use tokio::fs;
use tokio::fs::File;
use crate::lib::{sanitize_path, FileResponse, STORAGE_ROOT};

#[get("/<path..>")]
pub(crate) async fn download(path: PathBuf) -> Result<FileResponse, Status> {
    let safe_path = sanitize_path(path).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(safe_path);

    let metadata = fs::metadata(&full_path).await.map_err(|_| Status::NotFound)?;
    if !metadata.is_file() {
        return Err(Status::NotFound);
    }

    let canonical = fs::canonicalize(&full_path).await.map_err(|_| Status::NotFound)?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !canonical.starts_with(&canonical_root) {
        return Err(Status::Forbidden);
    }

    if let Some(filename) = canonical.file_name() {
        if filename.to_string_lossy().starts_with('.') {
            return Err(Status::Forbidden);
        }
    }

    let file = File::open(&canonical).await.map_err(|_| Status::NotFound)?;
    let file_size = metadata.len();

    Ok(FileResponse{
        stream: Box::new(file),
        size: file_size,
    })
}
