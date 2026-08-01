use super::*;

// ── Scrobble ──

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NowPlayingResponse {
    #[serde(rename = "nowPlaying")]
    pub now_playing: NowPlayingList,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NowPlayingList {
    pub entry: Vec<NowPlayingEntry>,
}

/// A `Child` plus now-playing annotations (OpenSubsonic `getNowPlaying` entry).
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NowPlayingEntry {
    #[serde(flatten)]
    pub song: SongEntry,
    pub username: String,
    #[serde(rename = "minutesAgo")]
    pub minutes_ago: i32,
    #[serde(rename = "playerId")]
    pub player_id: i32,
    #[serde(rename = "playerName", skip_serializing_if = "Option::is_none")]
    pub player_name: Option<String>,
}

// ── Phase 6: Scrobble ──

#[get("/rest/scrobble")]
pub(crate) async fn scrobble(
    pool: &State<Pool>,
    user: SubsonicUser,
    query: SubsonicQuery,
) -> Json<serde_json::Value> {
    let id = match query.first("id") {
        Some(s) => s,
        None => return param_err("Required parameter 'id' is missing"),
    };
    let Ok(song) = Uuid::parse_str(&id) else {
        return not_found_resp();
    };
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    match in_personal_library(&client, user.id, song).await {
        Ok(true) => {}
        _ => return not_found_resp(),
    }

    let submission = query
        .first("submission")
        .map(|s| s == "true" || s == "1")
        .unwrap_or(true);
    let played_at = query
        .first("time")
        .and_then(|t| t.parse::<i64>().ok())
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .unwrap_or_else(Utc::now);

    if client
        .execute(
            "INSERT INTO scrobbles (user_id, song_id, played_at, submission) VALUES ($1, $2, $3, $4)",
            &[&user.id, &song, &played_at, &submission],
        )
        .await
        .is_err()
    {
        return db_err_resp();
    }
    ok_empty_resp()
}

#[get("/rest/getNowPlaying")]
pub(crate) async fn get_now_playing(
    pool: &State<Pool>,
    _user: SubsonicUser,
) -> Json<serde_json::Value> {
    let Ok(client) = pool.get().await else {
        return db_err_resp();
    };
    let rows = match client
        .query(
            "SELECT s.*, sc.played_at, u.username \
             FROM scrobbles sc JOIN songs s ON sc.song_id = s.id \
             JOIN users u ON sc.user_id = u.id \
             WHERE sc.submission = false ORDER BY sc.played_at DESC LIMIT 10",
            &[],
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return db_err_resp(),
    };
    let now = Utc::now();
    let entry = rows
        .iter()
        .map(|r| {
            let played_at: chrono::DateTime<chrono::Utc> = r.get("played_at");
            let minutes_ago = (now - played_at).num_minutes().max(0) as i32;
            NowPlayingEntry {
                song: row_to_song_entry(r),
                username: r.get("username"),
                minutes_ago,
                player_id: 0,
                player_name: None,
            }
        })
        .collect::<Vec<_>>();
    ok_resp(NowPlayingResponse {
        now_playing: NowPlayingList { entry },
    })
}
