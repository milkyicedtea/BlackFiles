use super::*;

use std::path::Path;
use tokio::fs;
use uuid::Uuid;

pub(crate) fn row_to_song(row: &tokio_postgres::Row) -> SongResponse {
    SongResponse {
        id: row.get("id"),
        file_path: row.get("file_path"),
        title: row.get("title"),
        artist: row.get("artist"),
        album: row.get("album"),
        album_artist: row.get("album_artist"),
        genre: row.get("genre"),
        year: row.get("year"),
        track_number: row.get("track_number"),
        disc_number: row.get("disc_number"),
        duration_seconds: row.get("duration_seconds"),
        size_bytes: row.get("size_bytes"),
        format: row.get("format"),
        bitrate_kbps: row.get("bitrate_kbps"),
        has_cover_art: row.get("has_cover_art"),
        in_library: row.try_get("in_library").unwrap_or(false),
        created_at: row
            .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
            .to_rfc3339(),
        updated_at: row
            .get::<_, chrono::DateTime<chrono::Utc>>("updated_at")
            .to_rfc3339(),
    }
}

#[allow(clippy::too_many_arguments)]
#[get("/music/songs?<page>&<limit>&<search>&<artist>&<album>&<genre>")]
pub(crate) async fn list_songs(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    page: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
) -> Result<Json<SongListResponse>, (Status, Json<serde_json::Value>)> {
    let page = Page::new(page, limit);
    let client = get_client(pool).await?;

    // Build WHERE clause with direct SQL params.
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
    let mut idx = 1;

    if let Some(s) = &search {
        conditions.push(format!(
            "(title ILIKE ${idx} OR artist ILIKE ${idx} OR album ILIKE ${idx})"
        ));
        params.push(s);
        idx += 1;
    }
    if let Some(a) = &artist {
        conditions.push(format!("artist ILIKE ${idx}"));
        params.push(a);
        idx += 1;
    }
    if let Some(a) = &album {
        conditions.push(format!("album ILIKE ${idx}"));
        params.push(a);
        idx += 1;
    }
    if let Some(g) = &genre {
        conditions.push(format!("genre ILIKE ${idx}"));
        params.push(g);
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let total: i64 = client
        .query_one(
            &format!("SELECT COUNT(*) FROM songs {where_clause}"),
            &params,
        )
        .await
        .map_err(db_error)?
        .get(0);

    let user_p = idx;
    params.push(&user.id);
    idx += 1;
    let lim_p = idx;
    let off_p = idx + 1;
    params.push(&page.limit);
    params.push(&page.offset);

    let rows = client
        .query(
            &format!(
                "SELECT songs.*, EXISTS (
                     SELECT 1 FROM user_songs
                     WHERE user_songs.user_id = ${user_p}
                       AND user_songs.song_id = songs.id
                 ) AS in_library
                 FROM songs {where_clause} \
                 ORDER BY artist, album, disc_number, track_number, title \
                 LIMIT ${lim_p} OFFSET ${off_p}"
            ),
            &params,
        )
        .await
        .map_err(db_error)?;

    let songs: Vec<SongResponse> = rows.iter().map(row_to_song).collect();
    Ok(Json(SongListResponse {
        songs,
        total,
        page: page.number,
        limit: page.limit,
    }))
}

#[get("/music/songs/selection?<search>")]
pub(crate) async fn list_song_selection(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    search: Option<String>,
) -> Result<Json<SongSelectionResponse>, ApiError> {
    let client = get_client(pool).await?;
    let rows = client
        .query(
            "SELECT songs.id, EXISTS (
                 SELECT 1 FROM user_songs
                 WHERE user_songs.user_id = $1
                   AND user_songs.song_id = songs.id
             ) AS in_library
             FROM songs
             WHERE (
                 $2::TEXT IS NULL
                 OR songs.title ILIKE $2
                 OR songs.artist ILIKE $2
                 OR songs.album ILIKE $2
             )
             ORDER BY songs.id",
            &[&user.id, &search],
        )
        .await
        .map_err(db_error)?;

    Ok(Json(SongSelectionResponse {
        songs: rows
            .iter()
            .map(|row| SongSelectionEntry {
                id: row.get("id"),
                in_library: row.get("in_library"),
            })
            .collect(),
    }))
}

#[delete("/music/songs/<id>")]
pub(crate) async fn delete_song(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    require_permission(pool, user.id, "music_delete").await?;
    let song_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid song ID"))?;
    let client = get_client(pool).await?;
    let row = client
        .query_opt("SELECT file_path FROM songs WHERE id = $1", &[&song_id])
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Song not found"))?;
    let file_path: String = row.get("file_path");

    let full_path = Path::new(MUSIC_ROOT).join(&file_path);
    if let Err(e) = fs::remove_file(&full_path).await {
        eprintln!("Warning: could not delete file {file_path}: {e}");
    }

    client
        .execute("DELETE FROM songs WHERE id = $1", &[&song_id])
        .await
        .map_err(db_error)?;
    Ok(Json(serde_json::json!({"message": "Song deleted"})))
}
