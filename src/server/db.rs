use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

const FEATURE_SCRIPTS: &[(&str, &str)] = &[
    ("0000_init.sql", include_str!("../../dbinit/0000_init.sql")),
    (
        "0001_dense_role_positions.sql",
        include_str!("../../dbinit/0001_dense_role_positions.sql"),
    ),
    (
        "0002_core_seed.sql",
        include_str!("../../dbinit/0002_core_seed.sql"),
    ),
    (
        "0003_upload_links.sql",
        include_str!("../../dbinit/0003_upload_links.sql"),
    ),
    (
        "0004_upload_sessions.sql",
        include_str!("../../dbinit/0004_upload_sessions.sql"),
    ),
    (
        "0005_public_upload_sessions.sql",
        include_str!("../../dbinit/0005_public_upload_sessions.sql"),
    ),
];

/// Initialize the PostgreSQL connection pool.
pub fn init_pool() -> Pool {
    let pg_user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "blackfiles".to_string());
    let pg_password =
        std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "very_secure_password".to_string());
    let pg_host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let pg_db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "blackfiles".to_string());

    let mut cfg = Config::new();
    cfg.user = Some(pg_user);
    cfg.password = Some(pg_password);
    cfg.host = Some(pg_host);
    cfg.dbname = Some(pg_db);
    cfg.create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create database pool")
}

/// Apply every idempotent database feature script before serving requests.
pub async fn apply_feature_scripts(pool: &Pool) -> Result<(), String> {
    let client = pool
        .get()
        .await
        .map_err(|error| format!("could not acquire database connection: {error}"))?;

    for (name, script) in FEATURE_SCRIPTS {
        client.batch_execute(script).await.map_err(|error| {
            format!("could not apply database feature script {name}: {error:?}")
        })?;
    }

    Ok(())
}
