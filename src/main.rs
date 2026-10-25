use std::net::TcpListener;

use aws_app::{config::Config, db, modules::auth::JwtKeys, startup};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    let pool = db::connect(&config).await.map_err(std::io::Error::other)?;
    db::migrate(&pool).await.map_err(std::io::Error::other)?;
    log::info!("Database ready, migrations applied");

    let listener = TcpListener::bind(config.address())?;
    log::info!("Listening on http://{}", listener.local_addr()?);

    let jwt = JwtKeys::new(
        config.jwt_secret.as_bytes(),
        chrono::Duration::minutes(config.jwt_ttl_minutes),
    );
    startup::run(listener, pool, jwt)?.await
}
