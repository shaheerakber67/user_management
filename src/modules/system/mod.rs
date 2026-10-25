//! Unversioned operational routes (root, echo, health check).

use actix_web::web;
use utoipa::OpenApi;

mod echo;
mod health;
mod hello;

#[derive(OpenApi)]
#[openapi(
    paths(hello::hello, echo::echo, health::health_check),
    tags((name = "system", description = "Operational endpoints")),
)]
pub struct ApiDoc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(hello::hello)
        .service(echo::echo)
        .service(health::health_check);
}
