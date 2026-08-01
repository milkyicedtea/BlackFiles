use super::*;

// ── Starred ──

/// Shared starred-items container for `getStarred` / `getStarred2`.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct StarredContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<Vec<ArtistRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<Vec<AlbumRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song: Option<Vec<SongEntry>>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct StarredResponse {
    pub starred: StarredContainer,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Starred2Response {
    #[serde(rename = "starred2")]
    pub starred2: StarredContainer,
}

// ── Phase 6: Starred ──

#[get("/rest/star")]
pub(crate) async fn star(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    // Songs
    for sid in query.all("id") {
        let song = match Uuid::parse_str(&sid) {
            Ok(u) => u,
            Err(_) => return not_found_resp(),
        };
        match in_personal_library(&client, user.id, song).await {
            Ok(true) => {}
            _ => return not_found_resp(),
        }
        if client
            .execute(
                "INSERT INTO starred (user_id, song_id) VALUES ($1, $2) \
                 ON CONFLICT (user_id, song_id) DO NOTHING",
                &[&user.id, &song],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    // Albums
    for aid in query.all("albumId") {
        let (artist, album) = match decode_album_id(&aid) {
            Some(v) => v,
            None => return not_found_resp(),
        };
        let exists = match client
            .query_opt(
                "SELECT 1 FROM songs s JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 AND s.artist = $2 AND s.album = $3",
                &[&user.id, &artist, &album],
            )
            .await
        {
            Ok(r) => r.is_some(),
            Err(_) => return db_err_resp(),
        };
        if !exists {
            return not_found_resp();
        }
        if client
            .execute(
                "INSERT INTO starred (user_id, artist_name, album_name) VALUES ($1, $2, $3) \
                 ON CONFLICT (user_id, artist_name, album_name) DO NOTHING",
                &[&user.id, &artist, &album],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    // Artists
    for aid in query.all("artistId") {
        let artist = match decode_artist_id(&aid) {
            Some(v) => v,
            None => return not_found_resp(),
        };
        let exists = match client
            .query_opt(
                "SELECT 1 FROM songs s JOIN user_songs us ON s.id = us.song_id \
                 WHERE us.user_id = $1 AND s.artist = $2",
                &[&user.id, &artist],
            )
            .await
        {
            Ok(r) => r.is_some(),
            Err(_) => return db_err_resp(),
        };
        if !exists {
            return not_found_resp();
        }
        if client
            .execute(
                "INSERT INTO starred (user_id, artist_name) VALUES ($1, $2) \
                 ON CONFLICT (user_id, artist_name) WHERE album_name IS NULL DO NOTHING",
                &[&user.id, &artist],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    ok_empty_resp()
}

#[get("/rest/unstar")]
pub(crate) async fn unstar(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };

    for sid in query.all("id") {
        let song = match Uuid::parse_str(&sid) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if client
            .execute(
                "DELETE FROM starred WHERE user_id = $1 AND song_id = $2",
                &[&user.id, &song],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    for aid in query.all("albumId") {
        if let Some((artist, album)) = decode_album_id(&aid)
            && client
                .execute(
                    "DELETE FROM starred WHERE user_id = $1 AND artist_name = $2 AND album_name = $3",
                    &[&user.id, &artist, &album],
                )
                .await
                .is_err()
        {
            return db_err_resp();
        }
    }

    for aid in query.all("artistId") {
        if let Some(artist) = decode_artist_id(&aid)
            && client
                .execute(
                    "DELETE FROM starred WHERE user_id = $1 AND artist_name = $2 AND album_name IS NULL",
                    &[&user.id, &artist],
                )
                .await
                .is_err()
        {
            return db_err_resp();
        }
    }

    ok_empty_resp()
}

/// Build the shared starred container (artists, albums, songs) for the user.
async fn build_starred(
    client: &deadpool_postgres::Object,
    user_id: Uuid,
) -> Result<StarredContainer, ()> {
    let artist_rows = client
        .query(
            "SELECT DISTINCT artist_name FROM starred \
             WHERE user_id = $1 AND song_id IS NULL AND album_name IS NULL",
            &[&user_id],
        )
        .await
        .map_err(|_| ())?;
    let artists = Some(
        artist_rows
            .iter()
            .map(|r| {
                let name: String = r.get("artist_name");
                ArtistRef {
                    id: artist_id(&name),
                    name,
                }
            })
            .collect::<Vec<_>>(),
    );

    let album_rows = client
        .query(
            "SELECT st.artist_name, st.album_name, MIN(s.year) AS year, MIN(s.genre) AS genre, \
                    BOOL_OR(s.has_cover_art) AS has_cover, COUNT(*) AS cnt, \
                    COALESCE(SUM(s.duration_seconds), 0) AS dur \
             FROM starred st JOIN songs s ON s.artist = st.artist_name AND s.album = st.album_name \
             JOIN user_songs us ON us.song_id = s.id AND us.user_id = $1 \
             WHERE st.user_id = $1 AND st.song_id IS NULL AND st.album_name IS NOT NULL \
             GROUP BY st.artist_name, st.album_name \
             ORDER BY st.artist_name, st.album_name",
            &[&user_id],
        )
        .await
        .map_err(|_| ())?;
    let albums = Some(
        album_rows
            .iter()
            .map(|r| AlbumRef {
                info: AlbumInfo::from_row(r, "artist_name", "album_name"),
                song_count: r.get("cnt"),
                duration: Some(r.get("dur")),
            })
            .collect::<Vec<_>>(),
    );

    let song_rows = client
        .query(
            "SELECT s.* FROM starred st JOIN songs s ON st.song_id = s.id \
             JOIN user_songs us ON us.song_id = s.id AND us.user_id = $1 \
             WHERE st.user_id = $1 AND st.song_id IS NOT NULL ORDER BY st.created_at",
            &[&user_id],
        )
        .await
        .map_err(|_| ())?;
    let songs = Some(song_rows.iter().map(row_to_song_entry).collect::<Vec<_>>());

    Ok(StarredContainer {
        artist: artists,
        album: albums,
        song: songs,
    })
}

async fn starred_container(pool: &Pool, user_id: Uuid) -> Result<StarredContainer, ()> {
    let client = pool.get().await.map_err(|_| ())?;
    build_starred(&client, user_id).await
}

#[get("/rest/getStarred")]
pub(crate) async fn get_starred(pool: &State<Pool>, user: SubsonicUser) -> Json<serde_json::Value> {
    let container = match starred_container(pool, user.id).await {
        Ok(container) => container,
        Err(()) => return db_err_resp(),
    };
    ok_resp(StarredResponse { starred: container })
}

#[get("/rest/getStarred2")]
pub(crate) async fn get_starred2(
    pool: &State<Pool>,
    user: SubsonicUser,
) -> Json<serde_json::Value> {
    let container = match starred_container(pool, user.id).await {
        Ok(container) => container,
        Err(()) => return db_err_resp(),
    };
    ok_resp(Starred2Response {
        starred2: container,
    })
}
