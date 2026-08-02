use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

fn is_frontend_path(path: &Path) -> bool {
    !path.starts_with("api") && !path.starts_with("rest") && path.extension().is_none()
}

// Catch-all for client-side routing - serves index.html from the built public folder
#[get("/<path..>", rank = 20)]
pub async fn frontend_fallback(path: PathBuf) -> Option<NamedFile> {
    if !is_frontend_path(&path) {
        return None;
    }

    NamedFile::open(Path::new(crate::shared::BUILD_ROOT).join("index.html"))
        .await
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_routes_do_not_fall_back_to_the_frontend() {
        assert!(!is_frontend_path(Path::new("api/missing")));
        assert!(!is_frontend_path(Path::new("rest/getBookmarks")));
        assert!(!is_frontend_path(Path::new("assets/app.js")));
        assert!(is_frontend_path(Path::new("music/library")));
    }
}
