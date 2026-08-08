use crate::auth::{AuthenticatedUser, require_permission};
use crate::shared::{
    ApiError, MUSIC_ROOT, bad_request, db_error, get_client, not_found, server_error, status_error,
};
use deadpool_postgres::Pool;
use image::{ImageFormat, ImageReader};
use lofty::config::WriteOptions;
use lofty::picture::{Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;
use rocket::data::ToByteUnit;
use rocket::http::{Header, Status};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::{Data, Request, State};
use std::fmt;
use std::io::Cursor;
use std::path::Path;
use uuid::Uuid;

const MAX_DECODED_PIXELS: u64 = 100_000_000;

pub(crate) struct EmbeddedArtwork {
    pub(crate) bytes: Vec<u8>,
    pub(crate) content_type: String,
}

#[derive(Debug)]
pub(crate) enum ArtworkError {
    Missing,
    InvalidImage,
    File(String),
}

impl fmt::Display for ArtworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("Cover art not found"),
            Self::InvalidImage => formatter.write_str("Invalid cover image"),
            Self::File(message) => formatter.write_str(message),
        }
    }
}

fn primary_picture(tag: &Tag) -> Option<&Picture> {
    tag.get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first())
}

pub(crate) fn has_embedded_artwork(tag: Option<&Tag>) -> bool {
    tag.and_then(primary_picture).is_some()
}

fn format_content_type(format: ImageFormat, picture: &Picture) -> String {
    let known = match format {
        ImageFormat::Jpeg => Some("image/jpeg"),
        ImageFormat::Png => Some("image/png"),
        ImageFormat::Gif => Some("image/gif"),
        ImageFormat::WebP => Some("image/webp"),
        ImageFormat::Bmp => Some("image/bmp"),
        ImageFormat::Tiff => Some("image/tiff"),
        ImageFormat::Ico => Some("image/x-icon"),
        _ => None,
    };

    known
        .map(str::to_owned)
        .or_else(|| picture.mime_type().map(|mime| mime.as_str().to_owned()))
        .unwrap_or_else(|| "application/octet-stream".to_owned())
}

fn resize_artwork(bytes: &[u8], size: u32) -> Result<(Vec<u8>, ImageFormat), ArtworkError> {
    let format = image::guess_format(bytes).map_err(|_| ArtworkError::InvalidImage)?;
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| ArtworkError::InvalidImage)?;

    if u64::from(width) * u64::from(height) > MAX_DECODED_PIXELS {
        return Err(ArtworkError::InvalidImage);
    }
    if width <= size && height <= size {
        return Ok((bytes.to_vec(), format));
    }

    let image = ImageReader::with_format(Cursor::new(bytes), format)
        .decode()
        .map_err(|_| ArtworkError::InvalidImage)?;
    let resized = image.thumbnail(size, size);
    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, format)
        .map_err(|_| ArtworkError::InvalidImage)?;
    Ok((output.into_inner(), format))
}

pub(crate) fn read_embedded_artwork(
    path: &Path,
    size: Option<u32>,
) -> Result<EmbeddedArtwork, ArtworkError> {
    let tagged_file = Probe::open(path)
        .map_err(|error| ArtworkError::File(format!("Cannot open music file: {error}")))?
        .guess_file_type()
        .map_err(|error| ArtworkError::File(format!("Cannot determine file type: {error}")))?
        .read()
        .map_err(|error| ArtworkError::File(format!("Cannot read music tags: {error}")))?;
    let picture = tagged_file
        .primary_tag()
        .and_then(primary_picture)
        .ok_or(ArtworkError::Missing)?;

    let (bytes, format) = match size {
        Some(size) => resize_artwork(picture.data(), size)?,
        None => {
            let format =
                image::guess_format(picture.data()).map_err(|_| ArtworkError::InvalidImage)?;
            (picture.data().to_vec(), format)
        }
    };

    Ok(EmbeddedArtwork {
        content_type: format_content_type(format, picture),
        bytes,
    })
}

pub(crate) fn write_embedded_artwork(path: &Path, bytes: Vec<u8>) -> Result<(), ArtworkError> {
    let mut reader = Cursor::new(bytes);
    let mut picture = Picture::from_reader(&mut reader).map_err(|_| ArtworkError::InvalidImage)?;
    picture.set_pic_type(PictureType::CoverFront);

    let mut tagged_file = Probe::open(path)
        .map_err(|error| ArtworkError::File(format!("Cannot open music file: {error}")))?
        .guess_file_type()
        .map_err(|error| ArtworkError::File(format!("Cannot determine file type: {error}")))?
        .read()
        .map_err(|error| ArtworkError::File(format!("Cannot read music tags: {error}")))?;
    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| ArtworkError::File("No writable tag found".to_owned()))?;

    tag.remove_picture_type(PictureType::CoverFront);
    tag.push_picture(picture);
    tag.save_to_path(path, WriteOptions::default())
        .map_err(|error| ArtworkError::File(format!("Cannot write music tags: {error}")))
}

pub(crate) struct ArtworkResponse {
    artwork: EmbeddedArtwork,
}

impl ArtworkResponse {
    pub(crate) fn new(artwork: EmbeddedArtwork) -> Self {
        Self { artwork }
    }
}

impl<'r> Responder<'r, 'static> for ArtworkResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let length = self.artwork.bytes.len();
        rocket::Response::build()
            .header(Header::new("Content-Type", self.artwork.content_type))
            .header(Header::new(
                "Cache-Control",
                "private, max-age=3600, must-revalidate",
            ))
            .sized_body(length, Cursor::new(self.artwork.bytes))
            .ok()
    }
}

fn artwork_size(size: Option<i32>) -> Result<Option<u32>, ApiError> {
    match size {
        Some(value @ 1..=4096) => Ok(Some(value as u32)),
        Some(_) => Err(bad_request("Cover size must be between 1 and 4096")),
        None => Ok(None),
    }
}

async fn song_artwork_path(
    pool: &Pool,
    song_id: Uuid,
    require_cover: bool,
) -> Result<(deadpool_postgres::Object, std::path::PathBuf), ApiError> {
    let client = get_client(pool).await?;
    let row = client
        .query_opt(
            "SELECT file_path, has_cover_art FROM songs WHERE id = $1",
            &[&song_id],
        )
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Song not found"))?;

    if require_cover && !row.get::<_, bool>("has_cover_art") {
        return Err(not_found("Cover art not found"));
    }

    let file_path: String = row.get("file_path");
    Ok((client, Path::new(MUSIC_ROOT).join(file_path)))
}

async fn load_artwork(
    path: std::path::PathBuf,
    size: Option<u32>,
) -> Result<EmbeddedArtwork, ApiError> {
    match tokio::task::spawn_blocking(move || read_embedded_artwork(&path, size)).await {
        Ok(Ok(artwork)) => Ok(artwork),
        Ok(Err(ArtworkError::Missing | ArtworkError::InvalidImage)) => {
            Err(not_found("Cover art not found"))
        }
        Ok(Err(error)) => {
            eprintln!("Failed to read embedded cover art: {error}");
            Err(server_error())
        }
        Err(error) => {
            eprintln!("Cover art task failed: {error}");
            Err(server_error())
        }
    }
}

#[get("/music/songs/<id>/cover?<size>")]
pub(crate) async fn get_song_cover(
    pool: &State<Pool>,
    _user: AuthenticatedUser,
    id: &str,
    size: Option<i32>,
) -> Result<ArtworkResponse, ApiError> {
    let song_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid song ID"))?;
    let size = artwork_size(size)?;
    let (_, path) = song_artwork_path(pool, song_id, true).await?;
    let artwork = load_artwork(path, size).await?;
    Ok(ArtworkResponse::new(artwork))
}

#[put("/music/songs/<id>/cover", data = "<data>")]
pub(crate) async fn update_song_cover(
    pool: &State<Pool>,
    user: AuthenticatedUser,
    id: &str,
    data: Data<'_>,
) -> Result<Json<super::SongResponse>, ApiError> {
    require_permission(pool, user.id, "music_edit_tags").await?;
    let song_id = Uuid::parse_str(id).map_err(|_| not_found("Invalid song ID"))?;
    let (client, path) = song_artwork_path(pool, song_id, false).await?;

    let bytes = data
        .open(10.mebibytes())
        .into_bytes()
        .await
        .map_err(|_| server_error())?;
    if !bytes.is_complete() {
        return Err(status_error(
            Status::PayloadTooLarge,
            "Cover image must be 10 MiB or smaller",
        ));
    }
    let bytes = bytes.into_inner();
    if bytes.is_empty() {
        return Err(bad_request("Cover image is empty"));
    }

    match tokio::task::spawn_blocking(move || write_embedded_artwork(&path, bytes)).await {
        Ok(Ok(())) => {}
        Ok(Err(ArtworkError::InvalidImage)) => return Err(bad_request("Invalid cover image")),
        Ok(Err(error)) => {
            eprintln!("Failed to write embedded cover art: {error}");
            return Err(server_error());
        }
        Err(error) => {
            eprintln!("Cover art task failed: {error}");
            return Err(server_error());
        }
    }

    client
        .execute(
            "UPDATE songs SET has_cover_art = TRUE, updated_at = NOW() WHERE id = $1",
            &[&song_id],
        )
        .await
        .map_err(db_error)?;
    let row = client
        .query_one("SELECT * FROM songs WHERE id = $1", &[&song_id])
        .await
        .map_err(db_error)?;
    Ok(Json(super::row_to_song(&row)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_preserves_aspect_ratio_and_does_not_upscale() {
        let source = image::DynamicImage::new_rgb8(4, 2);
        let mut encoded = Cursor::new(Vec::new());
        source
            .write_to(&mut encoded, ImageFormat::Png)
            .expect("test image should encode");
        let encoded = encoded.into_inner();

        let (resized, format) = resize_artwork(&encoded, 2).expect("image should resize");
        let dimensions = ImageReader::with_format(Cursor::new(resized), format)
            .into_dimensions()
            .expect("resized image should decode");
        assert_eq!(dimensions, (2, 1));

        let (original, _) = resize_artwork(&encoded, 8).expect("small image should remain valid");
        assert_eq!(original, encoded);
    }
}
