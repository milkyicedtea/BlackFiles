use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

// Catch-all for client-side routing - serves index.html from the built public folder
#[get("/<path..>", rank = 20)]
pub async fn frontend_fallback(path: PathBuf) -> Option<NamedFile> {
    if path.starts_with("api") || path.extension().is_some() {
        return None;
    }

    NamedFile::open(Path::new(crate::shared::BUILD_ROOT).join("index.html"))
        .await
        .ok()
}
