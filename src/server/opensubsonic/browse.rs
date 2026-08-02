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
pub struct AlbumInfo {
    pub id: String,
    pub name: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
}

impl AlbumInfo {
    pub(super) fn from_row(
        row: &tokio_postgres::Row,
        artist_column: &str,
        album_column: &str,
    ) -> Self {
        let artist = row.get::<_, String>(artist_column);
        let name = row.get::<_, String>(album_column);
        let id = album_id(&artist, &name);
        let cover_art = row.get::<_, bool>("has_cover").then(|| id.clone());
        Self {
            id,
            name,
            artist,
            year: row.try_get("year").ok(),
            genre: row.try_get("genre").ok(),
            cover_art,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct AlbumRef {
    #[serde(flatten)]
    pub info: AlbumInfo,
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
    #[serde(flatten)]
    pub info: AlbumInfo,
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
    pub value: String,
    #[serde(rename = "songCount")]
    pub song_count: i64,
    #[serde(rename = "albumCount")]
    pub album_count: i64,
}

#[get("/getMusicFolders")]
pub(crate) fn get_music_folders(_user: SubsonicUser) -> Json<serde_json::Value> {
    ok_resp(MusicFoldersResponse {
        music_folders: MusicFolderList {
            music_folder: vec![MusicFolder {
                id: 1,
                name: "Personal Library".into(),
            }],
        },
    })
}

#[get("/getIndexes")]
pub(crate) async fn get_indexes(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Ok(rows) = client
        .query(
            "SELECT DISTINCT UPPER(LEFT(s.artist, 1)) AS letter FROM songs s \
             JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 ORDER BY letter",
            &[&user.id],
        )
        .await
    else {
        return db_err_resp();
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

    ok_resp(IndexesResponse {
        indexes: IndexList { index: indexes },
    })
}

#[get("/getMusicDirectory?<id>")]
pub(crate) async fn get_music_directory(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let (children, dir_name, dir_parent) = if id.is_empty() || id == "0" {
        // Root: list distinct artists
        let Ok(rows) = client
            .query(
                "SELECT DISTINCT s.artist FROM songs s \
                 JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 ORDER BY s.artist",
                &[&user.id],
            )
            .await
        else {
            return db_err_resp();
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
        let Ok(rows) = client
            .query(
                "SELECT DISTINCT s.album, s.artist, s.year, s.genre FROM songs s \
                 JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 AND s.artist = $2 ORDER BY s.album",
                &[&user.id, &artist_name],
            )
            .await
        else {
            return db_err_resp();
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
            let Ok(rows) = client
                .query(
                    "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
                     WHERE us.user_id = $1 AND s.artist = $2 AND s.album = $3 \
                     ORDER BY s.track_number, s.title",
                    &[&user.id, &artist_name, &album_name],
                )
                .await
            else {
                return db_err_resp();
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
            return not_found_resp();
        }
    } else {
        return not_found_resp();
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
    ok_resp(DirectoryResponse { directory: dir })
}

#[get("/getArtists")]
pub(crate) async fn get_artists(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Ok(rows) = client
        .query(
            "SELECT DISTINCT s.artist FROM songs s \
             JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 ORDER BY s.artist",
            &[&user.id],
        )
        .await
    else {
        return db_err_resp();
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
    ok_resp(ArtistsResponse {
        artists: ArtistList { index: indexes },
    })
}

#[get("/getArtist?<id>")]
pub(crate) async fn get_artist(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Some(artist_name) = decode_artist_id(&id) else {
        return not_found_resp();
    };

    let Ok(album_rows) = client
        .query(
            "SELECT DISTINCT s.album, s.artist, s.year, s.genre, \
                    COUNT(*) as song_count, SUM(s.duration_seconds) as total_dur, \
                    BOOL_OR(s.has_cover_art) as has_cover \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.artist = $2 \
             GROUP BY s.album, s.artist, s.year, s.genre ORDER BY s.year, s.album",
            &[&user.id, &artist_name],
        )
        .await
    else {
        return db_err_resp();
    };

    let albums: Vec<AlbumRef> = album_rows
        .iter()
        .map(|r| AlbumRef {
            info: AlbumInfo::from_row(r, "artist", "album"),
            song_count: r.get("song_count"),
            duration: r.get("total_dur"),
        })
        .collect();

    ok_resp(ArtistResponse {
        artist: ArtistDetail {
            id: artist_id(&artist_name),
            name: artist_name,
            album: (!albums.is_empty()).then_some(albums),
        },
    })
}

#[get("/getAlbum?<id>")]
pub(crate) async fn get_album(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Some((artist_name, album_name)) = decode_album_id(&id) else {
        return not_found_resp();
    };

    let Ok(song_rows) = client
        .query(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.artist = $2 AND s.album = $3 \
             ORDER BY s.track_number, s.title",
            &[&user.id, &artist_name, &album_name],
        )
        .await
    else {
        return db_err_resp();
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
    ok_resp(AlbumResponse {
        album: AlbumDetail {
            info: AlbumInfo {
                id: id.clone(),
                name: album_name,
                artist: artist_name,
                year,
                genre,
                cover_art: has_cover.then(|| id.clone()),
            },
            song_count,
            duration: (total_dur > 0.0).then_some(total_dur),
            song: (!songs.is_empty()).then_some(songs),
        },
    })
}

#[get("/getSong?<id>")]
pub(crate) async fn get_song(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Ok(song_uuid) = Uuid::parse_str(&id) else {
        return not_found_resp();
    };

    let row = match client
        .query_opt(
            "SELECT s.* FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.id = $2",
            &[&user.id, &song_uuid],
        )
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return not_found_resp(),
        Err(_) => return db_err_resp(),
    };
    ok_resp(SongResponse {
        song: row_to_song_entry(&row),
    })
}
#[derive(FromForm)]
pub(crate) struct AlbumListQuery {
    #[field(name = "type")]
    kind: String,
    size: Option<usize>,
    offset: Option<usize>,
    #[field(name = "fromYear")]
    from_year: Option<i32>,
    #[field(name = "toYear")]
    to_year: Option<i32>,
    genre: Option<String>,
}

async fn album_list(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: AlbumListQuery,
) -> Json<serde_json::Value> {
    let limit = query.size.unwrap_or(10).min(500) as i64;
    let skip = query.offset.unwrap_or(0) as i64;
    let from_year = query.from_year.unwrap_or(0);
    let to_year = query.to_year.unwrap_or(9999);
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&user.id];
    let (filter, order) = match query.kind.as_str() {
        "newest" => ("", "s.artist, s.album, s.created_at DESC"),
        "alphabeticalByName" => ("", "s.artist, s.album, s.album"),
        "alphabeticalByArtist" => ("", "s.artist, s.album"),
        "byYear" => {
            params.push(&from_year);
            params.push(&to_year);
            (
                "AND s.year BETWEEN $2 AND $3",
                "s.year DESC, s.artist, s.album",
            )
        }
        "byGenre" => {
            let Some(genre) = query.genre.as_ref() else {
                return param_err("Genre parameter required for byGenre list");
            };
            params.push(genre);
            ("AND s.genre = $2", "s.artist, s.album")
        }
        "random" => ("", "RANDOM()"),
        "frequent" => (
            "",
            "(SELECT COUNT(*) FROM scrobbles sc \
             WHERE sc.song_id = s.id AND sc.user_id = $1) DESC, s.artist, s.album",
        ),
        _ => return param_err("Invalid list type"),
    };
    let offset_parameter = params.len() + 1;
    let limit_parameter = offset_parameter + 1;
    params.push(&skip);
    params.push(&limit);
    let sql = format!(
        "SELECT DISTINCT ON (s.artist, s.album) \
                s.artist, s.album, s.year, s.genre, \
                COUNT(*) OVER w AS song_count, \
                SUM(s.duration_seconds) OVER w AS total_dur, \
                BOOL_OR(s.has_cover_art) OVER w AS has_cover \
         FROM songs s JOIN user_songs us ON s.id = us.song_id \
         WHERE us.user_id = $1 {filter} \
         WINDOW w AS (PARTITION BY s.artist, s.album) \
         ORDER BY {order} OFFSET ${offset_parameter} LIMIT ${limit_parameter}"
    );
    let Ok(rows) = client.query(&sql, &params).await else {
        return db_err_resp();
    };
    let albums = rows
        .iter()
        .map(|row| AlbumRef {
            info: AlbumInfo::from_row(row, "artist", "album"),
            song_count: row.get("song_count"),
            duration: row.try_get("total_dur").ok(),
        })
        .collect();
    ok_resp(AlbumListResponse {
        album_list: AlbumListContainer { album: albums },
    })
}

#[get("/getAlbumList2?<query..>")]
pub(crate) async fn get_album_list2(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: AlbumListQuery,
) -> Json<serde_json::Value> {
    album_list(pool, user, query).await
}

#[get("/getAlbumList?<query..>")]
pub(crate) async fn get_album_list(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: AlbumListQuery,
) -> Json<serde_json::Value> {
    album_list(pool, user, query).await
}

#[get("/getGenres")]
pub(crate) async fn get_genres(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    let Ok(rows) = client
        .query(
            "SELECT s.genre, COUNT(*) AS song_count, \
                    COUNT(DISTINCT s.album) AS album_count \
             FROM songs s JOIN user_songs us ON s.id = us.song_id \
             WHERE us.user_id = $1 AND s.genre IS NOT NULL AND s.genre != '' \
             GROUP BY s.genre ORDER BY s.genre",
            &[&user.id],
        )
        .await
    else {
        return db_err_resp();
    };

    let genres: Vec<GenreItem> = rows
        .iter()
        .map(|r| GenreItem {
            value: r.get("genre"),
            song_count: r.get("song_count"),
            album_count: r.get("album_count"),
        })
        .collect();

    ok_resp(GenresResponse {
        genres: GenreList { genre: genres },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genre_items_use_the_subsonic_value_field() {
        let response = serde_json::to_value(SubsonicResponse::ok(GenresResponse {
            genres: GenreList {
                genre: vec![GenreItem {
                    value: "Rock".into(),
                    song_count: 2,
                    album_count: 1,
                }],
            },
        }))
        .expect("genre response should serialize");
        let genre = &response["subsonic-response"]["genres"]["genre"][0];

        assert_eq!(
            genre,
            &serde_json::json!({
                "value": "Rock",
                "songCount": 2,
                "albumCount": 1,
            })
        );
    }
}
