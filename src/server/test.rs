#[cfg(test)]
mod tests {
    use crate::OpenSubsonicViewCompatibility;
    use crate::frontend::frontend_fallback;
    use crate::shared::api_error;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    #[get("/bare-status")]
    fn bare_status() -> Status {
        Status::Forbidden
    }

    #[get("/ping?<c>")]
    fn subsonic_ping_stub(c: &str) -> String {
        c.to_owned()
    }

    fn test_rocket() -> rocket::Rocket<rocket::Build> {
        rocket::build()
            .attach(OpenSubsonicViewCompatibility)
            .mount("/rest", routes![subsonic_ping_stub])
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
    fn open_subsonic_view_suffix_preserves_query_and_routes() {
        let client = Client::tracked(test_rocket()).expect("test Rocket should launch");
        let response = client
            .get("/rest/ping.view?u=test&p=test&v=1.13.1&c=Symfonium&f=json")
            .dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().as_deref(), Some("Symfonium"));
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
