#[macro_use]
extern crate rocket;

mod api;
mod frontend;
mod files;
mod shared;
mod auth;

use rocket::fs::{FileServer};
use crate::api::{list_directory, list_root};
use crate::auth::{check_auth, login};
use crate::files::{download};
use crate::frontend::{frontend_fallback};

fn prepare_dirs() {
    // create storage directory if it doesn't exist
    std::fs::create_dir_all(shared::STORAGE_ROOT).ok();

    // create static directory if it doesn't exist
    std::fs::create_dir_all(shared::STATIC_ROOT).ok();
}

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();

    prepare_dirs();

    rocket::build()
        .mount("/api", routes![check_auth, login, list_root, list_directory])
        .mount("/files", routes![download])
        .mount("/", FileServer::from("static"))
        .mount("/", routes![frontend_fallback])
}
