use super::*;

use std::path::PathBuf;
use tokio::fs;

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
