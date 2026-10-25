//! Authentication: JWT login and the `AuthUser` extractor.
//!
//! - `jwt`       signs and verifies access tokens
//! - `extractor` `AuthUser`, which guards handlers that need a logged-in user
//! - `v1/`       login and current-user endpoints

use actix_web::web;

mod extractor;
mod jwt;
pub mod v1;

pub use extractor::AuthUser;
pub use jwt::JwtKeys;

/// Mounted under `/api`, so v1 lives at `/api/v1/auth`.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/v1/auth").configure(v1::configure));
}
