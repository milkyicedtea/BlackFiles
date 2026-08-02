// Re-export shared infrastructure for submodules.
pub(crate) use crate::auth::{AuthenticatedUser, require_permission};
pub(crate) use crate::models::*;
pub(crate) use crate::shared::*;
pub(crate) use deadpool_postgres::Pool;
pub(crate) use rocket::State;
pub(crate) use rocket::http::Status;
pub(crate) use rocket::serde::json::Json;
pub(crate) use std::path::Path;

// Submodules
pub(crate) mod delete;
pub(crate) mod download;
pub(crate) mod folder;
pub(crate) mod helpers;
pub(crate) mod list;
pub(crate) mod rename;
pub(crate) mod tus;
pub(crate) mod upload_links;

// Re-exports for parent (main.rs) - explicit for modules with name collisions.
// Re-exports for parent (main.rs)
pub(crate) use {delete::*, folder::*, helpers::*, list::*, rename::*, tus::*, upload_links::*};
// download is re-exported via its module path - see main.rs.
