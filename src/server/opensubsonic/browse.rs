use super::*;

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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
        serde_json::to_value(SubsonicResponse::ok(IndexesResponse {
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
            Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
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
            Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
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
                Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
            };
            let children: Vec<ChildEntry> = rows
                .iter()
                .map(|r| {
                    let s = row_to_song_entry(r);
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
                serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    } else {
        return Json(
            serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
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
        serde_json::to_value(SubsonicResponse::ok(DirectoryResponse { directory: dir }))
            .unwrap_or_default(),
    )
}

#[get("/rest/getArtists")]
pub(crate) async fn get_artists(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return Json(
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let rows = match client.query(
        "SELECT DISTINCT s.artist FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 ORDER BY s.artist",
        &[&user.id],
    ).await {
        Ok(r) => r,
        Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let artist_name = match id.strip_prefix("ar:") {
        Some(n) => n.to_string(),
        None => {
            return Json(
                serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
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
        Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let (artist_name, album_name) = match id.strip_prefix("al:") {
        Some(rest) => match rest.split_once('|') {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => {
                return Json(
                    serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
                        .unwrap_or_default(),
                );
            }
        },
        None => {
            return Json(
                serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let songs: Vec<SongEntry> = song_rows.iter().map(row_to_song_entry).collect();
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
                    .unwrap_or_default(),
            );
        }
    };

    let song_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Json(
                serde_json::to_value(SubsonicResponse::error(70, "Resource not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let row = match client.query_opt(
        "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id WHERE us.user_id = $1 AND s.id = $2",
        &[&user.id, &song_uuid],
    ).await {
        Ok(Some(r)) => r,
        Ok(None) => return Json(serde_json::to_value(SubsonicResponse::error(70, "Resource not found")).unwrap_or_default()),
        Err(_) => return Json(serde_json::to_value(SubsonicResponse::error(0, "Database error")).unwrap_or_default()),
    };
    let song = row_to_song_entry(&row);
    Json(serde_json::to_value(SubsonicResponse::ok(SongResponse { song })).unwrap_or_default())
}
#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
                return Json(serde_json::to_value(SubsonicResponse::error(10, "Genre parameter required for byGenre list")).unwrap_or_default());
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
        _ => return Json(serde_json::to_value(SubsonicResponse::error(10, "Invalid list type")).unwrap_or_default()),
    };

    let rows = match rows {
        Ok(r) => r,
        Err(_) => {
            return Json(
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
#[allow(clippy::too_many_arguments)]
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
                serde_json::to_value(SubsonicResponse::error(0, "Database error"))
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
