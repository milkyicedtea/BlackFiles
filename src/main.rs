#[macro_use]
extern crate rocket;

mod api;
mod frontend;
mod files;
mod shared;

use crate::api::{list_directory, list_root};
use crate::files::{download};
use crate::frontend::{frontend_fallback, serve_css, serve_js};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![list_root, list_directory])
        .mount("/files", routes![download])
        .mount("/", routes![serve_css, serve_js, frontend_fallback])
}

