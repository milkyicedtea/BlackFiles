use crate::models::PaginationParams;
use rocket::Request;
use rocket::http::Header;
use rocket::response::Responder;
use rocket::serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncRead;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub(crate) struct FileEntry {
    pub(crate) name: String,
    pub(crate) is_dir: bool,
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) modified: u64,
}

pub(crate) struct FileResponse {
    pub(crate) stream: Box<dyn AsyncRead + Send + Unpin>,
    pub(crate) size: u64,
}

impl<'r> Responder<'r, 'static> for FileResponse {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        rocket::Response::build()
            .header(Header::new("Content-Length", self.size.to_string()))
            .streamed_body(self.stream)
            .ok()
    }
}

pub(crate) fn sanitize_path(path: PathBuf) -> Option<PathBuf> {
    if path.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }

    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let value = part.to_string_lossy();
                if value.contains('\0') || value == "." || value == ".." {
                    return None;
                }
                clean.push(part);
            }
            _ => return None,
        }
    }

    (!clean.as_os_str().is_empty()).then_some(clean)
}

pub(crate) fn path_to_web_string(path: &Path) -> String {
    let mut result = String::new();
    for component in path.iter() {
        if !result.is_empty() {
            result.push('/');
        }
        result.push_str(&component.to_string_lossy());
    }
    result
}

pub(crate) fn filter_by_search_term(pagination: &PaginationParams, entries: &mut Vec<FileEntry>) {
    if let Some(search) = pagination
        .search
        .as_ref()
        .filter(|search| !search.is_empty())
    {
        let search = search.to_lowercase();
        entries.retain(|entry| entry.name.to_lowercase().contains(&search));
    }
}
