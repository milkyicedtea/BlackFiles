#[cfg(test)]
mod tests {
    use crate::frontend::frontend_fallback;
    use crate::shared::api_error;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    #[get("/bare-status")]
    fn bare_status() -> Status {
        Status::Forbidden
    }

    fn test_rocket() -> rocket::Rocket<rocket::Build> {
        rocket::build()
            .mount("/", routes![frontend_fallback])
            .mount("/api", routes![bare_status])
            .register("/api", catchers![api_error])
    }

    #[test]
    fn api_route_miss_returns_json_error() {
        let client = Client::tracked(test_rocket()).expect("test Rocket should launch");
        let response = client.get("/api/missing").dispatch();

        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        assert_eq!(
            response.into_json::<serde_json::Value>(),
            Some(serde_json::json!({"error": "Not Found"}))
        );
    }

    #[test]
    fn bare_status_returns_json_error() {
        let client = Client::tracked(test_rocket()).expect("test Rocket should launch");
        let response = client.get("/api/bare-status").dispatch();

        assert_eq!(response.status(), Status::Forbidden);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        assert_eq!(
            response.into_json::<serde_json::Value>(),
            Some(serde_json::json!({"error": "Forbidden"}))
        );
    }
}
