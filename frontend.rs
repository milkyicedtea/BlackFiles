use std::path::{Path, PathBuf};
use rocket::fs::{relative, NamedFile};
use rocket::http::ContentType;

// Serve static CSS files
#[get("/css/<file..>")]
pub(crate) async fn serve_css(file: PathBuf) -> Option<(ContentType, NamedFile)> {
    let named_file = NamedFile::open(Path::new(relative!("src/frontend/css")).join(file)).await.ok()?;
    Some((ContentType::CSS, named_file))
}

// Serve static JS files
#[get("/js/<file..>")]
pub(crate) async fn serve_js(file: PathBuf) -> Option<(ContentType, NamedFile)> {
    let named_file = NamedFile::open(Path::new(relative!("src/frontend/js")).join(file)).await.ok()?;
    Some((ContentType::JavaScript, named_file))
}

// Catch-all for client-side routing - serves index.html
#[get("/<_path..>", rank = 10)]
pub(crate) async fn frontend_fallback(_path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(relative!("src/frontend")).join("index.html")).await.ok()
}