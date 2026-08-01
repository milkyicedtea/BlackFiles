use super::*;

// ── Endpoints ──

#[get("/rest/ping")]
pub(crate) fn ping(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(serde_json::to_value(SubsonicResponse::<EmptyResponse>::ok_empty()).unwrap_or_default())
}

#[get("/rest/getLicense")]
pub(crate) fn get_license(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(SubsonicResponse::ok(LicenseResponse { valid: true }))
            .unwrap_or_default(),
    )
}

#[get("/rest/getOpenSubsonicExtensions")]
pub(crate) fn get_open_subsonic_extensions(_user: SubsonicUser) -> Json<serde_json::Value> {
    Json(
        serde_json::to_value(SubsonicResponse::ok(ExtensionsResponse {
            extensions: vec![
                ExtensionInfo {
                    name: "formPost".into(),
                    versions: vec![1, 2],
                },
                ExtensionInfo {
                    name: "apiKeyAuth".into(),
                    versions: vec![1],
                },
                ExtensionInfo {
                    name: "songTitle".into(),
                    versions: vec![1],
                },
            ],
        }))
        .unwrap_or_default(),
    )
}
