use super::*;

// ── Shared helpers (pub(super) for sub-modules) ──

// ── Phase 4: Browsing endpoints ──

/// Encodes an artist name as a safe ID string.
pub(super) fn artist_id(name: &str) -> String {
    format!("ar:{}", name)
}

/// Encodes artist+album as a safe album ID string.
pub(super) fn album_id(artist: &str, album: &str) -> String {
    format!("al:{}|{}", artist, album)
}

/// Builds a SongEntry from a DB row.
pub(super) fn row_to_song_entry(row: &tokio_postgres::Row) -> SongEntry {
    let format: String = row.get("format");
    SongEntry {
        id: row.get::<_, Uuid>("id").to_string(),
        title: row.get("title"),
        artist: row.get("artist"),
        album: row.get("album"),
        track: row.try_get("track_number").ok(),
        year: row.try_get("year").ok(),
        genre: row.try_get("genre").ok(),
        cover_art: row.try_get::<_, bool>("has_cover_art").ok().and_then(|b| {
            if b {
                Some(row.get::<_, Uuid>("id").to_string())
            } else {
                None
            }
        }),
        duration: row.try_get("duration_seconds").ok(),
        bitrate: row.try_get("bitrate_kbps").ok(),
        size: row.try_get("size_bytes").ok(),
        content_type: format_to_mime(&format).to_string(),
        is_dir: false,
        is_video: false,
    }
}

pub(super) fn format_to_mime(fmt: &str) -> &str {
    match fmt {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" | "oga" => "audio/ogg",
        "m4a" | "aac" | "mp4" => "audio/mp4",
        "wav" => "audio/wav",
        "wma" => "audio/x-ms-wma",
        "opus" => "audio/opus",
        _ => "audio/mpeg",
    }
}

// ── Phase 5: Media & Search endpoints ──

/// Parse an HTTP Range header value. Returns (start, end_inclusive) in bytes.
pub(super) fn parse_range_header(range_header: &str, file_size: u64) -> Option<(u64, u64)> {
    let prefix = "bytes=";
    let range_value = range_header.strip_prefix(prefix)?;
    let (start_str, end_str) = range_value.split_once('-')?;

    if start_str.is_empty() {
        let suffix: u64 = end_str.parse().ok()?;
        if suffix == 0 || suffix > file_size {
            return None;
        }
        Some((file_size - suffix, file_size - 1))
    } else {
        let start: u64 = start_str.parse().ok()?;
        if start >= file_size {
            return None;
        }
        let end = if end_str.is_empty() {
            file_size - 1
        } else {
            let e: u64 = end_str.parse().ok()?;
            e.min(file_size - 1)
        };
        if start > end {
            return None;
        }
        Some((start, end))
    }
}

/// Helper: look up a song by UUID, verifying it belongs to the user's personal library.
pub(super) async fn get_user_song(
    client: &deadpool_postgres::Object,
    user_id: Uuid,
    song_id: Uuid,
) -> Result<tokio_postgres::Row, ()> {
    client
        .query_opt(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.id = $2",
            &[&user_id, &song_id],
        )
        .await
        .map_err(|_| ())?
        .ok_or(())
}

pub(crate) struct RangeHeader(pub(crate) Option<String>);

// ── URL decode ──

pub(super) fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    out.push(b as char);
                } else {
                    out.push('%');
                }
            }
            '+' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

// ── Phase 6 helpers ──

pub(super) fn db_err_resp() -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(SubsonicResponse::<EmptyResponse>::error(
            0,
            "Database error",
        ))
        .unwrap_or_default(),
    )
}

pub(super) fn not_found_resp() -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(SubsonicResponse::<EmptyResponse>::error(
            70,
            "Resource not found",
        ))
        .unwrap_or_default(),
    )
}

pub(super) fn param_err(msg: &str) -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(SubsonicResponse::<EmptyResponse>::error(10, msg)).unwrap_or_default(),
    )
}

pub(super) fn ok_empty_resp() -> Json<serde_json::Value> {
    Json(serde_json::to_value(SubsonicResponse::<EmptyResponse>::ok_empty()).unwrap_or_default())
}

pub(super) fn ok_resp<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(serde_json::to_value(SubsonicResponse::ok(data)).unwrap_or_default())
}

/// Decode an artist id of the form `ar:{name}`.
pub(super) fn decode_artist_id(id: &str) -> Option<String> {
    id.strip_prefix("ar:").map(|s| s.to_string())
}

/// Decode an album id of the form `al:{artist}|{album}`.
pub(super) fn decode_album_id(id: &str) -> Option<(String, String)> {
    let body = id.strip_prefix("al:")?;
    let (artist, album) = body.split_once('|')?;
    Some((artist.to_string(), album.to_string()))
}

/// Of the requested song ids, return those present in the user's personal
/// library, preserving order and dropping duplicates. (A playlist only ever
/// references songs the user can stream.)
pub(super) async fn personal_song_ids(
    client: &deadpool_postgres::Object,
    user_id: Uuid,
    songs: &[Uuid],
) -> Result<Vec<Uuid>, tokio_postgres::Error> {
    if songs.is_empty() {
        return Ok(Vec::new());
    }
    let rows = client
        .query(
            "SELECT s.id FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.id = ANY($2)",
            &[&user_id, &songs],
        )
        .await?;
    let present: HashSet<Uuid> = rows.iter().map(|r| r.get::<_, Uuid>("id")).collect();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for u in songs {
        if present.contains(u) && seen.insert(*u) {
            out.push(*u);
        }
    }
    Ok(out)
}

/// True if `song_id` is in the user's personal library.
pub(super) async fn in_personal_library(
    client: &deadpool_postgres::Object,
    user_id: Uuid,
    song_id: Uuid,
) -> Result<bool, tokio_postgres::Error> {
    Ok(client
        .query_opt(
            "SELECT 1 FROM user_songs WHERE user_id = $1 AND song_id = $2",
            &[&user_id, &song_id],
        )
        .await?
        .is_some())
}
