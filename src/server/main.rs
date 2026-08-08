#[macro_use]
extern crate rocket;

mod auth;
mod db;
mod files;
mod frontend;
mod models;
mod music;
mod opensubsonic;
mod shared;
pub mod test;

use crate::auth::{
    admin_revoke_api_key, create_api_key, create_default_admin, create_role, create_user,
    delete_role, delete_user, get_role, list_all_api_keys, list_my_api_keys, list_permissions,
    list_roles, list_users, login::check_auth, login::login, login::logout, login::me,
    login::refresh, move_role, revoke_api_key, update_role, update_user_password, update_user_role,
};
use crate::files::{
    create_folder, create_public_tus_upload, create_tus_upload, create_upload_link, delete_path,
    delete_upload_link, download::download, get_public_upload_link, head_public_tus_upload,
    head_tus_upload, list_directory, list_root, list_tus_uploads, list_upload_links,
    patch_public_tus_upload, patch_tus_upload, public_tus_options, rename_path,
    terminate_public_tus_upload, terminate_tus_upload, tus_options,
};
use crate::frontend::frontend_fallback;
use crate::music::{
    add_to_library, create_music_upload, delete_song, get_song_cover, head_music_upload,
    list_personal_library, list_song_selection, list_songs, music_tus_options, patch_music_upload,
    remove_from_library, scan_songs, set_library_membership, terminate_music_upload,
    update_song_cover, update_song_tags,
};
use crate::opensubsonic::{
    create_playlist, delete_playlist, get_album, get_album_list, get_album_list2, get_artist,
    get_artists, get_bookmarks, get_cover_art, get_genres, get_indexes, get_license,
    get_music_directory, get_music_folders, get_now_playing, get_open_subsonic_extensions,
    get_playlist, get_playlists, get_random_songs, get_song, get_starred, get_starred2, ping,
    scrobble, search2, search3, star, stream, subsonic_auth_error, subsonic_download, unstar,
    update_playlist,
};
use crate::shared::api_error;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::http::uri::Origin;
use rocket::{Build, Data, Request, Rocket};

fn prepare_dirs() {
    std::fs::create_dir_all(crate::shared::STORAGE_ROOT).ok();
    std::fs::create_dir_all(crate::shared::MUSIC_ROOT).ok();
    std::fs::create_dir_all(crate::shared::BUILD_ROOT).ok();
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
        .attach(OpenSubsonicViewCompatibility)
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
                create_folder,
                rename_path,
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
                list_songs,
                list_song_selection,
                delete_song,
                update_song_tags,
                get_song_cover,
                update_song_cover,
                scan_songs,
                list_personal_library,
                add_to_library,
                remove_from_library,
                set_library_membership,
                music_tus_options,
                create_music_upload,
                head_music_upload,
                patch_music_upload,
                terminate_music_upload,
                list_my_api_keys,
                create_api_key,
                revoke_api_key,
                list_all_api_keys,
                admin_revoke_api_key,
            ],
        )
        .register("/api", catchers![api_error])
        .mount(
            "/rest",
            routes![
                ping,
                get_license,
                get_open_subsonic_extensions,
                get_bookmarks,
                get_music_folders,
                get_indexes,
                get_music_directory,
                get_artists,
                get_artist,
                get_album,
                get_song,
                get_album_list,
                get_album_list2,
                get_genres,
                stream,
                subsonic_download,
                get_cover_art,
                search2,
                search3,
                get_random_songs,
                get_playlists,
                get_playlist,
                create_playlist,
                update_playlist,
                delete_playlist,
                star,
                unstar,
                get_starred,
                get_starred2,
                scrobble,
                get_now_playing,
                subsonic_auth_error,
            ],
        )
        .mount("/", FileServer::from(crate::shared::BUILD_ROOT))
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

struct OpenSubsonicViewCompatibility;

#[rocket::async_trait]
impl Fairing for OpenSubsonicViewCompatibility {
    fn info(&self) -> Info {
        Info {
            name: "OpenSubsonic .view Compatibility",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        let path = request.uri().path().to_string();

        if !path.starts_with("/rest/") {
            return;
        }

        let Some(path_without_suffix) = path.strip_suffix(".view") else {
            return;
        };

        let query = request.uri().query().map(|query| query.to_string());
        let mut rewritten = String::with_capacity(
            path_without_suffix.len() + query.as_ref().map_or(0, |value| value.len() + 1),
        );

        rewritten.push_str(path_without_suffix);

        if let Some(query) = query {
            rewritten.push('?');
            rewritten.push_str(&query);
        }

        let rewritten =
            Origin::parse_owned(rewritten).expect("removing .view preserves a valid request URI");

        request.set_uri(rewritten);
    }
}
