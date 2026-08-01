use base64::Engine;
use deadpool_postgres::Pool;
use rocket::Request;
use rocket::data::{Data, ToByteUnit};
use rocket::http::{Header, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::{Responder, Response};
use rocket::serde::Serialize;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_postgres::Row;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

use super::{
    ApiError, MUSIC_ROOT, STORAGE_ROOT, bad_request, conflict, db_error, get_client, not_found,
    path_to_web_string, sanitize_path, server_error,
};

const TUS_VERSION: &str = "1.0.0";
pub(crate) const TUS_CHUNK_SIZE: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_UPLOAD_SIZE: u64 = 10 * 1024 * 1024 * 1024;
const TEMP_DIRECTORY: &str = ".uploads";

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct PendingTusUpload {
    pub(crate) id: Uuid,
    pub(crate) target_path: String,
    pub(crate) upload_length: i64,
    pub(crate) upload_offset: i64,
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

    pub(crate) fn upload_length(&self) -> Result<u64, ApiError> {
        self.required_u64(self.upload_length.as_ref(), "Upload-Length")
    }

    pub(crate) fn upload_offset(&self) -> Result<u64, ApiError> {
        self.required_u64(self.upload_offset.as_ref(), "Upload-Offset")
    }

    pub(crate) fn content_length(&self) -> Result<u64, ApiError> {
        self.required_u64(self.content_length.as_ref(), "Content-Length")
    }

    pub(crate) fn metadata(&self) -> Result<&str, ApiError> {
        self.upload_metadata
            .as_deref()
            .ok_or_else(|| bad_request("Missing Upload-Metadata header"))
    }

    pub(crate) fn is_offset_octet_stream(&self) -> bool {
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
    pub(crate) fn options() -> Self {
        Self {
            status: Status::NoContent,
            location: None,
            offset: None,
            length: None,
            options: true,
            no_store: false,
        }
    }

    pub(crate) fn created(location: String) -> Self {
        Self {
            status: Status::Created,
            location: Some(location),
            offset: Some(0),
            length: None,
            options: false,
            no_store: true,
        }
    }

    pub(crate) fn head(offset: u64, length: u64) -> Self {
        Self {
            status: Status::Ok,
            location: None,
            offset: Some(offset),
            length: Some(length),
            options: false,
            no_store: true,
        }
    }

    pub(crate) fn patched(offset: u64) -> Self {
        Self {
            status: Status::NoContent,
            location: None,
            offset: Some(offset),
            length: None,
            options: false,
            no_store: true,
        }
    }

    pub(crate) fn terminated() -> Self {
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

pub(crate) struct UploadProgress {
    pub(crate) target_path: String,
    pub(crate) length: u64,
    pub(crate) offset: u64,
}

pub(crate) fn temporary_path(storage_root: &str, id: Uuid) -> PathBuf {
    Path::new(storage_root)
        .join(TEMP_DIRECTORY)
        .join(format!("{id}.part"))
}

pub(crate) fn filename_from_metadata(metadata: &str) -> Result<PathBuf, ApiError> {
    let metadata = parse_metadata(metadata)?;
    let filename = metadata
        .get("filename")
        .ok_or_else(|| bad_request("Upload metadata must include filename"))?;
    validate_filename(filename)
}

pub(crate) fn destination_from_metadata(
    storage_root: &str,
    metadata: &str,
) -> Result<(String, PathBuf), ApiError> {
    let metadata = parse_metadata(metadata)?;
    let filename = metadata
        .get("filename")
        .ok_or_else(|| bad_request("Upload metadata must include filename"))?;
    let filename = validate_filename(filename)?;
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
    Ok((target_path, Path::new(storage_root).join(relative_path)))
}

fn validate_filename(filename: &str) -> Result<PathBuf, ApiError> {
    let filename = PathBuf::from(filename);
    if filename.components().count() != 1 {
        return Err(bad_request("Invalid filename"));
    }
    sanitize_path(filename).ok_or_else(|| bad_request("Invalid filename"))
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

pub(crate) fn as_i64(value: u64) -> Result<i64, ApiError> {
    i64::try_from(value).map_err(|_| bad_request("Upload is too large"))
}

pub(crate) fn as_u64(value: i64) -> Result<u64, ApiError> {
    u64::try_from(value).map_err(|_| server_error())
}

pub(crate) fn parse_upload_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| not_found("Upload not found"))
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
        let id = row.get("id");
        fs::remove_file(temporary_path(STORAGE_ROOT, id)).await.ok();
        fs::remove_file(temporary_path(MUSIC_ROOT, id)).await.ok();
    }
}

pub(crate) async fn prepare_temporary_upload(
    storage_root: &str,
    destination: &Path,
    length: u64,
) -> Result<(Uuid, PathBuf, i64), ApiError> {
    if length > MAX_UPLOAD_SIZE {
        return Err(bad_request("Upload exceeds the maximum size"));
    }
    match fs::metadata(destination).await {
        Ok(_) => return Err(conflict("A file with this name already exists")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(server_error()),
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }
    fs::create_dir_all(Path::new(storage_root).join(TEMP_DIRECTORY))
        .await
        .map_err(|_| server_error())?;

    let id = Uuid::new_v4();
    let temporary = temporary_path(storage_root, id);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .await
        .map_err(|_| server_error())?;
    Ok((id, temporary, as_i64(length)?))
}

pub(crate) async fn create_user_upload(
    pool: &Pool,
    user_id: Uuid,
    storage_root: &str,
    headers: &TusHeaders,
    location_prefix: &str,
) -> Result<TusResponse, ApiError> {
    let length = headers.upload_length()?;
    let (target_path, destination) = destination_from_metadata(storage_root, headers.metadata()?)?;
    let (id, temporary, length) =
        prepare_temporary_upload(storage_root, &destination, length).await?;
    let client = get_client(pool).await?;
    let inserted = client
        .execute(
            "INSERT INTO upload_sessions (id, user_id, target_path, upload_length, expires_at)
             VALUES ($1, $2, $3, $4, NOW() + INTERVAL '24 hours')",
            &[&id, &user_id, &target_path, &length],
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
    Ok(TusResponse::created(format!("{location_prefix}/{id}")))
}

pub(crate) async fn head_user_upload(
    pool: &Pool,
    user_id: Uuid,
    id: Uuid,
) -> Result<TusResponse, ApiError> {
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()",
            &[&id, &user_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    head_response(&row)
}

pub(crate) fn head_response(row: &Row) -> Result<TusResponse, ApiError> {
    Ok(TusResponse::head(
        as_u64(row.get("upload_offset"))?,
        as_u64(row.get("upload_length"))?,
    ))
}

pub(crate) fn patch_headers(headers: &TusHeaders) -> Result<(u64, u64), ApiError> {
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
    Ok((requested_offset, content_length))
}

pub(crate) fn upload_progress(
    row: &Row,
    requested_offset: u64,
    content_length: u64,
) -> Result<UploadProgress, ApiError> {
    let progress = UploadProgress {
        target_path: row.get("target_path"),
        length: as_u64(row.get("upload_length"))?,
        offset: as_u64(row.get("upload_offset"))?,
    };
    if progress.offset == progress.length {
        return Err(conflict("Upload is already complete"));
    }
    if requested_offset != progress.offset {
        return Err(conflict("Upload offset does not match the server offset"));
    }
    let remaining = progress
        .length
        .checked_sub(progress.offset)
        .ok_or_else(server_error)?;
    if content_length > remaining {
        return Err(bad_request("Upload chunk exceeds the declared file size"));
    }
    Ok(progress)
}

pub(crate) async fn write_upload_chunk(
    storage_root: &str,
    id: Uuid,
    offset: u64,
    content_length: u64,
    data: Data<'_>,
) -> Result<u64, ApiError> {
    let temporary = temporary_path(storage_root, id);
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
    offset.checked_add(written).ok_or_else(server_error)
}

pub(crate) async fn patch_user_upload(
    pool: &Pool,
    user_id: Uuid,
    storage_root: &str,
    headers: &TusHeaders,
    id: Uuid,
    data: Data<'_>,
) -> Result<(TusResponse, Option<String>), ApiError> {
    let (requested_offset, content_length) = patch_headers(headers)?;
    let mut client = get_client(pool).await?;
    let transaction = client.transaction().await.map_err(db_error)?;
    let row = transaction
        .query_opt(
            "SELECT target_path, upload_length, upload_offset
             FROM upload_sessions
             WHERE id = $1 AND user_id = $2 AND expires_at > NOW()
             FOR UPDATE",
            &[&id, &user_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    let progress = upload_progress(&row, requested_offset, content_length)?;
    let next_offset =
        write_upload_chunk(storage_root, id, progress.offset, content_length, data).await?;
    transaction
        .execute(
            "UPDATE upload_sessions SET upload_offset = $1 WHERE id = $2 AND user_id = $3",
            &[&as_i64(next_offset)?, &id, &user_id],
        )
        .await
        .map_err(db_error)?;
    transaction.commit().await.map_err(db_error)?;
    let completed = (next_offset == progress.length).then_some(progress.target_path);
    Ok((TusResponse::patched(next_offset), completed))
}

pub(crate) async fn finalize_user_upload(
    pool: &Pool,
    storage_root: &str,
    id: Uuid,
    user_id: Uuid,
    target_path: &str,
) -> Result<(), ApiError> {
    let temporary = temporary_path(storage_root, id);
    let destination = Path::new(storage_root).join(target_path);
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
                eprintln!("Completed upload {id} could not be removed from session table: {error}");
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

pub(crate) async fn terminate_user_upload(
    pool: &Pool,
    storage_root: &str,
    user_id: Uuid,
    id: Uuid,
) -> Result<TusResponse, ApiError> {
    let client = get_client(pool).await?;
    client
        .query_opt(
            "DELETE FROM upload_sessions WHERE id = $1 AND user_id = $2 RETURNING id",
            &[&id, &user_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Upload not found"))?;
    fs::remove_file(temporary_path(storage_root, id)).await.ok();
    Ok(TusResponse::terminated())
}
