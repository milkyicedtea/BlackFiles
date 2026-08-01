use super::{ApiError, server_error};

pub(crate) fn db_error(error: impl std::fmt::Display) -> ApiError {
    eprintln!("DB error: {error}");
    server_error()
}

pub(crate) async fn get_client(
    pool: &deadpool_postgres::Pool,
) -> Result<deadpool_postgres::Object, ApiError> {
    pool.get().await.map_err(|error| {
        eprintln!("DB pool error: {error}");
        server_error()
    })
}
