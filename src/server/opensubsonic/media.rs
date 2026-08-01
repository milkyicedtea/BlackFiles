use super::*;

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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RangeHeader {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(RangeHeader(
            req.headers().get_one("Range").map(|s| s.to_string()),
        ))
    }
}

async fn binary_context(
    pool: &Pool,
    id: &str,
) -> Result<(deadpool_postgres::Object, Uuid), SubsonicBinaryResponse> {
    let song_id =
        Uuid::parse_str(id).map_err(|_| SubsonicBinaryResponse::error(70, "Resource not found"))?;
    let client = pool
        .get()
        .await
        .map_err(|_| SubsonicBinaryResponse::error(0, "Database error"))?;
    Ok((client, song_id))
}

async fn binary_user_song(
    pool: &Pool,
    user_id: Uuid,
    id: &str,
) -> Result<tokio_postgres::Row, SubsonicBinaryResponse> {
    let (client, song_id) = binary_context(pool, id).await?;
    get_user_song(&client, user_id, song_id)
        .await
        .map_err(|_| SubsonicBinaryResponse::error(70, "Resource not found"))
}

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
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
    let row = match binary_user_song(pool, user.id, &id).await {
        Ok(row) => row,
        Err(response) => return response,
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
    let row = match binary_user_song(pool, user.id, &id).await {
        Ok(row) => row,
        Err(response) => return response,
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
    let (client, song_uuid) = match binary_context(pool, &id).await {
        Ok(context) => context,
        Err(response) => return response,
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

#[derive(FromForm)]
pub(crate) struct SearchQuery {
    query: String,
    #[field(name = "artistCount")]
    artist_count: Option<i32>,
    #[field(name = "artistOffset")]
    artist_offset: Option<i32>,
    #[field(name = "albumCount")]
    album_count: Option<i32>,
    #[field(name = "albumOffset")]
    album_offset: Option<i32>,
    #[field(name = "songCount")]
    song_count: Option<i32>,
    #[field(name = "songOffset")]
    song_offset: Option<i32>,
}

async fn search(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SearchQuery,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let artist_count = query.artist_count.unwrap_or(20).min(500) as i64;
    let artist_offset = query.artist_offset.unwrap_or(0) as i64;
    let album_count = query.album_count.unwrap_or(20).min(500) as i64;
    let album_offset = query.album_offset.unwrap_or(0) as i64;
    let song_count = query.song_count.unwrap_or(20).min(500) as i64;
    let song_offset = query.song_offset.unwrap_or(0) as i64;
    let pattern = format!("%{}%", query.query);

    let artists = client
        .query(
            "SELECT DISTINCT s.artist FROM songs s \
             JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.artist ILIKE $2 \
             ORDER BY s.artist OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &artist_offset, &artist_count],
        )
        .await
        .ok()
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let name: String = row.get("artist");
                    ArtistRef {
                        id: artist_id(&name),
                        name,
                    }
                })
                .collect()
        });
    let albums = client
        .query(
            "SELECT DISTINCT s.artist, s.album, s.year, s.genre, \
                    BOOL_OR(s.has_cover_art) AS has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.album ILIKE $2 \
             GROUP BY s.artist, s.album, s.year, s.genre \
             ORDER BY s.artist, s.album OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &album_offset, &album_count],
        )
        .await
        .ok()
        .map(|rows| {
            rows.iter()
                .map(|row| AlbumRef {
                    info: AlbumInfo::from_row(row, "artist", "album"),
                    song_count: 0,
                    duration: None,
                })
                .collect()
        });
    let songs = client
        .query(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
               AND (s.title ILIKE $2 OR s.artist ILIKE $2 OR s.album ILIKE $2) \
             ORDER BY s.title OFFSET $3 LIMIT $4",
            &[&user.id, &pattern, &song_offset, &song_count],
        )
        .await
        .ok()
        .map(|rows| rows.iter().map(row_to_song_entry).collect());

    ok_resp(SearchResponse {
        search_result: SearchResult {
            artists,
            albums,
            songs,
        },
    })
}

#[get("/rest/search2?<query..>")]
pub(crate) async fn search2(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SearchQuery,
) -> Json<serde_json::Value> {
    search(pool, user, query).await
}

#[get("/rest/search3?<query..>")]
pub(crate) async fn search3(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SearchQuery,
) -> Json<serde_json::Value> {
    search(pool, user, query).await
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
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let count = size.unwrap_or(10).min(500) as i64;
    let Ok(rows) = client
        .query(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 \
               AND ($2::TEXT IS NULL OR s.genre = $2) \
               AND ($3::INTEGER IS NULL OR s.year >= $3) \
               AND ($4::INTEGER IS NULL OR s.year <= $4) \
             ORDER BY RANDOM() LIMIT $5",
            &[&user.id, &genre, &fromYear, &toYear, &count],
        )
        .await
    else {
        return db_err_resp();
    };
    let songs = rows.iter().map(row_to_song_entry).collect();
    ok_resp(RandomSongsResponse {
        randomSongs: RandomSongsList { song: songs },
    })
}
