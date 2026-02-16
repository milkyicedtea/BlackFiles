use std::path::{Path, PathBuf};
use rocket::fs::{NamedFile};

// Catch-all for client-side routing - serves index.html from the built static folder
#[get("/<_path..>", rank = 20)]
pub async fn frontend_fallback(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/index.html")).await.ok()
}