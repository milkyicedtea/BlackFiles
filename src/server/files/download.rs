use super::*;

use crate::shared::FileResponse;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};

#[get("/files/<path..>")]
pub async fn download(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<FileResponse, Status> {
    require_file_permission(pool, user.id, "download_files").await?;
    let safe_path = sanitize_path(path).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    if !metadata.is_file() {
        return Err(Status::NotFound);
    }

    let canonical = canonical_path_status(STORAGE_ROOT, &safe_path).await?;

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
