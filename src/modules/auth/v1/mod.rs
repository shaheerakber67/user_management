//! Auth API v1, mounted at `/api/v1/auth`.

use actix_web::web;
use utoipa::OpenApi;

pub mod dto;
mod handlers;

/// OpenAPI spec for these endpoints, with paths relative to the mount point.
#[derive(OpenApi)]
#[openapi(
    paths(handlers::login, handlers::me),
    tags((name = "auth", description = "Log in and inspect the current user")),
)]
pub struct ApiDoc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::login).service(handlers::me);
}
