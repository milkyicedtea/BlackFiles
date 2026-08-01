use super::*;

// ── Playlists ──

/// Summary of a playlist (no entries). Used by `getPlaylists`.
#[derive(Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct PlaylistRef {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(rename = "songCount")]
    pub song_count: i32,
    /// Total duration in seconds.
    pub duration: i64,
    pub public: bool,
    pub created: String,
    pub changed: String,
    #[serde(rename = "coverArt", skip_serializing_if = "Option::is_none")]
    pub cover_art: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PlaylistsResponse {
    pub playlists: PlaylistsList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PlaylistsList {
    pub playlist: Vec<PlaylistRef>,
}

/// A full playlist with its entries. Used by `getPlaylist`.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PlaylistDetailResponse {
    pub playlist: PlaylistDetail,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PlaylistDetail {
    #[serde(flatten)]
    pub base: PlaylistRef,
    pub entry: Vec<SongEntry>,
}

async fn playlist_owner(
    client: &impl deadpool_postgres::GenericClient,
    playlist_id: Uuid,
) -> Result<Option<Uuid>, ()> {
    client
        .query_opt(
            "SELECT user_id FROM playlists WHERE id = $1",
            &[&playlist_id],
        )
        .await
        .map(|row| row.map(|row| row.get("user_id")))
        .map_err(|_| ())
}

// ── Phase 6: Playlists ──

#[get("/rest/getPlaylists")]
pub(crate) async fn get_playlists(
    pool: &State<Pool>,
    user: SubsonicUser,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let rows = match client
        .query(
            "SELECT p.id, p.name, p.comment, p.public, p.created_at, p.updated_at, u.username, \
                    (SELECT COUNT(*) FROM playlist_songs ps WHERE ps.playlist_id = p.id) AS song_count, \
                    COALESCE((SELECT SUM(s.duration_seconds) FROM playlist_songs ps \
                              JOIN songs s ON ps.song_id = s.id WHERE ps.playlist_id = p.id), 0) AS duration \
             FROM playlists p JOIN users u ON p.user_id = u.id \
             WHERE p.user_id = $1 OR p.public = TRUE ORDER BY p.name",
            &[&user.id],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return db_err_resp(),
    };
    let playlist = rows
        .iter()
        .map(|r| PlaylistRef {
            id: r.get::<_, Uuid>("id").to_string(),
            name: r.get("name"),
            comment: r.try_get("comment").ok(),
            owner: Some(r.get::<_, String>("username")),
            song_count: r.get::<_, i64>("song_count") as i32,
            duration: r.get::<_, f64>("duration") as i64,
            public: r.get("public"),
            created: r
                .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
                .to_rfc3339(),
            changed: r
                .get::<_, chrono::DateTime<chrono::Utc>>("updated_at")
                .to_rfc3339(),
            cover_art: None,
        })
        .collect::<Vec<_>>();
    ok_resp(PlaylistsResponse {
        playlists: PlaylistsList { playlist },
    })
}

#[get("/rest/getPlaylist?<id>")]
pub(crate) async fn get_playlist(
    pool: &State<Pool>,
    user: SubsonicUser,
    id: String,
) -> Json<serde_json::Value> {
    let Ok(pid) = Uuid::parse_str(&id) else {
        return not_found_resp();
    };
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let row = match client
        .query_opt(
            "SELECT p.name, p.comment, p.public, p.created_at, p.updated_at, p.user_id \
             FROM playlists p WHERE p.id = $1",
            &[&pid],
        )
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return not_found_resp(),
        Err(_) => return db_err_resp(),
    };
    let owner_id: Uuid = row.get("user_id");
    let public: bool = row.get("public");
    if owner_id != user.id && !public {
        return not_found_resp();
    }

    let song_rows = match client
        .query(
            "SELECT s.* FROM playlist_songs ps JOIN songs s ON ps.song_id = s.id \
             WHERE ps.playlist_id = $1 ORDER BY ps.position",
            &[&pid],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return db_err_resp(),
    };
    let entry: Vec<SongEntry> = song_rows.iter().map(row_to_song_entry).collect();
    let song_count = entry.len() as i32;
    let duration = song_rows
        .iter()
        .map(|r| r.try_get::<_, f64>("duration_seconds").ok().unwrap_or(0.0) as i64)
        .sum();

    let base = PlaylistRef {
        id: pid.to_string(),
        name: row.get("name"),
        comment: row.try_get("comment").ok(),
        owner: Some(user.username.clone()),
        song_count,
        duration,
        public,
        created: row
            .get::<_, chrono::DateTime<chrono::Utc>>("created_at")
            .to_rfc3339(),
        changed: row
            .get::<_, chrono::DateTime<chrono::Utc>>("updated_at")
            .to_rfc3339(),
        cover_art: None,
    };
    ok_resp(PlaylistDetailResponse {
        playlist: PlaylistDetail { base, entry },
    })
}

#[get("/rest/createPlaylist")]
pub(crate) async fn create_playlist(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let raw_songs = query.all("songId");
    let mut requested: Vec<Uuid> = Vec::new();
    for s in &raw_songs {
        match Uuid::parse_str(s) {
            Ok(u) => {
                if !requested.contains(&u) {
                    requested.push(u);
                }
            }
            Err(_) => return not_found_resp(),
        }
    }

    let Ok(mut client) = pool.get().await else {
        return db_err_resp();
    };
    let songs = match personal_song_ids(&client, user.id, &requested).await {
        Ok(v) => v,
        Err(_) => return db_err_resp(),
    };
    if songs.len() != requested.len() {
        return not_found_resp();
    }

    let playlist_id_str = query.first("playlistId");
    let name = query.first("name");

    let tx = match client.transaction().await {
        Ok(t) => t,
        Err(_) => return db_err_resp(),
    };

    let pid: Uuid = if let Some(pid_str) = playlist_id_str {
        // create-or-replace: must own the existing playlist.
        let pid = match Uuid::parse_str(&pid_str) {
            Ok(u) => u,
            Err(_) => return not_found_resp(),
        };
        let owner = match playlist_owner(&tx, pid).await {
            Ok(Some(owner)) => owner,
            Ok(None) => return not_found_resp(),
            Err(()) => return db_err_resp(),
        };
        if owner != user.id {
            return not_found_resp();
        }
        if let Some(n) = name
            && tx
                .execute("UPDATE playlists SET name = $1 WHERE id = $2", &[&n, &pid])
                .await
                .is_err()
        {
            return db_err_resp();
        }
        // Replace the entire song list.
        if tx
            .execute("DELETE FROM playlist_songs WHERE playlist_id = $1", &[&pid])
            .await
            .is_err()
        {
            return db_err_resp();
        }
        pid
    } else {
        let n = match name {
            Some(n) if !n.is_empty() => n,
            _ => return param_err("Required parameter 'name' is missing"),
        };
        let row = match tx
            .query_one(
                "INSERT INTO playlists (user_id, name) VALUES ($1, $2) RETURNING id",
                &[&user.id, &n],
            )
            .await
        {
            Ok(r) => r,
            Err(_) => return db_err_resp(),
        };
        row.get::<_, Uuid>("id")
    };

    for (i, song) in songs.iter().enumerate() {
        if tx
            .execute(
                "INSERT INTO playlist_songs (playlist_id, song_id, position) VALUES ($1, $2, $3)",
                &[&pid, song, &(i as i32)],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    if tx.commit().await.is_err() {
        return db_err_resp();
    }
    ok_empty_resp()
}

#[get("/rest/updatePlaylist")]
pub(crate) async fn update_playlist(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let pid = match query
        .first("playlistId")
        .and_then(|s| Uuid::parse_str(&s).ok())
    {
        Some(u) => u,
        None => return param_err("Required parameter 'playlistId' is missing"),
    };

    let Ok(mut client) = pool.get().await else {
        return db_err_resp();
    };

    // Ownership check.
    let owner = match playlist_owner(&client, pid).await {
        Ok(Some(owner)) => owner,
        Ok(None) => return not_found_resp(),
        Err(()) => return db_err_resp(),
    };
    if owner != user.id {
        return not_found_resp();
    }

    // Current ordered song list.
    let cur_rows = match client
        .query(
            "SELECT song_id FROM playlist_songs WHERE playlist_id = $1 ORDER BY position",
            &[&pid],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return db_err_resp(),
    };
    let original: Vec<Uuid> = cur_rows.iter().map(|r| r.get("song_id")).collect();
    let mut list = original.clone();

    // Remove by zero-based index, descending so earlier removals don't shift
    // lower indices still pending.
    let mut removals: Vec<usize> = query
        .all("songIndexToRemove")
        .iter()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();
    removals.sort_unstable_by(|a, b| b.cmp(a));
    for idx in removals {
        if idx < list.len() {
            list.remove(idx);
        }
    }

    // Add new songs (validated against the personal library).
    let to_add_raw = query.all("songIdToAdd");
    let mut parsed_add: Vec<Uuid> = Vec::new();
    for s in &to_add_raw {
        match Uuid::parse_str(s) {
            Ok(u) => {
                if !list.contains(&u) && !parsed_add.contains(&u) {
                    parsed_add.push(u);
                }
            }
            Err(_) => return not_found_resp(),
        }
    }
    if !parsed_add.is_empty() {
        let present = match personal_song_ids(&client, user.id, &parsed_add).await {
            Ok(v) => v,
            Err(_) => return db_err_resp(),
        };
        if present.len() != parsed_add.len() {
            return not_found_resp();
        }
        for u in present {
            list.push(u);
        }
    }

    let name = query.first("name");
    let comment = query.first("comment");
    let public = query.first("public").and_then(|s| s.parse::<bool>().ok());
    let metadata = name.is_some() || comment.is_some() || public.is_some();
    let songs_changed = list != original;

    if !metadata && !songs_changed {
        return ok_empty_resp();
    }

    let tx = match client.transaction().await {
        Ok(t) => t,
        Err(_) => return db_err_resp(),
    };

    if metadata {
        if tx
            .execute(
                "UPDATE playlists SET name = COALESCE($1, name), \
                 comment = COALESCE($2, comment), public = COALESCE($3, public) WHERE id = $4",
                &[&name, &comment, &public, &pid],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    } else if songs_changed {
        // Bump `changed` even when only the song list moved (the trigger fires
        // on any playlist row UPDATE).
        if tx
            .execute(
                "UPDATE playlists SET updated_at = NOW() WHERE id = $1",
                &[&pid],
            )
            .await
            .is_err()
        {
            return db_err_resp();
        }
    }

    if songs_changed {
        if tx
            .execute("DELETE FROM playlist_songs WHERE playlist_id = $1", &[&pid])
            .await
            .is_err()
        {
            return db_err_resp();
        }
        for (i, song) in list.iter().enumerate() {
            if tx
                .execute(
                    "INSERT INTO playlist_songs (playlist_id, song_id, position) \
                     VALUES ($1, $2, $3)",
                    &[&pid, song, &(i as i32)],
                )
                .await
                .is_err()
            {
                return db_err_resp();
            }
        }
    }

    if tx.commit().await.is_err() {
        return db_err_resp();
    }
    ok_empty_resp()
}

#[get("/rest/deletePlaylist")]
pub(crate) async fn delete_playlist(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let pid = match query.first("id").and_then(|s| Uuid::parse_str(&s).ok()) {
        Some(u) => u,
        None => return param_err("Required parameter 'id' is missing"),
    };
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let affected = match client
        .execute(
            "DELETE FROM playlists WHERE id = $1 AND user_id = $2",
            &[&pid, &user.id],
        )
        .await
    {
        Ok(n) => n,
        Err(_) => return db_err_resp(),
    };
    if affected == 0 {
        return not_found_resp();
    }
    ok_empty_resp()
}
