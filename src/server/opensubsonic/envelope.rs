use super::*;

// ── Response envelope ──

pub const SUB_SERVER_TYPE: &str = "Blackfiles";
pub const SUB_SERVER_VERSION: &str = "0.1.0";

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicResponse<T: Serialize> {
    #[serde(rename = "subsonic-response")]
    pub body: SubsonicBody<T>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubsonicBody<T: Serialize> {
    pub status: String,
    pub version: String,
    #[serde(rename = "type")]
    pub server_type: String,
    #[serde(rename = "serverVersion")]
    pub server_version: String,
    #[serde(rename = "openSubsonic")]
    pub open_subsonic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SubsonicError>,
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> SubsonicResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            body: SubsonicBody {
                status: "ok".into(),
                version: "1.16.1".into(),
                server_type: SUB_SERVER_TYPE.into(),
                server_version: SUB_SERVER_VERSION.into(),
                open_subsonic: true,
                error: None,
                data: Some(data),
            },
        }
    }
}

impl SubsonicResponse<EmptyResponse> {
    pub fn ok_empty() -> Self {
        Self::ok(EmptyResponse {})
    }

    pub fn error(code: i32, message: &str) -> Self {
        Self {
            body: SubsonicBody {
                status: "failed".into(),
                version: "1.16.1".into(),
                server_type: SUB_SERVER_TYPE.into(),
                server_version: SUB_SERVER_VERSION.into(),
                open_subsonic: true,
                error: Some(SubsonicError {
                    code,
                    message: message.into(),
                }),
                data: None,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct EmptyResponse {}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct LicenseResponse {
    #[serde(default = "default_true")]
    pub valid: bool,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExtensionsResponse {
    #[serde(rename = "openSubsonicExtensions")]
    pub extensions: Vec<ExtensionInfo>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExtensionInfo {
    pub name: String,
    pub versions: Vec<i32>,
}
