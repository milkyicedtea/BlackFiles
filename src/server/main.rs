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

    // create build directory if it doesn't exist (it always should, but just in case)
    std::fs::create_dir_all(shared::BUILD_ROOT).ok();
}

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();

    prepare_dirs();

    rocket::build()
        .mount("/api", routes![check_auth, login, list_root, list_directory])
        .mount("/files", routes![download])
        .mount("/", FileServer::from(shared::BUILD_ROOT))
        .mount("/", routes![frontend_fallback])
}
