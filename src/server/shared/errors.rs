use rocket::Request;
use rocket::http::Status;
use rocket::serde::json::Json;

pub(crate) type ApiError = (Status, Json<serde_json::Value>);

pub(crate) fn server_error() -> ApiError {
    status_error(Status::InternalServerError, "Server error")
}

#[catch(default)]
pub(crate) fn api_error(status: Status, _: &Request<'_>) -> ApiError {
    let message = if status == Status::InternalServerError {
        "Server error"
    } else {
        status.reason().unwrap_or("Request failed")
    };

    status_error(status, message)
}

pub(crate) fn bad_request(message: &str) -> ApiError {
    status_error(Status::BadRequest, message)
}

pub(crate) fn unauthorized(message: &str) -> ApiError {
    status_error(Status::Unauthorized, message)
}

pub(crate) fn forbidden() -> ApiError {
    status_error(Status::Forbidden, "Insufficient permissions")
}

pub(crate) fn not_found(message: &str) -> ApiError {
    status_error(Status::NotFound, message)
}

pub(crate) fn conflict(message: &str) -> ApiError {
    status_error(Status::Conflict, message)
}

pub(crate) fn status_error(status: Status, message: &str) -> ApiError {
    (status, Json(serde_json::json!({"error": message})))
}
