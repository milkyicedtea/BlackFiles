// Re-export shared infrastructure for sub-modules.
pub(crate) use crate::auth::{AuthenticatedUser, require_permission};
pub(crate) use crate::shared::*;
pub(crate) use deadpool_postgres::Pool;
pub(crate) use lofty::prelude::*;
pub(crate) use rocket::State;
pub(crate) use rocket::http::Status;
pub(crate) use rocket::serde::json::Json;
pub(crate) use rocket::serde::{Deserialize, Serialize};
pub(crate) use tokio::fs;

// Sub-modules
pub(crate) mod artwork;
pub(crate) mod crud;
pub(crate) mod library;
pub(crate) mod tags;
pub(crate) mod upload;

// Re-exports for parent (main.rs)
pub(crate) use artwork::*;
pub(crate) use {crud::*, library::*, tags::*, upload::*};
