use argon2::PasswordVerifier;
use deadpool_postgres::Pool;
use rocket::State;
use rocket::http::Status;
use rocket::http::{ContentType, Header};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Serialize, json::Json};
use uuid::Uuid;

use crate::api_keys::hash_api_key;
use crate::shared::MUSIC_ROOT;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

// ── Response envelope ──

pub const SUB_SERVER_TYPE: &str = "Blackfiles";
pub const SUB_SERVER_VERSION: &str = "0.1.0";

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicResponse<T: Serialize> {
    #[serde(rename = "subsonic-response")]
    pub body: SubsonicBody<T>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicBody<T: Serialize> {
    pub status: String,
    pub version: String,
    #[serde(rename = "type")]
    pub server_type: String,
    #[serde(rename = "serverVersion")]
    pub server_version: String,
    #[serde(rename = "openSubsonic")]
    pub open_subsonic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SubsonicError>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> SubsonicResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            body: SubsonicBody {
                status: "ok".into(),
                version: "1.16.1".into(),
                server_type: SUB_SERVER_TYPE.into(),
                server_version: SUB_SERVER_VERSION.into(),
                open_subsonic: true,
                error: None,
                data: Some(data),
            },
        }
    }
}

impl SubsonicResponse<EmptyResponse> {
    pub fn ok_empty() -> Self {
        Self::ok(EmptyResponse {})
    }

    pub fn error(code: i32, message: &str) -> Self {
        Self {
            body: SubsonicBody {
                status: "failed".into(),
                version: "1.16.1".into(),
                server_type: SUB_SERVER_TYPE.into(),
                server_version: SUB_SERVER_VERSION.into(),
                open_subsonic: true,
                error: Some(SubsonicError {
                    code,
                    message: message.into(),
                }),
                data: None,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct EmptyResponse {}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LicenseResponse {
    #[serde(default = "default_true")]
    pub valid: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExtensionsResponse {
    #[serde(rename = "openSubsonicExtensions")]
    pub extensions: Vec<ExtensionInfo>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExtensionInfo {
    pub name: String,
    pub versions: Vec<i32>,
}

// ── Phase 4: Browsing response types ──

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFoldersResponse {
    #[serde(rename = "musicFolders")]
    pub music_folders: MusicFolderList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFolderList {
    #[serde(rename = "musicFolder")]
    pub music_folder: Vec<MusicFolder>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFolder {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexesResponse {
    pub indexes: IndexList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexList {
    pub index: Vec<IndexEntry>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexEntry {
    pub name: String,
    pub artist: Vec<ArtistRef>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ArtistRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DirectoryResponse {
    pub directory: Directory,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Directory {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child: Option<Vec<ChildEntry>>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(untagged)]
pub enum ChildEntry {
    Dir {
        id: String,
        parent: String,
        title: String,
        artist: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cover_art: Option<String>,
        #[serde(rename = "isDir")]
        is_dir: bool,
    },
    Song {
        id: String,
        parent: String,
        title: String,
        artist: String,
        album: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        track: Option<i16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        year: Option<i16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        genre: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cover_art: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bitrate: Option<i16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<i64>,
        #[serde(rename = "contentType")]
        content_type: String,
        #[serde(rename = "isDir")]
        is_dir: bool,
        #[serde(rename = "isVideo")]
        is_video: bool,
    },
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArtistsResponse {
    pub artists: ArtistList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArtistList {
    pub index: Vec<IndexEntry>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArtistResponse {
    pub artist: ArtistDetail,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Vec<AlbumRef>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct AlbumRef {
    pub id: String,
    pub name: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumResponse {
    pub album: AlbumDetail,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumDetail {
    pub id: String,
    pub name: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<SongEntry>>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct SongEntry {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(rename = "isVideo")]
    pub is_video: bool,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SongResponse {
    pub song: SongEntry,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumListResponse {
    #[serde(rename = "albumList")]
    pub album_list: AlbumListContainer,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumListContainer {
    pub album: Vec<AlbumRef>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GenresResponse {
    pub genres: GenreList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GenreList {
    pub genre: Vec<GenreItem>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct GenreItem {
    pub name: String,
    #[serde(rename = "songCount")]
    pub song_count: i64,
    #[serde(rename = "albumCount")]
    pub album_count: i64,
}

// ── Phase 5: Media & Search response types ──

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SearchResponse {
    #[serde(rename = "searchResult")]
    pub search_result: SearchResult,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SearchResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<Vec<ArtistRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub albums: Option<Vec<AlbumRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub songs: Option<Vec<SongEntry>>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RandomSongsResponse {
    #[serde(rename = "randomSongs")]
    #[allow(non_snake_case)]
    pub randomSongs: RandomSongsList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RandomSongsList {
    pub song: Vec<SongEntry>,
}

// ── Error helper ──

fn api_err(code: i32, msg: &str) -> (Status, Json<serde_json::Value>) {
    let resp = SubsonicResponse::<EmptyResponse>::error(code, msg);
    (
        Status::Ok,
        Json(serde_json::to_value(&resp).unwrap_or_default()),
    )
}

// ── Subsonic User Guard ──

#[derive(Debug, Clone)]
pub struct SubsonicUser {
    pub id: Uuid,
    pub username: String,
}

fn query_param(request: &Request<'_>, key: &str) -> Option<String> {
    request.uri().query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (k, v) = pair.as_str().split_once('=')?;
            if k == key { Some(url_decode(v)) } else { None }
        })
    })
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SubsonicUser {
    type Error = Json<serde_json::Value>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = match request.guard::<&State<Pool>>().await {
            Outcome::Success(p) => p,
            _ => return Outcome::Error(api_err(0, "Server configuration error")),
        };

        // 1. apiKey auth
        if let Some(api_key) = query_param(request, "apiKey") {
            if !api_key.is_empty() {
                let key_hash = hash_api_key(&api_key);
                let client = match pool.get().await {
                    Ok(c) => c,
                    Err(_) => return Outcome::Error(api_err(0, "Database error")),
                };
                match client.query_opt(
                    "SELECT u.id, u.username FROM api_keys ak JOIN users u ON ak.user_id=u.id WHERE ak.key_hash=$1",
                    &[&key_hash],
                ).await {
                    Ok(Some(row)) => {
                        let _ = client.execute("UPDATE api_keys SET last_used_at=NOW() WHERE key_hash=$1", &[&key_hash]).await;
                        return Outcome::Success(SubsonicUser { id: row.get("id"), username: row.get("username") });
                    }
                    Ok(None) => return Outcome::Error(api_err(44, "Invalid API key")),
                    Err(_) => return Outcome::Error(api_err(0, "Database error")),
                }
            }
        }

        // 2. Check conflicting auth
        let has_u = query_param(request, "u").is_some();
        let has_p = query_param(request, "p").is_some();
        let has_t = query_param(request, "t").is_some();
        let has_s = query_param(request, "s").is_some();
        if (has_t || has_s) && (has_u || has_p) {
            return Outcome::Error(api_err(
                43,
                "Multiple conflicting authentication mechanisms",
            ));
        }

        // 3. t+s (not supported with argon2)
        if has_t || has_s {
            return Outcome::Error(api_err(
                41,
                "Token authentication not supported. Use an API key.",
            ));
        }

        // 4. u+p
        let username = match query_param(request, "u") {
            Some(u) if !u.is_empty() => u,
            _ => return Outcome::Error(api_err(10, "Required parameter 'u' is missing")),
        };
        let password = match query_param(request, "p") {
            Some(p) if !p.is_empty() => p,
            _ => return Outcome::Error(api_err(10, "Required parameter 'p' is missing")),
        };
        let password = if let Some(hex) = password.strip_prefix("enc:") {
            match hex::decode(hex) {
                Ok(b) => String::from_utf8_lossy(&b).to_string(),
                Err(_) => return Outcome::Error(api_err(0, "Invalid hex-encoded password")),
            }
        } else {
            password
        };

        let client = match pool.get().await {
            Ok(c) => c,
            Err(_) => return Outcome::Error(api_err(0, "Database error")),
        };
        let row = match client
            .query_opt(
                "SELECT id,username,password_hash FROM users WHERE username=$1",
                &[&username],
            )
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => return Outcome::Error(api_err(40, "Wrong username or password")),
            Err(_) => return Outcome::Error(api_err(0, "Database error")),
        };
        let password_hash: String = row.get("password_hash");
        let parsed = match argon2::PasswordHash::new(&password_hash) {
            Ok(h) => h,
            Err(_) => return Outcome::Error(api_err(0, "Invalid password hash")),
        };
        if !argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
        {
            return Outcome::Error(api_err(40, "Wrong username or password"));
        }
        Outcome::Success(SubsonicUser {
            id: row.get("id"),
            username: row.get("username"),
        })
    }
}

// ── Endpoints ──

#[get("/rest/ping")]
pub(crate) fn ping(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(serde_json::to_value(&SubsonicResponse::<EmptyResponse>::ok_empty()).unwrap_or_default())
}

#[get("/rest/getLicense")]
pub(crate) fn get_license(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(&SubsonicResponse::ok(LicenseResponse { valid: true }))
            .unwrap_or_default(),
    )
}

#[get("/rest/getOpenSubsonicExtensions")]
pub(crate) fn get_open_subsonic_extensions(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(&SubsonicResponse::ok(ExtensionsResponse {
            extensions: vec![
                ExtensionInfo {
                    name: "formPost".into(),
                    versions: vec![1, 2],
                },
                ExtensionInfo {
                    name: "apiKeyAuth".into(),
                    versions: vec![1],
                },
                ExtensionInfo {
                    name: "songTitle".into(),
                    versions: vec![1],
                },
            ],
        }))
        .unwrap_or_default(),
    )
}

// ── Phase 4: Browsing endpoints ──

/// Encodes an artist name as a safe ID string.
fn artist_id(name: &str) -> String {
    format!("ar:{}", name)
}

/// Encodes artist+album as a safe album ID string.
fn album_id(artist: &str, album: &str) -> String {
    format!("al:{}|{}", artist, album)
}

/// Builds a SongEntry from a DB row.
fn row_to_song_entry(row: &tokio_postgres::Row) -> SongEntry {
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

fn format_to_mime(fmt: &str) -> &str {
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

#[get("/rest/getMusicFolders")]
pub(crate) fn get_music_folders(_user: SubsonicUser) -> Json<serde_json::Value> {
    let resp = SubsonicResponse::ok(MusicFoldersResponse {
        music_folders: MusicFolderList {
            music_folder: vec![MusicFolder {
                id: 1,
                name: "Personal Library".into(),
            }],
        },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[get("/rest/getIndexes")]
pub(crate) async fn get_indexes(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let rows = match client
        .query(
            "SELECT DISTINCT UPPER(LEFT(s.artist, 1)) AS letter FROM songs s \
         JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 ORDER BY letter",
            &[&user.id],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let mut indexes = Vec::new();
    for letter_row in &rows {
        let letter: String = letter_row.get("letter");
        let artists_rows = match client
            .query(
                "SELECT DISTINCT s.artist FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND UPPER(LEFT(s.artist, 1)) = $2 ORDER BY s.artist",
                &[&user.id, &letter],
            )
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };
        let artists: Vec<ArtistRef> = artists_rows
            .iter()
            .map(|r| {
                let name: String = r.get("artist");
                ArtistRef {
                    id: artist_id(&name),
                    name,
                }
            })
            .collect();
        if !artists.is_empty() {
            indexes.push(IndexEntry {
                name: letter,
                artist: artists,
            });
        }
    }

    Json(
        serde_json::to_value(&SubsonicResponse::ok(IndexesResponse {
            indexes: IndexList { index: indexes },
        }))
        .unwrap_or_default(),
    )
}

#[get("/rest/getMusicDirectory?<id>")]
pub(crate) async fn get_music_directory(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let (children, dir_name, dir_parent) = if id.is_empty() || id == "0" {
        // Root: list distinct artists
        let rows = match client.query(
            "SELECT DISTINCT s.artist FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 ORDER BY s.artist",
            &[&user.id],
        ).await {
            Ok(r) => r,
            Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
        };
        let children: Vec<ChildEntry> = rows
            .iter()
            .map(|r| {
                let name: String = r.get("artist");
                ChildEntry::Dir {
                    id: artist_id(&name),
                    parent: "0".into(),
                    title: name.clone(),
                    artist: name,
                    cover_art: None,
                    is_dir: true,
                }
            })
            .collect();
        (children, "root".to_string(), None)
    } else if let Some(artist_name) = id.strip_prefix("ar:") {
        // Artist: list albums
        let rows = match client.query(
            "SELECT DISTINCT s.album, s.artist, s.year, s.genre FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 AND s.artist = $2 ORDER BY s.album",
            &[&user.id, &artist_name],
        ).await {
            Ok(r) => r,
            Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
        };
        let children: Vec<ChildEntry> = rows
            .iter()
            .map(|r| {
                let album: String = r.get("album");
                let art: String = r.get("artist");
                ChildEntry::Dir {
                    id: album_id(&art, &album),
                    parent: id.clone(),
                    title: album.clone(),
                    artist: art.clone(),
                    cover_art: None,
                    is_dir: true,
                }
            })
            .collect();
        (children, artist_name.to_string(), Some("0".to_string()))
    } else if let Some(rest) = id.strip_prefix("al:") {
        // Album: list songs
        if let Some((artist_name, album_name)) = rest.split_once('|') {
            let rows = match client.query(
                "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 AND s.artist = $2 AND s.album = $3 ORDER BY s.track_number, s.title",
                &[&user.id, &artist_name, &album_name],
            ).await {
                Ok(r) => r,
                Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
            };
            let children: Vec<ChildEntry> = rows
                .iter()
                .map(|r| {
                    let s = row_to_song_entry(&r);
                    ChildEntry::Song {
                        id: s.id,
                        parent: id.clone(),
                        title: s.title,
                        artist: s.artist,
                        album: s.album,
                        track: s.track,
                        year: s.year,
                        genre: s.genre,
                        cover_art: s.cover_art,
                        duration: s.duration,
                        bitrate: s.bitrate,
                        size: s.size,
                        content_type: s.content_type,
                        is_dir: false,
                        is_video: false,
                    }
                })
                .collect();
            (
                children,
                album_name.to_string(),
                Some(artist_id(artist_name)),
            )
        } else {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    } else {
        return Json(
            serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                .unwrap_or_default(),
        );
    };

    let dir = Directory {
        id: id.clone(),
        name: dir_name,
        parent: dir_parent,
        child: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    };
    Json(
        serde_json::to_value(&SubsonicResponse::ok(DirectoryResponse { directory: dir }))
            .unwrap_or_default(),
    )
}

#[get("/rest/getArtists")]
pub(crate) async fn get_artists(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let rows = match client.query(
        "SELECT DISTINCT s.artist FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 ORDER BY s.artist",
        &[&user.id],
    ).await {
        Ok(r) => r,
        Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
    };

    // Group by first letter
    let mut index_map: std::collections::BTreeMap<String, Vec<ArtistRef>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        let name: String = row.get("artist");
        let letter = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "#".into());
        index_map.entry(letter).or_default().push(ArtistRef {
            id: artist_id(&name),
            name: name.clone(),
        });
    }

    let indexes: Vec<IndexEntry> = index_map
        .into_iter()
        .map(|(name, artist)| IndexEntry { name, artist })
        .collect();
    let resp = SubsonicResponse::ok(ArtistsResponse {
        artists: ArtistList { index: indexes },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[get("/rest/getArtist?<id>")]
pub(crate) async fn get_artist(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let artist_name = match id.strip_prefix("ar:") {
        Some(n) => n.to_string(),
        None => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let album_rows = match client.query(
        "SELECT DISTINCT s.album, s.artist, s.year, s.genre, COUNT(*) as song_count, SUM(s.duration_seconds) as total_dur, BOOL_OR(s.has_cover_art) as has_cover \
         FROM songs s JOIN user_songs us ON s.id = us.song_id \
         WHERE us.user_id = $1 AND s.artist = $2 GROUP BY s.album, s.artist, s.year, s.genre ORDER BY s.year, s.album",
        &[&user.id, &artist_name],
    ).await {
        Ok(r) => r,
        Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
    };

    let albums: Vec<AlbumRef> = album_rows
        .iter()
        .map(|r| {
            let album: String = r.get("album");
            let art: String = r.get("artist");
            let has_cover: bool = r.get("has_cover");
            let song_count: i64 = r.get("song_count");
            let total_dur: Option<f64> = r.get("total_dur");
            AlbumRef {
                id: album_id(&art, &album),
                name: album,
                artist: art,
                year: r.try_get("year").ok(),
                genre: r.try_get("genre").ok(),
                cover_art: if has_cover {
                    Some(album_id(
                        &r.get::<_, String>("artist"),
                        &r.get::<_, String>("album"),
                    ))
                } else {
                    None
                },
                song_count,
                duration: total_dur,
            }
        })
        .collect();

    let resp = SubsonicResponse::ok(ArtistResponse {
        artist: ArtistDetail {
            id: artist_id(&artist_name),
            name: artist_name.clone(),
            album: if albums.is_empty() {
                None
            } else {
                Some(albums)
            },
        },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[get("/rest/getAlbum?<id>")]
pub(crate) async fn get_album(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let (artist_name, album_name) = match id.strip_prefix("al:") {
        Some(rest) => match rest.split_once('|') {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => {
                return Json(
                    serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                        .unwrap_or_default(),
                );
            }
        },
        None => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let song_rows = match client
        .query(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
         WHERE us.user_id = $1 AND s.artist = $2 AND s.album = $3 ORDER BY s.track_number, s.title",
            &[&user.id, &artist_name, &album_name],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let songs: Vec<SongEntry> = song_rows.iter().map(|r| row_to_song_entry(&r)).collect();
    let total_dur: f64 = song_rows
        .iter()
        .filter_map(|r| r.try_get::<_, f64>("duration_seconds").ok())
        .sum();
    let has_cover: bool = song_rows
        .iter()
        .any(|r| r.try_get::<_, bool>("has_cover_art").unwrap_or(false));
    let year = song_rows
        .iter()
        .find_map(|r| r.try_get::<_, i16>("year").ok());
    let genre: Option<String> = song_rows.iter().find_map(|r| r.try_get("genre").ok());

    let song_count = songs.len() as i64;
    let resp = SubsonicResponse::ok(AlbumResponse {
        album: AlbumDetail {
            id: id.clone(),
            name: album_name,
            artist: artist_name,
            year,
            genre,
            cover_art: if has_cover { Some(id.clone()) } else { None },
            song_count,
            duration: if total_dur > 0.0 {
                Some(total_dur)
            } else {
                None
            },
            song: if songs.is_empty() { None } else { Some(songs) },
        },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[get("/rest/getSong?<id>")]
pub(crate) async fn get_song(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let song_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let row = match client.query_opt(
        "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 AND s.id = $2",
        &[&user.id, &song_uuid],
    ).await {
        Ok(Some(r)) => r,
        Ok(None) => return Json(serde_json::to_value(&SubsonicResponse::error(70, "Resource not found")).unwrap_or_default()),
        Err(_) => return Json(serde_json::to_value(&SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
    };
    let song = row_to_song_entry(&row);
    Json(serde_json::to_value(&SubsonicResponse::ok(SongResponse { song })).unwrap_or_default())
}
#[allow(non_snake_case)]
#[get("/rest/getAlbumList2?<type>&<size>&<offset>&<fromYear>&<toYear>&<genre>")]
pub(crate) async fn get_album_list2(
    pool: &State<Pool>,
    user: SubsonicUser,
    r#type: String,
    size: Option<usize>,
    offset: Option<usize>,
    fromYear: Option<i32>,
    toYear: Option<i32>,
    genre: Option<String>,
) -> Json<serde_json::Value> {
    let limit = size.unwrap_or(10).min(500);
    let skip = offset.unwrap_or(0);

    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let rows = match r#type.as_str() {
        "newest" => client.query(
            "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
             COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
             BOOL_OR(s.has_cover_art) OVER w AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
             WINDOW w AS (PARTITION BY s.artist, s.album) \
             ORDER BY s.artist, s.album, s.created_at DESC \
             OFFSET $2 LIMIT $3",
            &[&user.id, &(skip as i64), &(limit as i64)],
        ).await,
        "alphabeticalByName" => client.query(
            "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
             COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
             BOOL_OR(s.has_cover_art) OVER w AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
             WINDOW w AS (PARTITION BY s.artist, s.album) \
             ORDER BY s.artist, s.album, s.album \
             OFFSET $2 LIMIT $3",
            &[&user.id, &(skip as i64), &(limit as i64)],
        ).await,
        "alphabeticalByArtist" => client.query(
            "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
             COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
             BOOL_OR(s.has_cover_art) OVER w AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
             WINDOW w AS (PARTITION BY s.artist, s.album) \
             ORDER BY s.artist, s.album \
             OFFSET $2 LIMIT $3",
            &[&user.id, &(skip as i64), &(limit as i64)],
        ).await,
        "byYear" => {
            let fy = fromYear.unwrap_or(0);
            let ty = toYear.unwrap_or(9999);
            client.query(
                "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
                 COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
                 BOOL_OR(s.has_cover_art) OVER w AS has_cover \
                 FROM songs s JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 AND s.year BETWEEN $2 AND $3 \
                 WINDOW w AS (PARTITION BY s.artist, s.album) \
                 ORDER BY s.year DESC, s.artist, s.album \
                 OFFSET $4 LIMIT $5",
                &[&user.id, &fy, &ty, &(skip as i64), &(limit as i64)],
            ).await
        },
        "byGenre" => {
            if let Some(g) = &genre {
                client.query(
                    "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
                     COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
                     BOOL_OR(s.has_cover_art) OVER w AS has_cover \
                     FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.genre = $2 \
                     WINDOW w AS (PARTITION BY s.artist, s.album) \
                     ORDER BY s.artist, s.album \
                     OFFSET $3 LIMIT $4",
                    &[&user.id, &g, &(skip as i64), &(limit as i64)],
                ).await
            } else {
                return Json(serde_json::to_value(&SubsonicResponse::error(10, "Genre parameter required for byGenre list")).unwrap_or_default());
            }
        },
        "random" => client.query(
            "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
             COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
             BOOL_OR(s.has_cover_art) OVER w AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
             WINDOW w AS (PARTITION BY s.artist, s.album) \
             ORDER BY RANDOM() \
             OFFSET $2 LIMIT $3",
            &[&user.id, &(skip as i64), &(limit as i64)],
        ).await,
        "frequent" => client.query(
            "SELECT DISTINCT ON (s.artist, s.album) s.artist, s.album, s.year, s.genre, \
             COUNT(*) OVER w AS song_count, SUM(s.duration_seconds) OVER w AS total_dur, \
             BOOL_OR(s.has_cover_art) OVER w AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
             WINDOW w AS (PARTITION BY s.artist, s.album) \
             ORDER BY (SELECT COUNT(*) FROM scrobbles sc WHERE sc.song_id = s.id AND sc.user_id = $1) DESC, s.artist, s.album \
             OFFSET $2 LIMIT $3",
            &[&user.id, &(skip as i64), &(limit as i64)],
        ).await,
        _ => return Json(serde_json::to_value(&SubsonicResponse::error(10, "Invalid list type")).unwrap_or_default()),
    };

    let rows = match rows {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let albums: Vec<AlbumRef> = rows
        .iter()
        .map(|r| {
            let album: String = r.get("album");
            let art: String = r.get("artist");
            let has_cover: bool = r.get("has_cover");
            AlbumRef {
                id: album_id(&art, &album),
                name: album,
                artist: art,
                year: r.try_get("year").ok(),
                genre: r.try_get("genre").ok(),
                cover_art: if has_cover {
                    Some(album_id(
                        &r.get::<_, String>("artist"),
                        &r.get::<_, String>("album"),
                    ))
                } else {
                    None
                },
                song_count: r.get("song_count"),
                duration: r.try_get("total_dur").ok(),
            }
        })
        .collect();

    let resp = SubsonicResponse::ok(AlbumListResponse {
        album_list: AlbumListContainer { album: albums },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[allow(non_snake_case)]
#[get("/rest/getAlbumList?<type>&<size>&<offset>&<fromYear>&<toYear>&<genre>")]
pub(crate) async fn get_album_list(
    pool: &State<Pool>,
    user: SubsonicUser,
    r#type: String,
    size: Option<usize>,
    offset: Option<usize>,
    fromYear: Option<i32>,
    toYear: Option<i32>,
    genre: Option<String>,
) -> Json<serde_json::Value> {
    get_album_list2(pool, user, r#type, size, offset, fromYear, toYear, genre).await
}

#[get("/rest/getGenres")]
pub(crate) async fn get_genres(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let rows = match client
        .query(
            "SELECT s.genre, COUNT(*) AS song_count, COUNT(DISTINCT s.album) AS album_count \
         FROM songs s JOIN user_songs us ON s.id = us.song_id \
         WHERE us.user_id = $1 AND s.genre IS NOT NULL AND s.genre != '' \
         GROUP BY s.genre ORDER BY s.genre",
            &[&user.id],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let genres: Vec<GenreItem> = rows
        .iter()
        .map(|r| GenreItem {
            name: r.get("genre"),
            song_count: r.get("song_count"),
            album_count: r.get("album_count"),
        })
        .collect();

    Json(
        serde_json::to_value(SubsonicResponse::ok(GenresResponse {
            genres: GenreList { genre: genres },
        }))
        .unwrap_or_default(),
    )
}

// ── Phase 5: Media & Search endpoints ──

/// Parse an HTTP Range header value. Returns (start, end_inclusive) in bytes.
fn parse_range_header(range_header: &str, file_size: u64) -> Option<(u64, u64)> {
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
async fn get_user_song(
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RangeHeader {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(RangeHeader(
            req.headers().get_one("Range").map(|s| s.to_string()),
        ))
    }
}

#[allow(non_snake_case)]
#[get("/rest/stream?<id>&<_maxBitRate>&<_format>&<_timeOffset>&<_size>&<_estimateContentLength>")]
pub(crate) async fn stream(
    pool: &State<Pool>,
    range_hdr: RangeHeader,
    user: SubsonicUser,
    id: String,
    _maxBitRate: Option<i32>,
    _format: Option<String>,
    _timeOffset: Option<i32>,
    _size: Option<String>,
    _estimateContentLength: Option<bool>,
) -> SubsonicBinaryResponse {
    let song_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return SubsonicBinaryResponse::error(70, "Resource not found"),
    };

    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => return SubsonicBinaryResponse::error(0, "Database error"),
    };

    let row = match get_user_song(&client, user.id, song_uuid).await {
        Ok(r) => r,
        Err(_) => return SubsonicBinaryResponse::error(70, "Resource not found"),
    };

    let file_path: String = row.get("file_path");
    let format_str: String = row.get("format");
    let size_bytes: i64 = row.get("size_bytes");
    let file_size = size_bytes as u64;
    let content_type = format_to_mime(&format_str).to_string();

    let full_path = Path::new(MUSIC_ROOT).join(&file_path);
    let mut file = match File::open(&full_path).await {
        Ok(f) => f,
        Err(_) => return SubsonicBinaryResponse::error(70, "File not found on disk"),
    };

    let range = range_hdr
        .0
        .as_deref()
        .and_then(|r| parse_range_header(r, file_size));

    let (status, content_length, content_range) = match range {
        Some((start, end)) => {
            if file.seek(SeekFrom::Start(start)).await.is_err() {
                return SubsonicBinaryResponse::error(0, "Seek error");
            }
            let length = end - start + 1;
            (
                Status::PartialContent,
                length,
                Some(format!("bytes {}-{}/{}", start, end, file_size)),
            )
        }
        None => (Status::Ok, file_size, None),
    };

    let limited = file.take(content_length);

    let mut extra_headers = Vec::new();
    if let Some(cr) = content_range {
        extra_headers.push(("Content-Range".into(), cr));
    }

    SubsonicBinaryResponse::Stream(SubsonicStreamResponse {
        reader: Box::new(limited),
        content_type,
        content_length,
        status,
        extra_headers,
    })
}
#[get("/rest/download?<id>")]
pub(crate) async fn subsonic_download(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> SubsonicBinaryResponse {
    let song_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return SubsonicBinaryResponse::error(70, "Resource not found"),
    };

    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => return SubsonicBinaryResponse::error(0, "Database error"),
    };

    let row = match get_user_song(&client, user.id, song_uuid).await {
        Ok(r) => r,
        Err(_) => return SubsonicBinaryResponse::error(70, "Resource not found"),
    };

    let file_path: String = row.get("file_path");
    let format_str: String = row.get("format");
    let artist: String = row.get("artist");
    let title: String = row.get("title");
    let size_bytes: i64 = row.get("size_bytes");
    let content_type = format_to_mime(&format_str).to_string();

    let full_path = Path::new(MUSIC_ROOT).join(&file_path);
    let file = match File::open(&full_path).await {
        Ok(f) => f,
        Err(_) => return SubsonicBinaryResponse::error(70, "File not found on disk"),
    };

    let ext = match format_str.as_str() {
        "m4a" | "aac" | "mp4" => "m4a",
        other => other,
    };
    let filename = format!("{} - {}.{}", artist, title, ext);

    SubsonicBinaryResponse::Stream(SubsonicStreamResponse {
        reader: Box::new(file),
        content_type,
        content_length: size_bytes as u64,
        status: Status::Ok,
        extra_headers: vec![(
            "Content-Disposition".into(),
            format!("attachment; filename=\"{}\"", filename),
        )],
    })
}

#[get("/rest/getCoverArt?<id>&<_size>")]
pub(crate) async fn get_cover_art(
    pool: &State<Pool>,
    _user: SubsonicUser,
    id: String,
    _size: Option<i32>,
) -> SubsonicBinaryResponse {
    let song_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return SubsonicBinaryResponse::error(70, "Resource not found"),
    };

    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => return SubsonicBinaryResponse::error(0, "Database error"),
    };

    let row = match client
        .query_opt(
            "SELECT ca.file_path, ca.mime_type FROM cover_art ca WHERE ca.song_id = $1",
            &[&song_uuid],
        )
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return SubsonicBinaryResponse::error(70, "Cover art not found"),
        Err(_) => return SubsonicBinaryResponse::error(0, "Database error"),
    };

    let rel_path: String = row.get("file_path");
    let mime_type: String = row.get("mime_type");

    let full_path = Path::new(MUSIC_ROOT).join(".covers").join(&rel_path);
    let file = match File::open(&full_path).await {
        Ok(f) => f,
        Err(_) => return SubsonicBinaryResponse::error(70, "Cover art file not found"),
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => return SubsonicBinaryResponse::error(0, "Failed to read cover art"),
    };
    let file_size = metadata.len();

    SubsonicBinaryResponse::Stream(SubsonicStreamResponse {
        reader: Box::new(file),
        content_type: mime_type,
        content_length: file_size,
        status: Status::Ok,
        extra_headers: Vec::new(),
    })
}

// ── Phase 5: Binary response helpers ──

/// Response type for serving audio/image binary data via Subsonic endpoints.
/// Follows the same pattern as shared::FileResponse.
pub(crate) struct SubsonicStreamResponse {
    pub reader: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
    pub content_type: String,
    pub content_length: u64,
    pub status: Status,
    pub extra_headers: Vec<(String, String)>,
}

impl<'r> rocket::response::Responder<'r, 'static> for SubsonicStreamResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut resp = rocket::Response::build();
        resp.status(self.status)
            .header(Header::new("Accept-Ranges", "bytes"))
            .header(Header::new("Content-Type", self.content_type))
            .header(Header::new(
                "Content-Length",
                self.content_length.to_string(),
            ));
        for (k, v) in &self.extra_headers {
            resp.header(Header::new(k.clone(), v.clone()));
        }
        resp.streamed_body(self.reader).ok()
    }
}

/// Unified return type for binary Subsonic endpoints (stream, download, getCoverArt).
pub(crate) enum SubsonicBinaryResponse {
    Stream(SubsonicStreamResponse),
    Error(String),
}

impl SubsonicBinaryResponse {
    pub fn error(code: i32, msg: &str) -> Self {
        let resp = SubsonicResponse::<EmptyResponse>::error(code, msg);
        SubsonicBinaryResponse::Error(serde_json::to_string(&resp).unwrap_or_default())
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for SubsonicBinaryResponse {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        match self {
            SubsonicBinaryResponse::Stream(s) => s.respond_to(request),
            SubsonicBinaryResponse::Error(json) => rocket::Response::build()
                .status(Status::Ok)
                .header(ContentType::JSON)
                .sized_body(json.len(), std::io::Cursor::new(json))
                .ok(),
        }
    }
}

#[allow(non_snake_case)]
#[get(
    "/rest/search2?<query>&<artistCount>&<artistOffset>&<albumCount>&<albumOffset>&<songCount>&<songOffset>&<_musicFolderId>"
)]
pub(crate) async fn search2(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: String,
    artistCount: Option<i32>,
    artistOffset: Option<i32>,
    albumCount: Option<i32>,
    albumOffset: Option<i32>,
    songCount: Option<i32>,
    songOffset: Option<i32>,
    _musicFolderId: Option<String>,
) -> Json<serde_json::Value> {
    search3(
        pool,
        user,
        query,
        artistCount,
        artistOffset,
        albumCount,
        albumOffset,
        songCount,
        songOffset,
        _musicFolderId,
    )
    .await
}

#[allow(non_snake_case)]
#[get(
    "/rest/search3?<query>&<artistCount>&<artistOffset>&<albumCount>&<albumOffset>&<songCount>&<songOffset>&<_musicFolderId>"
)]
pub(crate) async fn search3(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: String,
    artistCount: Option<i32>,
    artistOffset: Option<i32>,
    albumCount: Option<i32>,
    albumOffset: Option<i32>,
    songCount: Option<i32>,
    songOffset: Option<i32>,
    _musicFolderId: Option<String>,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::<EmptyResponse>::error(
                    0,
                    "Database error",
                ))
                .unwrap_or_default(),
            );
        }
    };

    let art_count = artistCount.unwrap_or(20).min(500) as i64;
    let art_skip = artistOffset.unwrap_or(0) as i64;
    let alb_count = albumCount.unwrap_or(20).min(500) as i64;
    let alb_skip = albumOffset.unwrap_or(0) as i64;
    let sng_count = songCount.unwrap_or(20).min(500) as i64;
    let sng_skip = songOffset.unwrap_or(0) as i64;

    let pattern = format!("%{}%", query);

    let artists = match client
        .query(
            "SELECT DISTINCT s.artist FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.artist ILIKE $2 \
             ORDER BY s.artist OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &art_skip, &art_count],
        )
        .await
    {
        Ok(rows) => Some(
            rows.iter()
                .map(|r| {
                    let name: String = r.get("artist");
                    ArtistRef {
                        id: artist_id(&name),
                        name,
                    }
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => None,
    };

    let albums = match client
        .query(
            "SELECT DISTINCT s.artist, s.album, s.year, s.genre, BOOL_OR(s.has_cover_art) AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.album ILIKE $2 \
             GROUP BY s.artist, s.album, s.year, s.genre \
             ORDER BY s.artist, s.album OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &alb_skip, &alb_count],
        )
        .await
    {
        Ok(rows) => Some(
            rows.iter()
                .map(|r| {
                    let art: String = r.get("artist");
                    let alb: String = r.get("album");
                    let has_cover: bool = r.get("has_cover");
                    AlbumRef {
                        id: album_id(&art, &alb),
                        name: alb.clone(),
                        artist: art.clone(),
                        year: r.try_get("year").ok(),
                        genre: r.try_get("genre").ok(),
                        cover_art: if has_cover {
                            Some(album_id(&art, &alb))
                        } else {
                            None
                        },
                        song_count: 0,
                        duration: None,
                    }
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => None,
    };

    let songs = match client
        .query(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND (s.title ILIKE $2 OR s.artist ILIKE $2 OR s.album ILIKE $2) \
             ORDER BY s.title OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &sng_skip, &sng_count],
        )
        .await
    {
        Ok(rows) => Some(rows.iter().map(row_to_song_entry).collect::<Vec<_>>()),
        Err(_) => None,
    };

    let resp = SubsonicResponse::ok(SearchResponse {
        search_result: SearchResult {
            artists,
            albums,
            songs,
        },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

#[allow(non_snake_case)]
#[get("/rest/getRandomSongs?<size>&<genre>&<fromYear>&<toYear>&<_musicFolderId>")]
pub(crate) async fn get_random_songs(
    pool: &State<Pool>,
    user: SubsonicUser,
    size: Option<i32>,
    genre: Option<String>,
    fromYear: Option<i32>,
    toYear: Option<i32>,
    _musicFolderId: Option<String>,
) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::<EmptyResponse>::error(
                    0,
                    "Database error",
                ))
                .unwrap_or_default(),
            );
        }
    };

    let count = size.unwrap_or(10).min(500) as i64;

    let rows_result = match (&genre, fromYear, toYear) {
        (Some(g), Some(fy), Some(ty)) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.genre = $2 AND s.year BETWEEN $3 AND $4 \
                     ORDER BY RANDOM() LIMIT $5",
                    &[&user.id, g, &fy, &ty, &count],
                )
                .await
        }
        (Some(g), Some(fy), None) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.genre = $2 AND s.year >= $3 \
                     ORDER BY RANDOM() LIMIT $4",
                    &[&user.id, g, &fy, &count],
                )
                .await
        }
        (Some(g), None, Some(ty)) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.genre = $2 AND s.year <= $3 \
                     ORDER BY RANDOM() LIMIT $4",
                    &[&user.id, g, &ty, &count],
                )
                .await
        }
        (Some(g), None, None) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.genre = $2 \
                     ORDER BY RANDOM() LIMIT $3",
                    &[&user.id, g, &count],
                )
                .await
        }
        (None, Some(fy), Some(ty)) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.year BETWEEN $2 AND $3 \
                     ORDER BY RANDOM() LIMIT $4",
                    &[&user.id, &fy, &ty, &count],
                )
                .await
        }
        (None, Some(fy), None) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.year >= $2 \
                     ORDER BY RANDOM() LIMIT $3",
                    &[&user.id, &fy, &count],
                )
                .await
        }
        (None, None, Some(ty)) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.year <= $2 \
                     ORDER BY RANDOM() LIMIT $3",
                    &[&user.id, &ty, &count],
                )
                .await
        }
        (None, None, None) => {
            client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 \
                     ORDER BY RANDOM() LIMIT $2",
                    &[&user.id, &count],
                )
                .await
        }
    };

    let rows = match rows_result {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(&SubsonicResponse::<EmptyResponse>::error(
                    0,
                    "Database error",
                ))
                .unwrap_or_default(),
            );
        }
    };

    let songs: Vec<SongEntry> = rows.iter().map(row_to_song_entry).collect();

    let resp = SubsonicResponse::ok(RandomSongsResponse {
        randomSongs: RandomSongsList { song: songs },
    });
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

// ── URL decode ──

fn url_decode(s: &str) -> String {
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
