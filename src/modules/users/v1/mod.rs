//! Users API v1, mounted at `/api/v1/users`.

use actix_web::web;
use utoipa::OpenApi;

pub mod dto;
mod handlers;

/// OpenAPI spec for these endpoints, with paths relative to the mount point.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::create_user,
        handlers::list_users,
        handlers::get_user,
        handlers::update_user,
        handlers::delete_user,
        handlers::assign_role,
        handlers::remove_role,
    ),
    tags((name = "users", description = "Customer and manager accounts")),
)]
pub struct ApiDoc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(handlers::create_user)
        .service(handlers::list_users)
        .service(handlers::get_user)
        .service(handlers::update_user)
        .service(handlers::delete_user)
        .service(handlers::assign_role)
        .service(handlers::remove_role);
}
