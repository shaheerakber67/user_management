//! Builds the HTTP server.

use std::net::TcpListener;

use actix_web::{dev::Server, middleware::Logger, web, App, HttpServer};
use sqlx::PgPool;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::{error::AppError, modules, modules::auth::JwtKeys};

/// Registers all routes and extractor configs. Shared by the server and
/// integration tests. Handlers that need the database expect a
/// `web::Data<PgPool>` to be registered on the `App`, and authenticated ones
/// also a `web::Data<JwtKeys>`.
pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.app_data(
        web::JsonConfig::default()
            .error_handler(|err, _| AppError::BadRequest(err.to_string()).into()),
    )
    .app_data(
        web::PathConfig::default()
            .error_handler(|err, _| AppError::BadRequest(err.to_string()).into()),
    )
    .app_data(
        web::QueryConfig::default()
            .error_handler(|err, _| AppError::BadRequest(err.to_string()).into()),
    );

    modules::configure(cfg);
    // Browser UI at `/swagger-ui/`. The token entered under Authorize is kept
    // across page reloads.
    cfg.service(web::redirect("/swagger-ui", "/swagger-ui/"));
    cfg.service(
        SwaggerUi::new("/swagger-ui/{_:.*}")
            .url("/api-docs/openapi.json", modules::openapi())
            .config(Config::default().persist_authorization(true)),
    );
    cfg.default_service(web::to(|| async { Err::<&str, _>(AppError::NotFound) }));
}

/// Starts the server on an already-bound listener (pass port 0 in tests).
pub fn run(listener: TcpListener, pool: PgPool, jwt: JwtKeys) -> std::io::Result<Server> {
    let pool = web::Data::new(pool);
    let jwt = web::Data::new(jwt);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(pool.clone())
            .app_data(jwt.clone())
            .configure(configure_app)
    })
    .listen(listener)?
    .run();
    Ok(server)
}
