use crate::shared;
use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

// Catch-all for client-side routing - serves index.html from the built public folder
#[get("/<_path..>", rank = 20)]
pub async fn frontend_fallback(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(shared::BUILD_ROOT).join("index.html"))
        .await
        .ok()
}
