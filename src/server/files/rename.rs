use super::*;

use std::path::PathBuf;
use tokio::fs;

#[put("/rename", data = "<request>")]
pub async fn rename_path(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    request: Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    require_permission(pool, user.id, "rename_files").await?;

    let new_name = request.new_name.trim().to_string();
    if new_name.is_empty() || new_name.contains('/') || new_name.contains('\0') {
        return Err(bad_request("Invalid name"));
    }
    if new_name.starts_with('.') {
        return Err(bad_request("Filenames cannot start with a dot"));
    }

    let source = request.path.trim();
    if source.is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }

    let safe_path = sanitize_path(PathBuf::from(source)).ok_or(bad_request("Invalid path"))?;

    let canonical = canonical_path(STORAGE_ROOT, &safe_path, "Cannot rename storage root").await?;

    let parent = safe_path.parent().unwrap_or(Path::new(""));
    let new_path = Path::new(STORAGE_ROOT).join(parent).join(&new_name);

    if fs::metadata(&new_path).await.is_ok() {
        return Err(conflict("A file or folder with this name already exists"));
    }

    fs::rename(&canonical, &new_path)
        .await
        .map_err(|_| server_error())?;

    Ok(Json(serde_json::json!({"success": true})))
}
