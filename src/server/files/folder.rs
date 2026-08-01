use super::*;

use std::path::PathBuf;
use tokio::fs;

#[post("/folders", data = "<request>")]
pub async fn create_folder(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    request: Json<CreateFolderRequest>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let has_perm = check_permission(pool, user.id, "create_folders")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(forbidden());
    }

    let name = request.name.trim().to_string();
    if name.is_empty() || name.contains('/') || name.contains('\0') {
        return Err(bad_request("Invalid folder name"));
    }

    let parent = request.parent_path.trim();
    let safe_parent = if parent.is_empty() {
        PathBuf::new()
    } else {
        sanitize_path(PathBuf::from(parent)).ok_or(bad_request("Invalid parent path"))?
    };

    let new_path = Path::new(STORAGE_ROOT).join(&safe_parent).join(&name);

    // Verify parent exists and is within storage root
    if !safe_parent.as_os_str().is_empty() {
        let parent_full = Path::new(STORAGE_ROOT).join(&safe_parent);
        let canonical_parent = fs::canonicalize(&parent_full)
            .await
            .map_err(|_| not_found("Parent directory not found"))?;
        let canonical_root = fs::canonicalize(STORAGE_ROOT)
            .await
            .map_err(|_| server_error())?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(forbidden());
        }
    }

    if fs::metadata(&new_path).await.is_ok() {
        return Err(conflict("A file or folder with this name already exists"));
    }

    fs::create_dir(&new_path)
        .await
        .map_err(|_| server_error())?;

    Ok(Json(serde_json::json!({"success": true})))
}
