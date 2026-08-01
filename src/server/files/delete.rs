use super::*;

use std::path::PathBuf;
use tokio::fs;

#[delete("/files/<path..>")]
pub async fn delete_path(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    require_permission(pool, user.id, "delete_files").await?;

    let safe_path = sanitize_path(path).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }

    let canonical = canonical_path(STORAGE_ROOT, &safe_path, "Cannot delete storage root").await?;

    let metadata = fs::metadata(&canonical)
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
