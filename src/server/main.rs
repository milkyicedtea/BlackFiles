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
mod tus;
mod upload_links;

pub mod test;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::{Build, Rocket};

use crate::auth::{
    check_auth, create_default_admin, create_role, create_user, delete_role, delete_user, get_role,
    list_permissions, list_roles, list_users, login, logout, me, move_role, refresh, update_role,
    update_user_password, update_user_role,
};
use crate::files::{delete_path, download};
use crate::frontend::frontend_fallback;
use crate::list::{list_directory, list_root};
use crate::shared::api_error;
use crate::tus::{
    create_public_tus_upload, create_tus_upload, head_public_tus_upload, head_tus_upload,
    list_tus_uploads, patch_public_tus_upload, patch_tus_upload, public_tus_options,
    terminate_public_tus_upload, terminate_tus_upload, tus_options,
};
use crate::upload_links::{
    create_upload_link, delete_upload_link, get_public_upload_link, list_upload_links,
};

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
        .attach(DatabaseFeatures)
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
                move_role,
                delete_role,
                list_permissions,
                list_root,
                list_directory,
                download,
                delete_path,
                tus_options,
                list_tus_uploads,
                create_tus_upload,
                head_tus_upload,
                patch_tus_upload,
                terminate_tus_upload,
                public_tus_options,
                create_public_tus_upload,
                head_public_tus_upload,
                patch_public_tus_upload,
                terminate_public_tus_upload,
                create_upload_link,
                list_upload_links,
                delete_upload_link,
                get_public_upload_link,
            ],
        )
        .register("/api", catchers![api_error])
        .mount("/", FileServer::from(shared::BUILD_ROOT))
        .mount("/", routes![frontend_fallback])
}

// Fairing to apply the idempotent database feature scripts before bootstrapping users.

struct DatabaseFeatures;

#[rocket::async_trait]
impl Fairing for DatabaseFeatures {
    fn info(&self) -> Info {
        Info {
            name: "Database Features",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
        let pool = match rocket.state::<deadpool_postgres::Pool>() {
            Some(pool) => pool.clone(),
            None => {
                eprintln!("Database features: DB pool not available");
                return Err(rocket);
            }
        };
        if let Err(error) = db::apply_feature_scripts(&pool).await {
            eprintln!("Database features: {error}");
            return Err(rocket);
        }
        Ok(rocket)
    }
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
