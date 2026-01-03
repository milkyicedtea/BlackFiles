#[macro_use]
extern crate rocket;

mod api;
mod frontend;
mod files;
mod shared;
mod auth;

use rocket::fs::{relative, FileServer};
use crate::api::{list_directory, list_root};
use crate::auth::{check_auth, login};
use crate::files::{download};
use crate::frontend::{frontend_fallback};

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();

    rocket::build()
        .mount("/api", routes![check_auth, login, list_root, list_directory])
        .mount("/files", routes![download])
        .mount("/", FileServer::from(relative!["static"]))
        .mount("/", routes![frontend_fallback])
}
