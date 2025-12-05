use std::path::PathBuf;
use rocket::{Request, Response};
use rocket::http::Header;
use rocket::response::Responder;
use rocket::serde::Serialize;
use tokio::io::AsyncRead;

pub(crate) const STORAGE_ROOT: &str = "storage";

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub path: String,
    pub size: u64,
    pub modified: u64, // unix time
}

pub(crate) struct FileResponse {
    pub stream: Box<dyn AsyncRead + Send + Unpin>,
    pub size: u64,
}

impl<'r> Responder<'r, 'static> for FileResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .header(Header::new("Content-Length", self.size.to_string()))
            .streamed_body(self.stream)
            .ok()
    }
}

pub(crate) fn sanitize_path(path: PathBuf) -> Option<PathBuf> {
    // allow empty paths for root directory
    if path.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }

    let mut clean = PathBuf::new();

    for part in path.components() {
        match part {
            std::path::Component::Normal(p) => {
                // additional check to reject null bytes or looks sus
                let s = p.to_string_lossy();
                if s.contains('\0') || s == "." || s == ".." {
                    return None;
                }
                clean.push(p);
            }
            _ => return None,
        }
    }

    // reject empty paths
    if clean.as_os_str().is_empty() {
        return None;
    }

    Some(clean)
}
