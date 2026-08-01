use super::*;

use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use std::path::Path;
use uuid::Uuid;

// ── Response types ──

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SongResponse {
    pub id: Uuid,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i16>,
    pub track_number: Option<i16>,
    pub disc_number: Option<i16>,
    pub duration_seconds: Option<f32>,
    pub size_bytes: i64,
    pub format: Option<String>,
    pub bitrate_kbps: Option<i16>,
    pub has_cover_art: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SongListResponse {
    pub songs: Vec<SongResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UpdateTagsRequest {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i16>,
    pub track_number: Option<i16>,
    pub disc_number: Option<i16>,
}

// ── Tag scanner ──

pub(crate) async fn scan_and_insert_song(pool: &Pool, relative_path: &str) -> Result<Uuid, String> {
    let full_path = Path::new(MUSIC_ROOT).join(relative_path);

    let tagged_file = Probe::open(&full_path)
        .map_err(|e| format!("Cannot open file: {e}"))?
        .guess_file_type()
        .map_err(|e| format!("Cannot determine file type: {e}"))?
        .read()
        .map_err(|e| format!("Cannot read tags: {e}"))?;

    let properties = tagged_file.properties();
    let tag = tagged_file.primary_tag();

    let title = tag
        .and_then(|t| t.title().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            Path::new(relative_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });
    let artist = tag
        .and_then(|t| t.artist().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());
    let album = tag
        .and_then(|t| t.album().map(|s| s.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());
    let album_artist = tag.and_then(|t| t.get_string(ItemKey::AlbumArtist).map(|s| s.to_string()));
    let genre = tag.and_then(|t| t.genre().map(|s| s.to_string()));
    let year = tag.and_then(|t| {
        t.get_string(ItemKey::Year)
            .and_then(|s| s.parse::<i16>().ok())
    });
    let track_number = tag.and_then(|t| {
        t.get_string(ItemKey::TrackNumber)
            .and_then(|s| s.parse::<i16>().ok())
    });
    let disc_number = tag.and_then(|t| {
        t.get_string(ItemKey::DiscNumber)
            .and_then(|s| s.parse::<i16>().ok())
    });

    let duration_seconds = properties.duration().as_secs_f32();
    let bitrate_kbps = properties.audio_bitrate().map(|b| (b / 1000) as i16);

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|e| format!("Cannot stat file: {e}"))?;
    let size_bytes = metadata.len() as i64;

    let format = Path::new(relative_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    let has_cover_art = extract_cover_art(tag, relative_path).await;

    let client = pool
        .get()
        .await
        .map_err(|e| format!("Database connection error: {e}"))?;
    let song_id = Uuid::new_v4();

    client
        .execute(
            "INSERT INTO songs (id, file_path, title, artist, album, album_artist, genre, year,
                            track_number, disc_number, duration_seconds, size_bytes, format,
                            bitrate_kbps, has_cover_art)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         ON CONFLICT (file_path) DO UPDATE SET
            title = EXCLUDED.title, artist = EXCLUDED.artist, album = EXCLUDED.album,
            album_artist = EXCLUDED.album_artist, genre = EXCLUDED.genre, year = EXCLUDED.year,
            track_number = EXCLUDED.track_number, disc_number = EXCLUDED.disc_number,
            duration_seconds = EXCLUDED.duration_seconds, size_bytes = EXCLUDED.size_bytes,
            format = EXCLUDED.format, bitrate_kbps = EXCLUDED.bitrate_kbps,
            has_cover_art = EXCLUDED.has_cover_art, updated_at = NOW()",
            &[
                &song_id,
                &relative_path,
                &title,
                &artist,
                &album,
                &album_artist,
                &genre,
                &year,
                &track_number,
                &disc_number,
                &duration_seconds,
                &size_bytes,
                &format,
                &bitrate_kbps,
                &has_cover_art,
            ],
        )
        .await
        .map_err(|e| format!("Database error: {e}"))?;

    Ok(song_id)
}

async fn extract_cover_art(tag: Option<&lofty::tag::Tag>, relative_path: &str) -> bool {
    let tag = match tag {
        Some(t) => t,
        None => return false,
    };
    let front_cover = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first());
    let picture = match front_cover {
        Some(p) => p,
        None => return false,
    };

    let covers_dir = Path::new(MUSIC_ROOT).join(".covers");
    if fs::create_dir_all(&covers_dir).await.is_err() {
        return false;
    }

    let cover_name = relative_path
        .replace(['/', '\\', ' '], "_")
        .replace('.', "_");
    let ext = match picture.mime_type() {
        Some(mime) if mime.as_str() == "image/png" => "png",
        Some(mime) if mime.as_str() == "image/gif" => "gif",
        _ => "jpg",
    };
    let cover_path = covers_dir.join(format!("{cover_name}.{ext}"));
    fs::write(&cover_path, picture.data()).await.is_ok()
}

// ── Permission helper ──

pub(crate) async fn require_music_permission(
    pool: &Pool,
    user: &AuthenticatedUser,
    permission: &str,
) -> Result<(), (Status, Json<serde_json::Value>)> {
    if !check_permission(pool, user.id, permission)
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }
    Ok(())
}

#[put("/music/songs/<id>/tags", data = "<req>")]
pub(crate) async fn update_song_tags(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: &str,
    req: Json<UpdateTagsRequest>,
) -> Result<Json<SongResponse>, (Status, Json<serde_json::Value>)> {
    require_music_permission(pool, &user, "music_edit_tags").await?;
    let song_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid song ID"))?;
    let client = get_client(pool).await?;
    let row = client.query_opt(
        "SELECT file_path, title, artist, album, album_artist, genre, year, track_number, disc_number FROM songs WHERE id = $1",
        &[&song_id],
    ).await.map_err(db_error)?.ok_or_else(|| not_found("Song not found"))?;

    let file_path: String = row.get("file_path");
    let title = req.title.as_deref().unwrap_or(row.get("title"));
    let artist = req.artist.as_deref().unwrap_or(row.get("artist"));
    let album = req.album.as_deref().unwrap_or(row.get("album"));
    let album_artist = req.album_artist.as_deref().or(row.get("album_artist"));
    let genre = req.genre.as_deref().or(row.get("genre"));
    let year = req.year.or_else(|| row.get("year"));
    let track_number = req.track_number.or_else(|| row.get("track_number"));
    let disc_number = req.disc_number.or_else(|| row.get("disc_number"));

    client.execute(
        "UPDATE songs SET title=$1,artist=$2,album=$3,album_artist=$4,genre=$5,year=$6,track_number=$7,disc_number=$8,updated_at=NOW() WHERE id=$9",
        &[&title,&artist,&album,&album_artist,&genre,&year,&track_number,&disc_number,&song_id],
    ).await.map_err(db_error)?;

    let full_path = Path::new(MUSIC_ROOT).join(&file_path);
    if let Err(e) = write_file_tags(&full_path, &req).await {
        eprintln!("Warning: failed to write tags to {file_path}: {e}");
    }

    let updated = client
        .query_one("SELECT * FROM songs WHERE id = $1", &[&song_id])
        .await
        .map_err(db_error)?;
    Ok(Json(row_to_song(&updated)))
}

#[post("/music/scan")]
pub(crate) async fn scan_songs(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    require_music_permission(pool, &user, "music_upload").await?;
    let client = get_client(pool).await?;
    let rows = client
        .query("SELECT file_path FROM songs ORDER BY file_path", &[])
        .await
        .map_err(db_error)?;
    let mut scanned = 0i64;
    let mut failed = 0i64;
    for row in &rows {
        let file_path: String = row.get("file_path");
        match scan_and_insert_song(pool, &file_path).await {
            Ok(_) => scanned += 1,
            Err(e) => {
                eprintln!("Scan failed for {file_path}: {e}");
                failed += 1;
            }
        }
    }
    Ok(Json(
        serde_json::json!({"message": "Scan complete", "scanned": scanned, "failed": failed}),
    ))
}

// ── File tag writing ──

async fn write_file_tags(path: &Path, req: &UpdateTagsRequest) -> Result<(), String> {
    let mut tagged_file = Probe::open(path)
        .map_err(|e| format!("Cannot open file: {e}"))?
        .guess_file_type()
        .map_err(|e| format!("Cannot determine file type: {e}"))?
        .read()
        .map_err(|e| format!("Cannot read file: {e}"))?;

    let tag = match tagged_file.primary_tag_mut() {
        Some(t) => t,
        None => return Err("No writable tag found".to_string()),
    };

    if let Some(ref v) = req.title {
        tag.set_title(v.to_string());
    }
    if let Some(ref v) = req.artist {
        tag.set_artist(v.to_string());
    }
    if let Some(ref v) = req.album {
        tag.set_album(v.to_string());
    }
    if let Some(ref v) = req.album_artist {
        tag.insert_text(ItemKey::AlbumArtist, v.to_string());
    }
    if let Some(ref v) = req.genre {
        tag.set_genre(v.to_string());
    }
    if let Some(v) = req.year {
        tag.insert_text(ItemKey::Year, v.to_string());
    }
    if let Some(v) = req.track_number {
        tag.set_track(v as u32);
    }
    if let Some(v) = req.disc_number {
        tag.set_disk(v as u32);
    }

    use lofty::config::WriteOptions;
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|e| format!("Cannot write tags: {e}"))?;
    Ok(())
}
