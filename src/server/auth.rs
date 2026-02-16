use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::{Deserialize, Serialize};
use rocket::serde::json::Json;

pub struct Auth;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LoginRequest {
    token: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AuthResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = ();

    async fn from_request(request: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let cookie_token = request.cookies()
            .get("blackfiles_token")
            .map(|c| c.value());
        // load expected token from env
        let expected = std::env::var("BLACKFILES_TOKEN").ok();

        match (cookie_token, expected) {
            (Some(t), Some(e)) if t == e => Outcome::Success(Auth),
            _ => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}

#[post("/auth", data = "<login>")]
pub fn login(jar: &CookieJar<'_>, login: Json<LoginRequest>) -> (Status, Json<AuthResponse>) {
    let expected = match std::env::var("BLACKFILES_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            return (
                Status::InternalServerError,
                Json(AuthResponse {
                    success: false,
                    message: Some("Server configuration error. Could not find BLACKFILES_TOKEN variable".to_string()),
                })
            )
        }
    };

    if login.token == expected {
        jar.add(Cookie::new("blackfiles_token", login.token.clone()));
        (
            Status::Ok,
            Json(AuthResponse {
                success: true,
                message: None,
            })
        )
    } else {
        (
            Status::Unauthorized,
            Json(AuthResponse {
                success: false,
                message: Some("Invalid token".to_string()),
            })
        )
    }
}

#[get("/check")]
pub fn check_auth(_auth: Auth) -> Json<AuthResponse> {
    Json(AuthResponse { success: true, message: None })
}
