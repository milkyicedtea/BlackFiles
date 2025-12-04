#[macro_use]
extern crate rocket;

use rocket::http::{Status};
use rocket::response::stream::{ReaderStream};
use rocket::tokio::fs::File;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use rocket::response::content::RawHtml;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use tokio::fs;

const STORAGE_ROOT: &str = "storage";

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct FileEntry {
    name: String,
    is_dir: bool,
    path: String,
    size: u64,
    modified: u64, // unix time
}

#[get("/<path..>", rank = 2)]
async fn index_with_path(path: PathBuf) -> RawHtml<&'static str> {
    RawHtml(include_str!("frontend/index.html"))
}

#[get("/", rank = 1)]
async fn index() -> RawHtml<&'static str> {
    RawHtml(include_str!("frontend/index.html"))
}

#[get("/api/list/<path..>")]
async fn list_directory(path: PathBuf) -> Result<Json<Vec<FileEntry>>, Status> {
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

#[get("/api/list")]
async fn list_root() -> Result<Json<Vec<FileEntry>>, Status> {
    list_directory(PathBuf::new()).await
}

#[get("/files/<path..>")]
async fn download(path: PathBuf) -> Result<ReaderStream![File], Status> {
    let safe_path = sanitize_path(path).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(safe_path);

    // get metadata and check if it's a regular
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

    // prevent access to hidden files
    if let Some(filename) = canonical.file_name() {
        if filename.to_string_lossy().starts_with('.') {
            return Err(Status::Forbidden);
        }
    }

    let file = File::open(&canonical).await.map_err(|_| Status::NotFound)?;
    Ok(ReaderStream::one(file))
}

fn sanitize_path(path: PathBuf) -> Option<PathBuf> {
    // allow empty paths for root directory
    if path.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }

    let mut clean = PathBuf::new();

    for part in path.components() {
        match part {
            std::path::Component::Normal(p) => {
                // additional check to reject null bytes or looks sus
                let s = p.to_string_lossy();
                if s.contains('\0') || s == "." || s == ".." {
                    return None;
                }
                clean.push(p);
            }
            _ => return None,
        }
    }

    // reject empty paths
    if clean.as_os_str().is_empty() {
        return None;
    }

    Some(clean)
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, index_with_path, list_root, list_directory, download])
}

