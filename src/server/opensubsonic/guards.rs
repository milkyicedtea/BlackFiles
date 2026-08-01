use super::*;

// ── Error helper ──

pub(super) fn api_err(code: i32, msg: &str) -> (Status, Json<serde_json::Value>) {
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

pub(super) fn query_param(request: &Request<'_>, key: &str) -> Option<String> {
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
        if let Some(api_key) = query_param(request, "apiKey")
            && !api_key.is_empty()
        {
            let key_hash = sha256_hex(&api_key);
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
        if argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_err()
        {
            return Outcome::Error(api_err(40, "Wrong username or password"));
        }
        Outcome::Success(SubsonicUser {
            id: row.get("id"),
            username: row.get("username"),
        })
    }
}

/// Parsed Subsonic query string with multi-value support.
///
/// Subsonic repeats keys for lists (e.g. `songId=a&songId=b`); this guard
/// captures every `key=value` pair once so handlers can read both single
/// (`first`) and repeated (`all`) parameters without relying on Rocket's
/// per-key typed query fields.
pub(crate) struct SubsonicQuery {
    pairs: Vec<(String, String)>,
}

impl SubsonicQuery {
    /// First decoded value for `key`, if present.
    pub(crate) fn first(&self, key: &str) -> Option<String> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }

    /// All decoded values for `key`, preserving request order.
    pub(crate) fn all(&self, key: &str) -> Vec<String> {
        self.pairs
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .collect()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SubsonicQuery {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut pairs = Vec::new();
        if let Some(q) = request.uri().query() {
            for pair in q.split('&') {
                if let Some((k, v)) = pair.as_str().split_once('=') {
                    pairs.push((k.to_string(), url_decode(v)));
                }
            }
        }
        Outcome::Success(SubsonicQuery { pairs })
    }
}
