#[macro_use]
extern crate rocket;

mod auth;
mod db;
mod files;
mod frontend;
mod guards;
mod list;
mod models;
mod shared;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::{Build, Rocket};

use crate::auth::{
    check_auth, create_default_admin, create_role, create_user, delete_role, delete_user, get_role,
    list_permissions, list_roles, list_users, login, logout, me, refresh, update_role,
    update_user_password, update_user_role,
};
use crate::files::{delete_path, download, upload, upload_progress, upload_ws};
use crate::frontend::frontend_fallback;
use crate::list::{list_directory, list_root};
use crate::shared::api_error;

fn prepare_dirs() {
    std::fs::create_dir_all(shared::STORAGE_ROOT).ok();
    std::fs::create_dir_all(shared::BUILD_ROOT).ok();
}

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();
    prepare_dirs();

    let pool = db::init_pool();

    rocket::build()
        .manage(pool)
        .attach(AdminBootstrap)
        .mount(
            "/api",
            routes![
                login,
                logout,
                me,
                refresh,
                check_auth,
                create_user,
                list_users,
                update_user_role,
                update_user_password,
                delete_user,
                list_roles,
                get_role,
                create_role,
                update_role,
                delete_role,
                list_permissions,
                list_root,
                list_directory,
                download,
                upload,
                delete_path,
                upload_progress,
                upload_ws,
            ],
        )
        .register("/api", catchers![api_error])
        .mount("/", FileServer::from(shared::BUILD_ROOT))
        .mount("/", routes![frontend_fallback])
}

// Fairing to bootstrap admin user on startup

struct AdminBootstrap;

#[rocket::async_trait]
impl Fairing for AdminBootstrap {
    fn info(&self) -> Info {
        Info {
            name: "Admin Bootstrap",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
        let pool = match rocket.state::<deadpool_postgres::Pool>() {
            Some(p) => p.clone(),
            None => {
                eprintln!("Admin bootstrap: DB pool not available");
                return Err(rocket);
            }
        };
        create_default_admin(&pool).await;
        Ok(rocket)
    }
}

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
