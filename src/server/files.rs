use crate::guards::{AuthenticatedUser, check_permission};
use crate::shared::{
    FileResponse, STORAGE_ROOT, bad_request, conflict, forbidden, not_found, sanitize_path,
    server_error,
};
use deadpool_postgres::Pool;
use rocket::State;
use rocket::data::{Data, ToByteUnit};
use rocket::form::FromForm;
use rocket::futures::{SinkExt, StreamExt};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_ws as ws;
use rocket_ws::Message;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// Query parameters for chunked uploads — `?offset=<start>`.
#[derive(FromForm)]
pub struct ChunkQuery {
    offset: Option<u64>,
}

#[get("/files/<path..>")]
pub async fn download(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<FileResponse, Status> {
    let has_perm = check_permission(pool, user.id, "download_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(Status::Forbidden);
    }
    let safe_path = sanitize_path(path).ok_or(Status::BadRequest)?;
    let full_path = Path::new(STORAGE_ROOT).join(safe_path);

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    if !metadata.is_file() {
        return Err(Status::NotFound);
    }

    let canonical = fs::canonicalize(&full_path)
        .await
        .map_err(|_| Status::NotFound)?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !canonical.starts_with(&canonical_root) {
        return Err(Status::Forbidden);
    }

    if let Some(filename) = canonical.file_name()
        && filename.to_string_lossy().starts_with('.')
    {
        return Err(Status::Forbidden);
    }

    let file = File::open(&canonical).await.map_err(|_| Status::NotFound)?;
    let file_size = metadata.len();

    Ok(FileResponse {
        stream: Box::new(file),
        size: file_size,
    })
}

/// Upload a file or chunk — PUT /api/files/<path..>?offset=<start>
///
/// Full-file upload (no offset): sends entire body. Conflicts with existing files.
/// Chunked upload (with offset): writes chunk at the given byte position.
///   The file is created on first chunk, opened for write on subsequent chunks.
#[put("/files/<path..>?<chunk..>", data = "<data>")]
pub async fn upload(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
    data: Data<'_>,
    chunk: ChunkQuery,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let has_perm = check_permission(pool, user.id, "upload_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(forbidden());
    }

    let safe_path = sanitize_path(path).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }
    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }

    if let Some(start) = chunk.offset {
        // Chunked upload — write at the given byte offset
        let file_exists = fs::metadata(&full_path).await.is_ok();

        let mut file = if file_exists {
            fs::OpenOptions::new()
                .write(true)
                .open(&full_path)
                .await
                .map_err(|_| server_error())?
        } else {
            File::create(&full_path).await.map_err(|_| server_error())?
        };

        file.seek(SeekFrom::Start(start))
            .await
            .map_err(|_| server_error())?;

        let write_result = data
            .open((10_u64 * 1024 * 1024).into())
            .stream_to(&mut file)
            .await;

        file.flush().await.map_err(|_| server_error())?;
        drop(file);

        match write_result {
            Ok(written) => Ok(Json(serde_json::json!({
                "success": true,
                "written": *written,
            }))),
            Err(_) => {
                // Client likely disconnected — clean up partial file
                fs::remove_file(&full_path).await.ok();
                Err(server_error())
            }
        }
    } else {
        // Full-file upload — backwards compatible
        if fs::metadata(&full_path).await.is_ok() {
            let filename = full_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            return Err(conflict(&format!("File '{}' already exists", filename)));
        }

        let mut file = File::create(&full_path).await.map_err(|_| server_error())?;

        data.open(10.gibibytes())
            .stream_to(&mut file)
            .await
            .map_err(|_| server_error())?;

        Ok(Json(serde_json::json!({"success": true})))
    }
}

/// Check upload progress — HEAD /api/files/<path..>
/// Returns 206 if the file exists, 404 if not started.
#[head("/files/<path..>")]
pub async fn upload_progress(
    _pool: &State<Pool>,
    _user: AuthenticatedUser,
    path: PathBuf,
) -> Result<Status, (Status, Json<serde_json::Value>)> {
    let safe_path = sanitize_path(path).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }
    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    match fs::metadata(&full_path).await {
        Ok(meta) if meta.is_file() => Ok(Status::PartialContent),
        Ok(_) => Err(not_found("Not a file")),
        Err(_) => Ok(Status::NotFound),
    }
}

/// Delete a file or directory — DELETE /api/files/<path..>
#[delete("/files/<path..>")]
pub async fn delete_path(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    path: PathBuf,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let has_perm = check_permission(pool, user.id, "delete_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(forbidden());
    }

    let safe_path = sanitize_path(path).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }

    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    let canonical = fs::canonicalize(&full_path)
        .await
        .map_err(|_| not_found("File not found"))?;
    let canonical_root = fs::canonicalize(STORAGE_ROOT)
        .await
        .map_err(|_| server_error())?;

    if canonical == canonical_root {
        return Err(bad_request("Cannot delete storage root"));
    }
    if !canonical.starts_with(&canonical_root) {
        return Err(forbidden());
    }

    let metadata = fs::metadata(&full_path)
        .await
        .map_err(|_| not_found("File not found"))?;

    if metadata.is_dir() {
        fs::remove_dir_all(&canonical)
            .await
            .map_err(|_| server_error())?;
    } else {
        fs::remove_file(&canonical)
            .await
            .map_err(|_| server_error())?;
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// WebSocket upload — `GET /api/upload?<path>`
///
/// Streams a single file over a WebSocket:
///   client → server: text `{"type":"init","size":N,"mime":...}`
///                  then N binary frames of bytes,
///                  then text `{"type":"end"}`.
///   server → client: text `{"type":"ack","bytes":M}` after each write,
///                     text `{"type":"done","bytes":M}` on flush,
///                     text `{"type":"error","message":...}` on failure.
///
/// Auth is via the accessToken cookie (browsers send cookies on the WS
/// handshake for the same origin). The `AuthenticatedUser` guard validates it
/// before the route runs, so the handshake is rejected with 401 if absent.
/// Permission and a file-conflict check happen synchronously, before the
/// connection upgrades, so errors return as normal HTTP statuses.
/// On client disconnect without an "end" the partial file is removed (matches
/// the chunked-PUT behaviour in `upload`).
#[get("/upload?<path>")]
pub async fn upload_ws(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    ws: ws::WebSocket,
    path: String,
) -> Result<ws::Channel<'static>, (Status, Json<serde_json::Value>)> {
    let has_perm = check_permission(pool, user.id, "upload_files")
        .await
        .unwrap_or(false);
    if !has_perm {
        return Err(forbidden());
    }

    let safe_path = sanitize_path(PathBuf::from(path)).ok_or(bad_request("Invalid path"))?;
    if safe_path.as_os_str().is_empty() {
        return Err(bad_request("Path cannot be empty"));
    }
    let full_path = Path::new(STORAGE_ROOT).join(&safe_path);

    if fs::metadata(&full_path).await.is_ok() {
        let filename = full_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        return Err(conflict(&format!("File '{}' already exists", filename)));
    }

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| server_error())?;
    }

    Ok(ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut file = match File::create(&full_path).await {
                Ok(f) => f,
                Err(_) => {
                    let _ = stream
                        .send(Message::text(
                            r#"{"type":"error","message":"Failed to create file"}"#,
                        ))
                        .await;
                    return Ok(());
                }
            };

            let mut written: u64 = 0;

            while let Some(msg) = stream.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(_) => break,
                };

                match msg {
                    Message::Text(s) => {
                        let v: serde_json::Value = match serde_json::from_str(&s) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        if v.get("type").and_then(|t| t.as_str()) == Some("end") {
                            if file.flush().await.is_err() {
                                let _ = stream
                                    .send(Message::text(
                                        r#"{"type":"error","message":"flush failed"}"#,
                                    ))
                                    .await;
                                fs::remove_file(&full_path).await.ok();
                                return Ok(());
                            }
                            let _ = stream
                                .send(Message::text(format!(
                                    r#"{{"type":"done","bytes":{}}}"#,
                                    written
                                )))
                                .await;
                            return Ok(());
                        }
                        // "init" carries advisory metadata (size/mime); nothing to do here.
                    }
                    Message::Binary(bytes) => {
                        if file.write_all(&bytes).await.is_err() {
                            let _ = stream
                                .send(Message::text(
                                    r#"{"type":"error","message":"write failed"}"#,
                                ))
                                .await;
                            fs::remove_file(&full_path).await.ok();
                            return Ok(());
                        }
                        written += bytes.len() as u64;
                        let _ = stream
                            .send(Message::text(format!(
                                r#"{{"type":"ack","bytes":{}}}"#,
                                written
                            )))
                            .await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            // Stream ended via close/disconnect without an explicit "end":
            // remove the partial file, matching the chunked-PUT cleanup behaviour.
            fs::remove_file(&full_path).await.ok();
            Ok(())
        })
    }))
}
