//! Database pool setup and migrations.

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Config;

/// Opens a Postgres connection pool.
pub async fn connect(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await
}

/// Applies any pending migrations from `./migrations` (embedded at compile time).
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
