use std::path::{Path};
use rocket::http::{Status, ContentType};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::{self, Responder, Response};
use std::io::Cursor;
use rocket::tokio::fs;
use rocket::serde::Serialize;
use rocket::serde::json::serde_json;
use lofty::prelude::*;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use std::cmp::Ordering;

// --- Authentication & Context ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubsonicFormat {
    Xml,
    Json,
}

pub struct SubsonicContext {
    pub authorized: bool,
    pub format: SubsonicFormat,
}

#[derive(Debug)]
pub enum SubsonicError {
    _AuthFailed,
    _InvalidParameter,
    _NotFound,
    _Generic(String),
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SubsonicContext {
    type Error = SubsonicError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let u = req.query_value::<String>("u").and_then(|r| r.ok());
        let t = req.query_value::<String>("t").and_then(|r| r.ok());
        let s = req.query_value::<String>("s").and_then(|r| r.ok());
        let f = req.query_value::<String>("f").and_then(|r| r.ok());

        let format = match f.as_deref() {
            Some("json") => SubsonicFormat::Json,
            _ => SubsonicFormat::Xml,
        };

        let env_token = std::env::var("BLACKFILES_TOKEN").unwrap_or_default();

        // 1. Password/Token check
        // Subsonic supports raw password in 'p' (legacy) or token 't' + salt 's'
        // token = md5(password + salt)

        let mut authorized = false;

        if let (Some(_user), Some(token), Some(salt)) = (&u, t, s) {
             let expected = format!("{}{}", env_token, salt);
             let digest = md5::compute(expected);
             let expected_token = format!("{:x}", digest);

             if token == expected_token {
                 authorized = true;
             }
        }

        // internal check for 'p' (no encryption) - discouraged but some clients use it
        if !authorized {
            if let Some(p) = req.query_value::<String>("p").and_then(|r| r.ok()) {
                if p.starts_with("enc:") {
                     // hex encoded
                     if let Ok(decoded) = hex::decode(&p[4..]) {
                         if let Ok(s) = String::from_utf8(decoded) {
                             if s == env_token {
                                 authorized = true;
                             }
                         }
                     }
                } else if p == env_token {
                    authorized = true;
                }
            }
        }

        Outcome::Success(SubsonicContext { authorized, format })
    }
}

// --- Response Helper ---

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct JsonResponseWrapper<T> {
    #[serde(rename = "subsonic-response")]
    response: JsonResponseBody<T>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct JsonResponseBody<T> {
    status: String,
    version: String,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonErrorBody>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct JsonErrorBody {
    code: i32,
    message: String,
}

pub enum SubsonicResponse<T> {
    XmlOk(String),
    XmlError(i32, String),
    JsonOk(Option<T>),
    JsonError(i32, String),
}

impl<'r, T: Serialize> Responder<'r, 'static> for SubsonicResponse<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        match self {
            SubsonicResponse::XmlOk(content) => {
                let body = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><subsonic-response xmlns=\"http://subsonic.org/restapi\" status=\"ok\" version=\"1.16.1\">{}</subsonic-response>",
                    content
                );
                Response::build()
                    .header(ContentType::XML)
                    .sized_body(body.len(), Cursor::new(body))
                    .ok()
            },
            SubsonicResponse::XmlError(code, message) => {
                 let body = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><subsonic-response xmlns=\"http://subsonic.org/restapi\" status=\"failed\" version=\"1.16.1\"><error code=\"{}\" message=\"{}\"/></subsonic-response>",
                    code, message
                );
                 Response::build()
                    .header(ContentType::XML)
                    .sized_body(body.len(), Cursor::new(body))
                    .ok()
            },
            SubsonicResponse::JsonOk(data) => {
                let wrapper = JsonResponseWrapper {
                    response: JsonResponseBody {
                        status: "ok".to_string(),
                        version: "1.16.1".to_string(),
                        data,
                        error: None
                    }
                };
                let json = serde_json::to_string(&wrapper).unwrap_or_default();
                Response::build()
                    .header(ContentType::JSON)
                    .sized_body(json.len(), Cursor::new(json))
                    .ok()
            },
            SubsonicResponse::JsonError(code, message) => {
                 let wrapper: JsonResponseWrapper<()> = JsonResponseWrapper {
                    response: JsonResponseBody {
                        status: "failed".to_string(),
                        version: "1.16.1".to_string(),
                        data: None,
                        error: Some(JsonErrorBody { code, message })
                    }
                };
                let json = serde_json::to_string(&wrapper).unwrap_or_default();
                Response::build()
                    .header(ContentType::JSON)
                    .sized_body(json.len(), Cursor::new(json))
                    .ok()
            }
        }
    }
}

// --- Data Structs for JSON ---

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFoldersList {
    #[serde(rename = "musicFolders")]
    pub music_folders: MusicFoldersInner
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFoldersInner {
    #[serde(rename = "musicFolder")]
    pub music_folder: Vec<MusicFolder>
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MusicFolder {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UserResponse {
    pub user: User,
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
#[allow(non_snake_case)]
pub struct User {
    pub username: String,
    pub email: String,
    pub scrobblingEnabled: bool,
    pub adminRole: bool,
    pub settingsRole: bool,
    pub downloadRole: bool,
    pub uploadRole: bool,
    pub playlistRole: bool,
    pub coverArtRole: bool,
    pub commentRole: bool,
    pub podcastRole: bool,
    pub streamRole: bool,
    pub jukeboxRole: bool,
    pub shareRole: bool,
    pub videoConversionRole: bool,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexesList {
    #[serde(rename = "indexes")]
    pub indexes: IndexesInner
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexesInner {
    #[serde(rename = "ignoredArticles")]
    pub ignored_articles: String,
    #[serde(rename = "index")]
    pub index: Vec<IndexItem>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct IndexItem {
    pub name: String,
    #[serde(rename = "artist")]
    pub artist: Vec<ArtistItem>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ArtistItem {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DirectoryRoot {
    #[serde(rename = "directory")]
    pub directory: DirectoryInner
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DirectoryInner {
    pub id: String,
    pub parent: Option<String>,
    pub name: String,
    #[serde(rename = "child")]
    pub child: Vec<DirectoryChild>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DirectoryChild {
    pub id: String,
    pub parent: String,
    pub title: String,
    #[serde(rename = "isDir")]
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<u32>,
    #[serde(rename = "coverArt")]
    pub cover_art: Option<String>,
}

// --- Response Helpers ---

fn respond<T: Serialize>(ctx: &SubsonicContext, xml: String, json: T) -> SubsonicResponse<T> {
     if !ctx.authorized {
        return SubsonicResponse::JsonError(40, "Wrong username or password".to_string());
    }
    match ctx.format {
        SubsonicFormat::Xml => SubsonicResponse::XmlOk(xml),
        SubsonicFormat::Json => SubsonicResponse::JsonOk(Some(json)),
    }
}

fn respond_error<T: Serialize>(ctx: &SubsonicContext, code: i32, msg: &str) -> SubsonicResponse<T> {
    match ctx.format {
        SubsonicFormat::Xml => SubsonicResponse::XmlError(code, msg.to_string()),
        SubsonicFormat::Json => SubsonicResponse::JsonError(code, msg.to_string()),
    }
}

// --- Endpoints ---

#[get("/ping.view")]
pub fn ping(ctx: SubsonicContext) -> SubsonicResponse<()> {
    if !ctx.authorized { return respond_error(&ctx, 40, "Wrong username or password"); }
    // Ping returns empty data on success
    match ctx.format {
        SubsonicFormat::Xml => SubsonicResponse::XmlOk("".to_string()),
        SubsonicFormat::Json => SubsonicResponse::JsonOk(Some(())),
    }
}

#[get("/getUser.view?<username>")]
pub fn get_user(username: String, ctx: SubsonicContext) -> SubsonicResponse<UserResponse> {
    if !ctx.authorized {
        return match ctx.format {
             SubsonicFormat::Xml => SubsonicResponse::XmlError(40, "Wrong username or password".to_string()),
             SubsonicFormat::Json => SubsonicResponse::JsonError(40, "Wrong username or password".to_string()),
        };
    }

    let user = User {
        username,
        email: "blackfiles@local".to_string(),
        scrobblingEnabled: true,
        adminRole: true,
        settingsRole: true,
        downloadRole: true,
        uploadRole: true,
        playlistRole: true,
        coverArtRole: true,
        commentRole: true,
        podcastRole: true,
        streamRole: true,
        jukeboxRole: true,
        shareRole: true,
        videoConversionRole: true,
    };

    match ctx.format {
        SubsonicFormat::Json => SubsonicResponse::JsonOk(Some(UserResponse { user })),
        SubsonicFormat::Xml => {
             let xml = format!(
                 r#"<user username="{}" email="{}" scrobblingEnabled="{}" adminRole="{}" settingsRole="{}" downloadRole="{}" uploadRole="{}" playlistRole="{}" coverArtRole="{}" commentRole="{}" podcastRole="{}" streamRole="{}" jukeboxRole="{}" shareRole="{}" videoConversionRole="{}"/>"#,
                 user.username, user.email, user.scrobblingEnabled, user.adminRole, user.settingsRole, user.downloadRole, user.uploadRole, user.playlistRole, user.coverArtRole, user.commentRole, user.podcastRole, user.streamRole, user.jukeboxRole, user.shareRole, user.videoConversionRole
             );
             SubsonicResponse::XmlOk(xml)
        }
    }
}

#[get("/getMusicFolders.view")]
pub fn get_music_folders(ctx: SubsonicContext) -> SubsonicResponse<MusicFoldersList> {
    let folders = MusicFoldersList {
        music_folders: MusicFoldersInner {
            music_folder: vec![MusicFolder { id: "root".to_string(), name: "Music".to_string() }]
        }
    };

    // Manual XML construction for consistency with previous implementation
    let xml = "<musicFolders><musicFolder id=\"root\" name=\"Music\"/></musicFolders>".to_string();

    respond(&ctx, xml, folders)
}

#[get("/getIndexes.view?<musicFolderId>&<ifModifiedSince>")]
#[allow(non_snake_case)]
pub async fn get_indexes(ctx: SubsonicContext, musicFolderId: Option<String>, ifModifiedSince: Option<i64>) -> SubsonicResponse<IndexesList> {
    if !ctx.authorized { return respond_error(&ctx, 40, "Wrong username or password"); }
    let _ = musicFolderId;
    let _ = ifModifiedSince;

    let root_path = Path::new(crate::shared::STORAGE_ROOT);

    // Data structures for JSON
    let mut indexes_map: std::collections::BTreeMap<String, Vec<ArtistItem>> = std::collections::BTreeMap::new();

    // Read dir
    match fs::read_dir(root_path).await {
        Ok(mut entries) => {
            let mut entries_vec = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                 entries_vec.push(entry);
            }
            entries_vec.sort_by_key(|e| e.file_name());

            for entry in entries_vec {
                let ft = entry.file_type().await.unwrap();
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let first_char = name.chars().next().unwrap_or('?').to_uppercase().to_string();

                    // ID is hex encoded path relative to storage
                    let rel_path = name.clone(); // Since it's root
                    let id = hex::encode(&rel_path);

                    indexes_map.entry(first_char).or_insert(Vec::new()).push(ArtistItem { id, name });
                }
            }
        }
        Err(_) => return respond_error(&ctx, 70, "Could not read music folder"),
    }

    // Construct XML and JSON
    let mut xml = String::from("<indexes ignoredArticles=\"The El La Los Las Le Les\">");
    let mut json_indexes = Vec::new();

    for (letter, artists) in indexes_map {
        xml.push_str(&format!("<index name=\"{}\">", letter));
        let mut json_artists = Vec::new();
        for artist in &artists {
            xml.push_str(&format!("<artist id=\"{}\" name=\"{}\" />", artist.id, artist.name));
            json_artists.push(ArtistItem { id: artist.id.clone(), name: artist.name.clone() });
        }
        xml.push_str("</index>");
        json_indexes.push(IndexItem { name: letter, artist: json_artists });
    }
    xml.push_str("</indexes>");

    let json_response = IndexesList {
        indexes: IndexesInner {
            ignored_articles: "The El La Los Las Le Les".to_string(),
            index: json_indexes,
        }
    };

    respond(&ctx, xml, json_response)
}

#[get("/getMusicDirectory.view?<id>")]
pub async fn get_music_directory(ctx: SubsonicContext, id: String) -> SubsonicResponse<DirectoryRoot> {
    if !ctx.authorized { return respond_error(&ctx, 40, "Wrong username or password"); }

    let rel_path_bytes = match hex::decode(&id) {
        Ok(p) => p,
        Err(_) => return respond_error(&ctx, 70, "Invalid ID"),
    };
    let rel_path = match String::from_utf8(rel_path_bytes) {
        Ok(s) => s,
        Err(_) => return respond_error(&ctx, 70, "Invalid ID encoding"),
    };

    if rel_path.contains("..") || rel_path.starts_with("/") || rel_path.starts_with("\\") {
         return respond_error(&ctx, 70, "Invalid Path");
    }

    let storage = Path::new(crate::shared::STORAGE_ROOT);
    let full_path = storage.join(&rel_path);

    let parent_path = Path::new(&rel_path).parent().unwrap_or(Path::new(""));
    let parent_id_str = if parent_path == Path::new("") {
         if rel_path.is_empty() { "".to_string() } else { "root".to_string() }
    } else {
        hex::encode(parent_path.to_string_lossy().to_string())
    };

    let name = Path::new(&rel_path).file_name().unwrap_or_default().to_string_lossy().to_string();

    let mut xml = format!("<directory id=\"{}\" parent=\"{}\" name=\"{}\">", id, parent_id_str, name);
    let mut children = Vec::new();

    match fs::read_dir(&full_path).await {
        Ok(mut entries) => {
             let mut entries_vec = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                 entries_vec.push(entry);
            }
            // Need to sort, but metadata lookup is async.
            // Ideally we pre-fetch metadata.
            let mut entries_with_meta = Vec::new();
            for e in entries_vec {
                let md = e.metadata().await.ok();
                entries_with_meta.push((e, md));
            }

            entries_with_meta.sort_by(|(a, a_md), (b, b_md)| {
                let a_is_dir = a_md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let b_is_dir = b_md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                if a_is_dir != b_is_dir {
                     // Dirs first
                    if a_is_dir { Ordering::Less } else { Ordering::Greater }
                } else {
                    a.file_name().cmp(&b.file_name())
                }
            });

            for (entry, _) in entries_with_meta {
                let name = entry.file_name().to_string_lossy().to_string();
                let sub_rel_path = Path::new(&rel_path).join(&name);
                let sub_id = hex::encode(sub_rel_path.to_string_lossy().to_string());

                let ft = entry.file_type().await.unwrap();
                if ft.is_dir() {
                    xml.push_str(&format!("<child id=\"{}\" parent=\"{}\" title=\"{}\" isDir=\"true\" coverArt=\"{}\"/>", sub_id, id, name, sub_id));
                    children.push(DirectoryChild {
                        id: sub_id.clone(),
                        parent: id.clone(),
                        title: name,
                        is_dir: true,
                        suffix: None, artist: None, album: None, duration: None, track: None,
                        cover_art: Some(sub_id),
                    });
                } else {
                    let suffix = Path::new(&name).extension().unwrap_or_default().to_string_lossy().to_string();

                    // Metadata extraction
                    let full_entry_path = entry.path();
                    let mut artist = None;
                    let mut album = None;
                    let mut title = name.clone();
                    let mut duration = None;
                    let mut track = None;

                    // Only attempt to read tags for common audio extensions to avoid overhead on other files
                    if ["mp3", "flac", "ogg", "m4a", "wav", "opus"].contains(&suffix.to_lowercase().as_str()) {
                         // Blocking read in async context, but it's iterating dir, so we are committed to this thread.
                         // For better perf, should use tokio::task::spawn_blocking for each file, but that's heavy.
                         // Just use blocking probe here.
                         if let Ok(tagged_file) = Probe::open(&full_entry_path).map(|p| p.read()) {
                            if let Ok(tf) = tagged_file {
                                let mut tag_opt = tf.primary_tag();
                                if tag_opt.is_none() {
                                    tag_opt = tf.first_tag();
                                }

                                if let Some(tag) = tag_opt {
                                    if let Some(t) = tag.title() { title = t.to_string(); }
                                    if let Some(a) = tag.artist() { artist = Some(a.to_string()); }
                                    if let Some(a) = tag.album() { album = Some(a.to_string()); }
                                    if let Some(t) = tag.track() { track = Some(t); }
                                }
                                let props = tf.properties();
                                duration = Some(props.duration().as_secs());
                            }
                         }
                    }

                    let mut xml_attrs = format!("suffix=\"{}\" coverArt=\"{}\"", suffix, sub_id);
                    if let Some(a) = &artist { xml_attrs.push_str(&format!(" artist=\"{}\"", a)); }
                    if let Some(a) = &album { xml_attrs.push_str(&format!(" album=\"{}\"", a)); }
                    if let Some(d) = duration { xml_attrs.push_str(&format!(" duration=\"{}\"", d)); }
                    if let Some(t) = track { xml_attrs.push_str(&format!(" track=\"{}\"", t)); }

                    xml.push_str(&format!("<child id=\"{}\" parent=\"{}\" title=\"{}\" isDir=\"false\" {} />", sub_id, id, title, xml_attrs));
                     children.push(DirectoryChild {
                        id: sub_id.clone(),
                        parent: id.clone(),
                        title,
                        is_dir: false,
                        suffix: Some(suffix),
                        artist,
                        album,
                        duration,
                        track,
                        cover_art: Some(sub_id),
                    });
                }
            }
        },
        Err(_) => return respond_error(&ctx, 70, "Folder not found"),
    }

    xml.push_str("</directory>");

    let json_resp = DirectoryRoot {
        directory: DirectoryInner {
            id,
            parent: if parent_id_str.is_empty() { None } else { Some(parent_id_str) },
            name,
            child: children
        }
    };

    respond(&ctx, xml, json_resp)
}

#[get("/stream.view?<id>")]
pub async fn stream(ctx: SubsonicContext, id: String) -> Result<crate::shared::FileResponse, Status> {
    if !ctx.authorized { return Err(Status::Unauthorized); }
    // Reuse existing download logic if possible, or reimplement basic file serving

    let rel_path_bytes = hex::decode(&id).map_err(|_| Status::BadRequest)?;
    let rel_path = String::from_utf8(rel_path_bytes).map_err(|_| Status::BadRequest)?;

    if rel_path.contains("..") { return Err(Status::Forbidden); }

    let storage = Path::new(crate::shared::STORAGE_ROOT);
    let full_path = storage.join(&rel_path);

    let metadata = fs::metadata(&full_path).await.map_err(|_| Status::NotFound)?;
    if !metadata.is_file() {
        return Err(Status::NotFound);
    }

    let file = fs::File::open(&full_path).await.map_err(|_| Status::NotFound)?;

    Ok(crate::shared::FileResponse {
        stream: Box::new(file),
        size: metadata.len(),
    })
}

pub enum CoverArtResponse {
    File(rocket::fs::NamedFile),
    Bytes(Vec<u8>, ContentType),
}

impl<'r> Responder<'r, 'static> for CoverArtResponse {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        match self {
            CoverArtResponse::File(f) => f.respond_to(req),
            CoverArtResponse::Bytes(data, ct) => {
                 Response::build()
                    .header(ct)
                    .sized_body(data.len(), Cursor::new(data))
                    .ok()
            }
        }
    }
}

#[get("/getCoverArt.view?<id>")]
pub async fn get_cover_art(ctx: SubsonicContext, id: String) -> Result<CoverArtResponse, Status> {
    if !ctx.authorized { return Err(Status::Unauthorized); }

     let rel_path_bytes = hex::decode(&id).map_err(|_| Status::BadRequest)?;
    let rel_path = String::from_utf8(rel_path_bytes).map_err(|_| Status::BadRequest)?;

    if rel_path.contains("..") { return Err(Status::Forbidden); }

    let storage = Path::new(crate::shared::STORAGE_ROOT);
    let full_path = storage.join(&rel_path);

    let metadata = fs::metadata(&full_path).await.map_err(|_| Status::NotFound)?;

    if metadata.is_dir() {
        // Look for cover.jpg, folder.jpg, etc.
        for name in &["cover.jpg", "folder.jpg", "cover.png", "folder.png", "front.jpg"] {
            let cover_path = full_path.join(name);
            if cover_path.exists() { // blocking exists check, acceptable for now or use fs::try_exists
               return rocket::fs::NamedFile::open(cover_path).await.map_err(|_| Status::NotFound).map(CoverArtResponse::File);
            }
        }
    } else {
        // It's a file, extract embedded art
        let path_clone = full_path.clone();
        let task = tokio::task::spawn_blocking(move || {
            if let Ok(tagged_file) = Probe::open(&path_clone).map(|p| p.read()) {
                if let Ok(tf) = tagged_file {
                    let mut tag_opt = tf.primary_tag();
                    if tag_opt.is_none() {
                        tag_opt = tf.first_tag();
                    }
                    if let Some(tag) = tag_opt {
                        if let Some(pic) = tag.pictures().first() {
                             // Extract strict values to ensure ownership is clear
                             let data = pic.data().to_vec();
                             // Convert mime to string immediately inside the closure to drop any potential references
                             let mime_string = format!("{:?}", pic.mime_type());
                             return Some((data, Some(mime_string)));
                        }
                    }
                }
             }
             None
        });

        if let Ok(Some((data, mime_opt))) = task.await {
             if let Some(mime_str) = mime_opt {
                 let content_type = ContentType::parse_flexible(&mime_str).unwrap_or(ContentType::Binary);
                 return Ok(CoverArtResponse::Bytes(data, content_type));
             }
        }
    }

    Err(Status::NotFound)
}
