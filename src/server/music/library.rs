use super::*;

use uuid::Uuid;

async fn library_song(
    pool: &Pool,
    song_id: &str,
) -> Result<(Uuid, deadpool_postgres::Object), ApiError> {
    let song_id = Uuid::parse_str(song_id).map_err(|_| not_found("Invalid song ID"))?;
    Ok((song_id, get_client(pool).await?))
}

#[get("/music/library?<page>&<limit>&<search>")]
pub(crate) async fn list_personal_library(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    page: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
) -> Result<Json<SongListResponse>, (Status, Json<serde_json::Value>)> {
    let page = Page::new(page, limit);
    let client = get_client(pool).await?;

    let search_pattern = search.map(|s| format!("%{s}%"));

    let total: i64 = if let Some(ref pattern) = search_pattern {
        client
            .query_one(
                "SELECT COUNT(*) FROM user_songs us JOIN songs s ON us.song_id=s.id \
                 WHERE us.user_id = $1 \
                 AND (s.title ILIKE $2 OR s.artist ILIKE $2 OR s.album ILIKE $2)",
                &[&user.id, pattern],
            )
            .await
            .map_err(db_error)?
            .get(0)
    } else {
        client
            .query_one(
                "SELECT COUNT(*) FROM user_songs us JOIN songs s ON us.song_id=s.id \
                 WHERE us.user_id = $1",
                &[&user.id],
            )
            .await
            .map_err(db_error)?
            .get(0)
    };

    let rows = if let Some(ref pattern) = search_pattern {
        client
            .query(
                "SELECT s.* FROM user_songs us JOIN songs s ON us.song_id=s.id \
                 WHERE us.user_id = $1 \
                 AND (s.title ILIKE $2 OR s.artist ILIKE $2 OR s.album ILIKE $2) \
                 ORDER BY s.artist, s.album, s.disc_number, s.track_number, s.title \
                 LIMIT $3 OFFSET $4",
                &[&user.id, pattern, &page.limit, &page.offset],
            )
            .await
            .map_err(db_error)?
    } else {
        client
            .query(
                "SELECT s.* FROM user_songs us JOIN songs s ON us.song_id=s.id \
                 WHERE us.user_id = $1 \
                 ORDER BY s.artist, s.album, s.disc_number, s.track_number, s.title \
                 LIMIT $2 OFFSET $3",
                &[&user.id, &page.limit, &page.offset],
            )
            .await
            .map_err(db_error)?
    };

    let songs: Vec<SongResponse> = rows.iter().map(row_to_song).collect();
    Ok(Json(SongListResponse {
        songs,
        total,
        page: page.number,
        limit: page.limit,
    }))
}

#[post("/music/library/<song_id>")]
pub(crate) async fn add_to_library(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    song_id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let (song_id, client) = library_song(pool, song_id).await?;
    if client
        .query_opt("SELECT 1 FROM songs WHERE id=$1", &[&song_id])
        .await
        .map_err(db_error)?
        .is_none()
    {
        return Err(not_found("Song not found"));
    }
    client
        .execute(
            "INSERT INTO user_songs (user_id,song_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
            &[&user.id, &song_id],
        )
        .await
        .map_err(db_error)?;
    Ok(Json(serde_json::json!({"message": "Added to library"})))
}

#[delete("/music/library/<song_id>")]
pub(crate) async fn remove_from_library(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    song_id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let (song_id, client) = library_song(pool, song_id).await?;
    let deleted = client
        .execute(
            "DELETE FROM user_songs WHERE user_id=$1 AND song_id=$2",
            &[&user.id, &song_id],
        )
        .await
        .map_err(db_error)?;
    if deleted == 0 {
        return Err(not_found("Song not in your library"));
    }
    Ok(Json(serde_json::json!({"message": "Removed from library"})))
}
