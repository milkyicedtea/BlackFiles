use base64::Engine;
use deadpool_postgres::Pool;
use rocket::State;
use rocket::data::{Data, ToByteUnit};
use rocket::http::{Header, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::{Responder, Response};
use rocket::serde::{Serialize, json::Json};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use crate::guards::{AuthenticatedUser, check_permission};
use crate::shared::{
    STORAGE_ROOT, bad_request, conflict, db_error, forbidden, get_client, not_found,
    path_to_web_string, sanitize_path, server_error,
};
use crate::upload_links::hash_token;

const TUS_VERSION: &str = "1.0.0";
const TUS_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024;
const TEMP_DIRECTORY: &str = ".uploads";

pub(crate) type ApiError = (Status, Json<serde_json::Value>);

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct PendingTusUpload {
    id: Uuid,
    target_path: String,
    upload_length: i64,
    upload_offset: i64,
}

pub(crate) struct TusHeaders {
    upload_length: Option<String>,
    upload_offset: Option<String>,
    upload_metadata: Option<String>,
    content_length: Option<String>,
    content_type: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TusHeaders {
    type Error = Status;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if request.headers().get_one("Tus-Resumable") != Some(TUS_VERSION) {
            return Outcome::Error((Status::PreconditionFailed, Status::PreconditionFailed));
        }

        let header = |name| request.headers().get_one(name).map(str::to_owned);
        Outcome::Success(Self {
            upload_length: header("Upload-Length"),
            upload_offset: header("Upload-Offset"),
            upload_metadata: header("Upload-Metadata"),
            content_length: header("Content-Length"),
            content_type: header("Content-Type"),
        })
    }
}

impl TusHeaders {
    fn required_u64(&self, value: Option<&String>, name: &str) -> Result<u64, ApiError> {
        value
            .ok_or_else(|| bad_request(&format!("Missing {name} header")))?
            .parse::<u64>()
            .map_err(|_| bad_request(&format!("Invalid {name} header")))
    }

    fn upload_length(&self) -> Result<u64, ApiError> {
        self.required_u64(self.upload_length.as_ref(), "Upload-Length")
    }

    fn upload_offset(&self) -> Result<u64, ApiError> {
        self.required_u64(self.upload_offset.as_ref(), "Upload-Offset")
    }

    fn content_length(&self) -> Result<u64, ApiError> {
        self.required_u64(self.content_length.as_ref(), "Content-Length")
    }

    fn metadata(&self) -> Result<&str, ApiError> {
        self.upload_metadata
            .as_deref()
            .ok_or_else(|| bad_request("Missing Upload-Metadata header"))
    }

    fn is_offset_octet_stream(&self) -> bool {
        self.content_type
            .as_deref()
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.trim() == "application/offset+octet-stream")
    }
}

pub(crate) struct TusResponse {
    status: Status,
    location: Option<String>,
    offset: Option<u64>,
    length: Option<u64>,
    options: bool,
    no_store: bool,
}

impl TusResponse {
    fn options() -> Self {
        Self {
            status: Status::NoContent,
            location: None,
            offset: None,
            length: None,
            options: true,
            no_store: false,
        }
    }

    fn created(location: String) -> Self {
        Self {
            status: Status::Created,
            location: Some(location),
            offset: Some(0),
            length: None,
            options: false,
            no_store: true,
        }
    }

    fn head(offset: u64, length: u64) -> Self {
        Self {
            status: Status::Ok,
            location: None,
            offset: Some(offset),
            length: Some(length),
            options: false,
            no_store: true,
        }
    }

    fn patched(offset: u64) -> Self {
        Self {
            status: Status::NoContent,
            location: None,
            offset: Some(offset),
            length: None,
            options: false,
            no_store: true,
        }
    }

    fn terminated() -> Self {
        Self {
            status: Status::NoContent,
            location: None,
            offset: None,
            length: None,
            options: false,
            no_store: true,
        }
    }
}

impl<'r> Responder<'r, 'static> for TusResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut response = Response::build();
        response
            .status(self.status)
            .header(Header::new("Tus-Resumable", TUS_VERSION));

        if let Some(location) = self.location {
            response.header(Header::new("Location", location));
        }
        if let Some(offset) = self.offset {
            response.header(Header::new("Upload-Offset", offset.to_string()));
        }
        if let Some(length) = self.length {
            response.header(Header::new("Upload-Length", length.to_string()));
        }
        if self.options {
            response
                .header(Header::new("Tus-Version", TUS_VERSION))
                .header(Header::new("Tus-Extension", "creation,termination"))
                .header(Header::new("Tus-Max-Size", MAX_UPLOAD_SIZE.to_string()));
        }
        if self.no_store {
            response.header(Header::new("Cache-Control", "no-store"));
        }

        response.ok()
    }
}

fn temporary_path(id: Uuid) -> PathBuf {
    Path::new(STORAGE_ROOT)
        .join(TEMP_DIRECTORY)
        .join(format!("{id}.part"))
}

fn filename_from_metadata(metadata: &str) -> Result<PathBuf, ApiError> {
    let metadata = parse_metadata(metadata)?;
    let filename = metadata
        .get("filename")
        .ok_or_else(|| bad_request("Upload metadata must include filename"))?;
    let filename = PathBuf::from(filename);
    if filename.components().count() != 1 {
        return Err(bad_request("Invalid filename"));
    }
    sanitize_path(filename).ok_or_else(|| bad_request("Invalid filename"))
}

fn destination_from_metadata(metadata: &str) -> Result<(String, PathBuf), ApiError> {
    let metadata = parse_metadata(metadata)?;
    let filename = metadata
        .get("filename")
        .ok_or_else(|| bad_request("Upload metadata must include filename"))?;
    let filename = PathBuf::from(filename);
    if filename.components().count() != 1 {
        return Err(bad_request("Invalid filename"));
    }
    let filename = sanitize_path(filename).ok_or_else(|| bad_request("Invalid filename"))?;

    let target_directory = metadata
        .get("targetPath")
        .map(String::as_str)
        .unwrap_or_default();

    let target_directory = if target_directory.is_empty() {
        PathBuf::new()
    } else {
        sanitize_path(PathBuf::from(target_directory))
            .ok_or_else(|| bad_request("Invalid target path"))?
    };

    let relative_path = target_directory.join(filename);
    let target_path = path_to_web_string(&relative_path);
    Ok((target_path, Path::new(STORAGE_ROOT).join(relative_path)))
}

fn parse_metadata(value: &str) -> Result<std::collections::HashMap<String, String>, ApiError> {
    let mut metadata = std::collections::HashMap::new();

    for entry in value.split(',') {
        let entry = entry.trim();
        let (key, encoded_value) = entry.split_once(' ').unwrap_or((entry, ""));
        if key.is_empty() || metadata.contains_key(key) {
            return Err(bad_request("Invalid Upload-Metadata header"));
        }

        let decoded = if encoded_value.is_empty() {
            Vec::new()
        } else {
            base64::engine::general_purpose::STANDARD
                .decode(encoded_value)
                .map_err(|_| bad_request("Invalid Upload-Metadata header"))?
        };
        let decoded = String::from_utf8(decoded)
            .map_err(|_| bad_request("Invalid Upload-Metadata header"))?;
        metadata.insert(key.to_owned(), decoded);
    }

    Ok(metadata)
}

async fn require_upload_permission(pool: &Pool, user: &AuthenticatedUser) -> Result<(), ApiError> {
    if !check_permission(pool, user.id, "upload_files")
        .await
        .unwrap_or(false)
    {
        return Err(forbidden());
    }
    Ok(())
}

pub(crate) async fn cleanup_expired_uploads(pool: &Pool) {
    let client = match pool.get().await {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Unable to clean expired uploads: {error}");
            return;
        }
    };
    let rows = match client
        .query(
            "DELETE FROM upload_sessions WHERE expires_at <= NOW() RETURNING id",
            &[],
        )
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Unable to clean expired uploads: {error}");
            return;
        }
    };

    for row in rows {
        let id: Uuid = row.get("id");
        fs::remove_file(temporary_path(id)).await.ok();
    }
}

fn as_i64(value: u64) -> Result<i64, ApiError> {
    i64::try_from(value).map_err(|_| bad_request("Upload is too large"))
}

fn as_u64(value: i64) -> Result<u64, ApiError> {
    u64::try_from(value).map_err(|_| server_error())
}

fn parse_upload_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| not_found("Upload not found"))
}

#[options("/uploads")]
pub(crate) fn tus_options() -> TusResponse {
    TusResponse::options()
}

#[get("/uploads")]
pub(crate) async fn list_tus_uploads(
    pool: &State<Pool>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<PendingTusUpload>>, ApiError> {
    require_upload_permission(pool, &user).await?;
    cleanup_expired_uploads(pool).await;
    let client = get_client(pool).await?;
    let rows = client
        .query(
            "SELECT id, target_path, upload_length, upload_offset
             FROM upload_sessions
             WHERE user_id = $1 AND expires_at > NOW()
             ORDER BY created_at",
            &[&user.id],
        )
        .await
        .map_err(db_error)?;

    Ok(Json(
        rows.into_iter()
            .map(|row| PendingTusUpload {
                id: row.get("id"),
                target_path: row.get("target_path"),
                upload_length: row.get("upload_length"),
                upload_offset: row.get("upload_offset"),
            })
            .collect(),
    ))
}

#[post("/uploads")]
pub(crate) async fn create_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    require_upload_permission(pool, &user).await?;
    cleanup_expired_uploads(pool).await;

    let length = headers.upload_length()?;
    if length > MAX_UPLOAD_SIZE {
        return Err(bad_request("Upload exceeds the maximum size"));
    }
    let (target_path, destination) = destination_from_metadata(headers.metadata()?)?;

    match fs::metadata(&destination).await {
        Ok(_) => return Err(conflict("A file with this name already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(server_error()),
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }
    let temp_directory = Path::new(STORAGE_ROOT).join(TEMP_DIRECTORY);
    fs::create_dir_all(&temp_directory)
        .await
        .map_err(|_| server_error())?;

    let id = Uuid::new_v4();
    let temporary = temporary_path(id);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;

    let length = as_i64(length)?;
    let client = get_client(pool).await?;
    let inserted = client
        .execute(
            "INSERT INTO upload_sessions (id, user_id, target_path, upload_length, expires_at)
             VALUES ($1, $2, $3, $4, NOW() + INTERVAL '24 hours')",
            &[&id, &user.id, &target_path, &length],
        )
        .await;

    if let Err(error) = inserted {
        fs::remove_file(&temporary).await.ok();
        if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
            return Err(conflict(
                "An upload to this destination is already in progress",
            ));
        }
        return Err(db_error(error));
    }

    Ok(TusResponse::created(format!("/api/uploads/{id}")))
}

#[head("/uploads/<id>")]
pub(crate) async fn head_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_upload_permission(pool, &user).await?;
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;

    Ok(TusResponse::head(
        as_u64(row.get::<_, i64>("upload_offset"))?,
        as_u64(row.get::<_, i64>("upload_length"))?,
    ))
}

#[patch("/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_upload_permission(pool, &user).await?;
    if !headers.is_offset_octet_stream() {
        return Err(bad_request(
            "PATCH requires application/offset+octet-stream",
        ));
    }

    let requested_offset = headers.upload_offset()?;
    let content_length = headers.content_length()?;
    if content_length > TUS_CHUNK_SIZE {
        return Err(bad_request("Upload chunk exceeds the maximum size"));
    }

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let row = transaction
        .query_opt(
            "SELECT target_path, upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()
             FOR UPDATE",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;

    let target_path: String = row.get("target_path");
    let length = as_u64(row.get::<_, i64>("upload_length"))?;
    let offset = as_u64(row.get::<_, i64>("upload_offset"))?;
    if offset == length {
        return Err(conflict("Upload is already complete"));
    }
    if requested_offset != offset {
        return Err(conflict("Upload offset does not match the server offset"));
    }
    let remaining = length.checked_sub(offset).ok_or_else(server_error)?;
    if content_length > remaining {
        return Err(bad_request("Upload chunk exceeds the declared file size"));
    }

    let temporary = temporary_path(id);
    let metadata = fs::metadata(&temporary).await.map_err(|_| server_error())?;
    if metadata.len() != offset {
        eprintln!("Upload session {id} has an inconsistent temporary file offset");
        return Err(server_error());
    }

    let mut file = OpenOptions::new()
        .write(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;
    file.seek(SeekFrom::Start(offset))
        .await
        .map_err(|_| server_error())?;
    let written = *data
        .open(content_length.bytes())
        .stream_to(&mut file)
        .await
        .map_err(|_| server_error())?;
    if written != content_length {
        return Err(server_error());
    }
    file.flush().await.map_err(|_| server_error())?;
    file.sync_data().await.map_err(|_| server_error())?;

    let next_offset = offset.checked_add(written).ok_or_else(server_error)?;
    transaction
        .execute(
            "UPDATE upload_sessions SET upload_offset = $1 WHERE id = $2 AND user_id = $3",
            &[&as_i64(next_offset)?, &id, &user.id],
        )
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;

    if next_offset == length {
        finalize_upload(pool, id, user.id, &target_path).await?;
    }

    Ok(TusResponse::patched(next_offset))
}

async fn finalize_upload(
    pool: &Pool,
    id: Uuid,
    user_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    let temporary = temporary_path(id);
    let destination = Path::new(STORAGE_ROOT).join(target_path);

    match fs::hard_link(&temporary, &destination).await {
        Ok(()) => {
            fs::remove_file(&temporary)
                .await
                .map_err(|_| server_error())?;
            let client = get_client(pool).await?;
            if let Err(error) = client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2",
                    &[&id, &user_id],
                )
                .await
            {
                eprintln!(
                    "Completed upload {id} could not be removed from the session table: {error}"
                );
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(&temporary).await.ok();
            let client = get_client(pool).await?;
            client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2",
                    &[&id, &user_id],
                )
                .await
                .map_err(db_error)?;
            Err(conflict("A file with this name already exists"))
        }
        Err(_) => Err(server_error()),
    }
}

#[delete("/uploads/<id>")]
pub(crate) async fn terminate_tus_upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    require_upload_permission(pool, &user).await?;
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2 RETURNING id",
            &[&id, &user.id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    let id: Uuid = row.get("id");
    fs::remove_file(temporary_path(id)).await.ok();

    Ok(TusResponse::terminated())
}

#[options("/public/upload-links/<_token>/uploads")]
pub(crate) fn public_tus_options(_token: &str) -> TusResponse {
    TusResponse::options()
}

#[post("/public/upload-links/<token>/uploads")]
pub(crate) async fn create_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    headers: TusHeaders,
) -> Result<TusResponse, ApiError> {
    cleanup_expired_uploads(pool).await;

    let length = headers.upload_length()?;
    if length > MAX_UPLOAD_SIZE {
        return Err(bad_request("Upload exceeds the maximum size"));
    }
    let filename = filename_from_metadata(headers.metadata()?)?;
    let token_hash = hash_token(token);

    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let link = transaction
        .query_opt(
            "SELECT id, target_path FROM upload_links
             WHERE token_hash = $1 AND used_at IS NULL
             FOR UPDATE",
            &[&token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload link is invalid or has already been used"))?;
    let link_id: Uuid = link.get("id");
    let target_directory = PathBuf::from(link.get::<_, String>("target_path"));
    let relative_path = target_directory.join(filename);
    let target_path = path_to_web_string(&relative_path);
    let destination = Path::new(STORAGE_ROOT).join(&relative_path);

    match fs::metadata(&destination).await {
        Ok(_) => return Err(conflict("A file with this name already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(server_error()),
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }
    let temp_directory = Path::new(STORAGE_ROOT).join(TEMP_DIRECTORY);
    fs::create_dir_all(&temp_directory)
        .await
        .map_err(|_| server_error())?;

    let id = Uuid::new_v4();
    let temporary = temporary_path(id);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;

    let length = as_i64(length)?;
    let inserted = transaction
        .execute(
            "INSERT INTO upload_sessions (id, upload_link_id, target_path, upload_length, expires_at)
             VALUES ($1, $2, $3, $4, NOW() + INTERVAL '24 hours')",
            &[&id, &link_id, &target_path, &length],
        )
        .await;
    if let Err(error) = inserted {
        fs::remove_file(&temporary).await.ok();
        if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
            return Err(conflict(
                "An upload for this link or destination is already in progress",
            ));
        }
        return Err(db_error(error));
    }
    transaction.commit().await.map_err(db_error)?;

    Ok(TusResponse::created(format!(
        "/api/public/upload-links/{token}/uploads/{id}"
    )))
}

#[head("/public/upload-links/<token>/uploads/<id>")]
pub(crate) async fn head_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    let token_hash = hash_token(token);
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT s.upload_length, s.upload_offset
             FROM upload_sessions s
             JOIN upload_links l ON l.id = s.upload_link_id
             WHERE s.id = $1 AND l.token_hash = $2 AND l.used_at IS NULL
               AND s.expires_at > NOW()",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;

    Ok(TusResponse::head(
        as_u64(row.get::<_, i64>("upload_offset"))?,
        as_u64(row.get::<_, i64>("upload_length"))?,
    ))
}

#[patch("/public/upload-links/<token>/uploads/<id>", data = "<data>")]
pub(crate) async fn patch_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    headers: TusHeaders,
    id: &str,
    data: Data<'_>,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    if !headers.is_offset_octet_stream() {
        return Err(bad_request(
            "PATCH requires application/offset+octet-stream",
        ));
    }
    let requested_offset = headers.upload_offset()?;
    let content_length = headers.content_length()?;
    if content_length > TUS_CHUNK_SIZE {
        return Err(bad_request("Upload chunk exceeds the maximum size"));
    }

    let token_hash = hash_token(token);
    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let row = transaction
        .query_opt(
            "SELECT s.upload_link_id, s.target_path, s.upload_length, s.upload_offset
             FROM upload_sessions s
             JOIN upload_links l ON l.id = s.upload_link_id
             WHERE s.id = $1 AND l.token_hash = $2 AND l.used_at IS NULL
               AND s.expires_at > NOW()
             FOR UPDATE OF s, l",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    let link_id: Uuid = row.get("upload_link_id");
    let target_path: String = row.get("target_path");
    let length = as_u64(row.get::<_, i64>("upload_length"))?;
    let offset = as_u64(row.get::<_, i64>("upload_offset"))?;
    if offset == length {
        return Err(conflict("Upload is already complete"));
    }
    if requested_offset != offset {
        return Err(conflict("Upload offset does not match the server offset"));
    }
    let remaining = length.checked_sub(offset).ok_or_else(server_error)?;
    if content_length > remaining {
        return Err(bad_request("Upload chunk exceeds the declared file size"));
    }

    let temporary = temporary_path(id);
    let metadata = fs::metadata(&temporary).await.map_err(|_| server_error())?;
    if metadata.len() != offset {
        eprintln!("Upload session {id} has an inconsistent temporary file offset");
        return Err(server_error());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;
    file.seek(SeekFrom::Start(offset))
        .await
        .map_err(|_| server_error())?;
    let written = *data
        .open(content_length.bytes())
        .stream_to(&mut file)
        .await
        .map_err(|_| server_error())?;
    if written != content_length {
        return Err(server_error());
    }
    file.flush().await.map_err(|_| server_error())?;
    file.sync_data().await.map_err(|_| server_error())?;

    let next_offset = offset.checked_add(written).ok_or_else(server_error)?;
    transaction
        .execute(
            "UPDATE upload_sessions
             SET upload_offset = $1
             WHERE id = $2 AND upload_link_id = $3",
            &[&as_i64(next_offset)?, &id, &link_id],
        )
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;

    if next_offset == length {
        finalize_public_upload(pool, id, link_id, &target_path).await?;
    }
    Ok(TusResponse::patched(next_offset))
}

async fn finalize_public_upload(
    pool: &Pool,
    id: Uuid,
    link_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    let temporary = temporary_path(id);
    let destination = Path::new(STORAGE_ROOT).join(target_path);
    match fs::hard_link(&temporary, &destination).await {
        Ok(()) => {
            let mut client = get_client(pool).await?;
            let transaction = client.transaction().await.map_err(db_error)?;
            let marked = transaction
                .execute(
                    "UPDATE upload_links SET used_at = NOW()
                     WHERE id = $1 AND used_at IS NULL",
                    &[&link_id],
                )
                .await
                .map_err(db_error)?;
            if marked != 1 {
                return Err(conflict("Upload link is no longer available"));
            }
            transaction
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND upload_link_id = $2",
                    &[&id, &link_id],
                )
                .await
                .map_err(db_error)?;
            transaction.commit().await.map_err(db_error)?;
            fs::remove_file(&temporary)
                .await
                .map_err(|_| server_error())?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(&temporary).await.ok();
            let client = get_client(pool).await?;
            client
                .execute(
                    "DELETE FROM upload_sessions WHERE id = $1 AND upload_link_id = $2",
                    &[&id, &link_id],
                )
                .await
                .map_err(db_error)?;
            Err(conflict("A file with this name already exists"))
        }
        Err(_) => Err(server_error()),
    }
}

#[delete("/public/upload-links/<token>/uploads/<id>")]
pub(crate) async fn terminate_public_tus_upload(
    pool: &State<Pool>,
    token: &str,
    _headers: TusHeaders,
    id: &str,
) -> Result<TusResponse, ApiError> {
    let id = parse_upload_id(id)?;
    let token_hash = hash_token(token);
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "DELETE FROM upload_sessions s
             USING upload_links l
             WHERE s.id = $1 AND s.upload_link_id = l.id
               AND l.token_hash = $2 AND l.used_at IS NULL
             RETURNING s.id",
            &[&id, &token_hash],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    let id: Uuid = row.get("id");
    fs::remove_file(temporary_path(id)).await.ok();

    Ok(TusResponse::terminated())
}
