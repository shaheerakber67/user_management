//! Users module: accounts and their role assignments.
//!
//! - `v1/`         HTTP endpoints and request/response DTOs for API version 1
//! - `models/`     database row types and domain enums
//! - `repository/` SQL queries; the only place that talks to the database

use actix_web::web;

pub mod models;
pub mod repository;
pub mod v1;

/// Mounted under `/api`, so v1 lives at `/api/v1/users`.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/v1/users").configure(v1::configure));
}
