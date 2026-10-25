//! Argon2id password hashing. Hashing is CPU-heavy, so it runs on the
//! blocking thread pool instead of an async worker.

use actix_web::web;
use argon2::{
    password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier},
    Argon2,
};

use crate::error::{AppError, AppResult};

/// Hashes `password` into a PHC string (`$argon2id$v=19$...`) with a random salt.
pub async fn hash(password: String) -> AppResult<String> {
    web::block(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(e.to_string().into()))
    })
    .await
    .map_err(|e| AppError::Internal(Box::new(e)))?
}

/// Returns `true` if `password` matches the stored PHC hash.
pub async fn verify(password: String, password_hash: String) -> AppResult<bool> {
    web::block(move || {
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|e| AppError::Internal(e.to_string().into()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    })
    .await
    .map_err(|e| AppError::Internal(Box::new(e)))?
}
