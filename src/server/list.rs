use crate::guards::{AuthenticatedUser, check_permission};
use crate::models::PaginationParams;
use crate::shared::{FileEntry, STORAGE_ROOT, path_to_web_string, sanitize_path, filter_by_search_term};
use deadpool_postgres::Pool;
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs;

/// Read all file entries in a directory, sorted (dirs first, then alphabetically).
async fn read_dir_entries(safe_path: &Path) -> Result<Vec<FileEntry>, Status> {
    let full_path = Path::new(STORAGE_ROOT).join(safe_path);

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
    let mut dir = fs::read_dir(&canonical)
        .await
        .map_err(|_| Status::NotFound)?;

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|_| Status::InternalServerError)?
    {
        let metadata = entry
            .metadata()
            .await
            .map_err(|_| Status::InternalServerError)?;
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files
        if name.starts_with('.') {
            continue;
        }

        let relative_path = if safe_path.as_os_str().is_empty() {
            path_to_web_string(Path::new(&name))
        } else {
            let p = Path::new(&safe_path).join(&name);
            path_to_web_string(&p)
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
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(entries)
}

#[get("/list/<path..>?<pagination..>")]
pub async fn list_directory(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
    pagination: PaginationParams,
) -> Result<Json<serde_json::Value>, Status> {
    let has_perm = check_permission(pool, user.id, "list_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(Status::Forbidden);
    }
    let safe_path = sanitize_path(path.clone()).ok_or(Status::BadRequest)?;
    let mut entries = read_dir_entries(&safe_path).await?;

    // Filter by search term
    filter_by_search_term(&pagination, &mut entries);

    let total = entries.len() as i64;

    // Apply pagination
    let limit = pagination.effective_limit() as usize;
    let offset = pagination.effective_offset() as usize;
    let data: Vec<FileEntry> = entries.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({"data": data, "total": total})))
}

#[get("/list?<pagination..>")]
pub async fn list_root(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    pagination: PaginationParams,
) -> Result<Json<serde_json::Value>, Status> {
    let has_perm = check_permission(pool, user.id, "list_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(Status::Forbidden);
    }
    let mut entries = read_dir_entries(Path::new("")).await?;

    // Filter by search term
    filter_by_search_term(&pagination, &mut entries);

    let total = entries.len() as i64;

    // Apply pagination
    let limit = pagination.effective_limit() as usize;
    let offset = pagination.effective_offset() as usize;
    let data: Vec<FileEntry> = entries.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({"data": data, "total": total})))
}
