// Re-export shared infrastructure for submodules.
pub(crate) use crate::shared::{MUSIC_ROOT, sha256_hex, url_decode};
pub(crate) use argon2::PasswordVerifier;
pub(crate) use chrono::Utc;
pub(crate) use deadpool_postgres::Pool;
pub(crate) use rocket::State;
pub(crate) use rocket::form::FromForm;
pub(crate) use rocket::http::Status;
pub(crate) use rocket::http::{ContentType, Header};
pub(crate) use rocket::request::{FromRequest, Outcome, Request};
pub(crate) use rocket::serde::Serialize;
pub(crate) use rocket::serde::json::Json;
pub(crate) use std::collections::HashSet;
pub(crate) use std::path::Path;
pub(crate) use tokio::fs::File;
pub(crate) use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
pub(crate) use uuid::Uuid;

// Submodules
pub(crate) mod browse;
pub(crate) mod envelope;
pub(crate) mod guards;
pub(crate) mod media;
pub(crate) mod playlists;
pub(crate) mod scrobble_api;
pub(crate) mod shared;
pub(crate) mod starred;
pub(crate) mod system;

// Re-exports for parent (main.rs)
pub(crate) use {
    browse::*, envelope::*, guards::*, media::*, playlists::*, scrobble_api::*, shared::*,
    starred::*, system::*,
};
