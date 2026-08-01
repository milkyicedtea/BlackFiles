use super::*;

use crate::models::{AuthError, Claims};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

const ROLE_POSITION_LOCK: i64 = 1_976_101;

pub(crate) async fn lock_role_positions(
    transaction: &deadpool_postgres::Transaction<'_>,
) -> Result<(), tokio_postgres::Error> {
    transaction
        .execute("SELECT pg_advisory_xact_lock($1)", &[&ROLE_POSITION_LOCK])
        .await?;
    Ok(())
}

// Password hashing

pub(crate) fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AuthError::HashingError)
}

pub(crate) fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::HashingError)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// JWT generation

pub(crate) fn generate_jwt(
    user_id: &Uuid,
    username: &str,
    role: &str,
    secret: &str,
    expiration_hours: i64,
) -> Result<String, AuthError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiration_hours))
        .ok_or(AuthError::TokenGenerationError)?
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::TokenGenerationError)
}

// Refresh token helpers

pub(crate) fn generate_refresh_token() -> String {
    random_hex::<32>()
}
