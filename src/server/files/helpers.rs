use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::auth::has_permission;
use crate::shared::{ApiError, bad_request, forbidden, not_found, server_error};

pub(crate) async fn require_file_permission(
    pool: &deadpool_postgres::Pool,
    user_id: Uuid,
    permission: &str,
) -> Result<(), rocket::http::Status> {
    if has_permission(pool, user_id, permission).await {
        Ok(())
    } else {
        Err(rocket::http::Status::Forbidden)
    }
}

pub(crate) async fn canonical_path(
    root: &str,
    relative_path: &Path,
    root_error: &str,
) -> Result<PathBuf, ApiError> {
    let canonical = fs::canonicalize(Path::new(root).join(relative_path))
        .await
        .map_err(|_| not_found("File not found"))?;
    let canonical_root = fs::canonicalize(root).await.map_err(|_| server_error())?;

    if canonical == canonical_root {
        return Err(bad_request(root_error));
    }
    if !canonical.starts_with(canonical_root) {
        return Err(forbidden());
    }
    Ok(canonical)
}

pub(crate) async fn canonical_path_status(
    root: &str,
    relative_path: &Path,
) -> Result<PathBuf, rocket::http::Status> {
    let canonical = fs::canonicalize(Path::new(root).join(relative_path))
        .await
        .map_err(|_| rocket::http::Status::NotFound)?;
    let canonical_root = fs::canonicalize(root)
        .await
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    if canonical.starts_with(canonical_root) {
        Ok(canonical)
    } else {
        Err(rocket::http::Status::Forbidden)
    }
}
