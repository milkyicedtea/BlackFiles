use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

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
