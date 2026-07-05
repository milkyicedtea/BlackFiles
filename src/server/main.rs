#[macro_use]
extern crate rocket;

mod api;
mod auth;
mod db;
mod files;
mod frontend;
mod guards;
mod models;
mod shared;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::{Build, Rocket};

use crate::api::{list_directory, list_root};
use crate::auth::{
    check_auth, create_default_admin, create_role, create_user, delete_role, delete_user, get_role,
    list_permissions, list_roles, list_users, login, logout, me, refresh, update_role,
    update_user_password, update_user_role,
};
use crate::files::{delete_path, download, upload, upload_progress, upload_ws};
use crate::frontend::frontend_fallback;

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
