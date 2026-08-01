use deadpool_postgres::Pool;
use rocket::State;
use rocket::data::Data;
use uuid::Uuid;

use crate::auth::{AuthenticatedUser, require_permission};
use crate::shared::*;

#[options("/music/uploads")]
pub(crate) fn music_tus_options() -> TusResponse {
    TusResponse::options()
}

#[post("/music/uploads")]
pub(crate) async fn create_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    require_permission(pool, user.id, "music_upload").await?;
    cleanup_expired_uploads(pool).await;
    create_user_upload(pool, user.id, MUSIC_ROOT, &headers, "/api/music/uploads").await
}

#[head("/music/uploads/<id>")]
pub(crate) async fn head_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "music_upload").await?;
    head_user_upload(pool, user.id, id).await
}

#[patch("/music/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "music_upload").await?;
    let (response, completed) =
        patch_user_upload(pool, user.id, MUSIC_ROOT, &headers, id, data).await?;
    if let Some(target_path) = completed {
        finalize_music_upload(pool, id, user.id, &target_path).await?;
    }
    Ok(response)
}

#[delete("/music/uploads/<id>")]
pub(crate) async fn terminate_music_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_permission(pool, user.id, "music_upload").await?;
    terminate_user_upload(pool, MUSIC_ROOT, user.id, id).await
}

async fn finalize_music_upload(
    pool: &Pool,
    id: Uuid,
    user_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    finalize_user_upload(pool, MUSIC_ROOT, id, user_id, target_path).await?;
    if let Err(error) = super::scan_and_insert_song(pool, target_path).await {
        eprintln!("Failed to scan tags for {target_path}: {error}");
    }
    Ok(())
}
