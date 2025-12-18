#[macro_use]
extern crate rocket;

mod api;
mod frontend;
mod files;
mod shared;

use rocket::fs::{relative, FileServer};
use crate::api::{list_directory, list_root};
use crate::files::{download};
use crate::frontend::{frontend_fallback};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![list_root, list_directory])
        .mount("/files", routes![download])
        .mount("/", FileServer::from(relative!["static"]))
        .mount("/", routes![frontend_fallback])
}
