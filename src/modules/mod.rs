//! Feature modules. Each module owns its versioned endpoints (`v1`, `v2`, ...),
//! its models and its repository (SQL queries).
//!
//! To add a module: create `modules/<name>/` with the same layout as `users`,
//! declare it here, mount it inside the `/api` scope below and nest its
//! `ApiDoc` in [`ApiDoc`] at the same path.

use actix_web::web;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

pub mod auth;
pub mod system;
pub mod users;

/// Versioned API modules. Use [`openapi`] for the full spec.
#[derive(OpenApi)]
#[openapi(
    info(title = "Food Ordering API"),
    components(schemas(crate::error::ErrorBody)),
    modifiers(&BearerAuth),
    nest(
        (path = "/api/v1/auth", api = auth::v1::ApiDoc),
        (path = "/api/v1/users", api = users::v1::ApiDoc),
    ),
)]
pub struct ApiDoc;

/// Declares the `bearer_auth` scheme that protected paths reference, which
/// gives Swagger UI its **Authorize** button.
struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some(
                            "Paste the `access_token` from `POST /api/v1/auth/login`.",
                        ))
                        .build(),
                ),
            );
        }
    }
}

/// The full OpenAPI spec, served at `/api-docs/openapi.json`. System routes
/// sit at the root, so they are merged in rather than nested.
pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut doc = ApiDoc::openapi();
    doc.merge(system::ApiDoc::openapi());
    doc
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    system::configure(cfg);
    cfg.service(
        web::scope("/api")
            .configure(auth::configure)
            .configure(users::configure),
    );
}
