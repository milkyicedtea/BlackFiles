use std::path::{Path, PathBuf};
use std::time::SystemTime;
use rocket::http::Status;
use rocket::serde::json::Json;
use tokio::fs;
use crate::shared::{sanitize_path, FileEntry, STORAGE_ROOT};

#[get("/list/<path..>")]
pub(crate) async fn list_directory(path: PathBuf) -> Result<Json<Vec<FileEntry>>, Status> {
    let safe_path = sanitize_path(path.clone()).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    let canonical = fs::canonicalize(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !canonical.starts_with(&canonical_root) {
        return Err(Status::Forbidden);
    }

    let mut entries = Vec::new();
    let mut dir = fs::read_dir(&canonical).await.map_err(|_| Status::NotFound)?;

    while let Some(entry) = dir.next_entry().await.map_err(|_| Status::InternalServerError)? {
        let metadata = entry.metadata().await.map_err(|_| Status::InternalServerError)?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let relative_path = if safe_path.as_os_str().is_empty() {
            name.clone()
        } else {
            format!("{}/{}", safe_path.display(), name)
        };

        let modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        entries.push(FileEntry {
            name,
            is_dir: metadata.is_dir(),
            path: relative_path,
            size: metadata.len(),
            modified,
        });
    }

    // Sort: directories first, then files, alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(Json(entries))
}

#[get("/list")]
pub(crate) async fn list_root() -> Result<Json<Vec<FileEntry>>, Status> {
    list_directory(PathBuf::new()).await
}
