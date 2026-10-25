//! Runtime configuration loaded from environment variables.

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_ttl_minutes: i64,
}

/// HS256 needs a secret at least as long as its 256-bit output.
const MIN_JWT_SECRET_LEN: usize = 32;

impl Config {
    /// Reads `APP_HOST`, `APP_PORT`, `RUST_LOG`, `DATABASE_URL`,
    /// `DATABASE_MAX_CONNECTIONS`, `JWT_SECRET` and `JWT_TTL_MINUTES`, falling
    /// back to defaults where sensible.
    ///
    /// Panics if `DATABASE_URL` is missing or `JWT_SECRET` is missing or
    /// shorter than 32 bytes, since the app cannot run safely without them.
    pub fn from_env() -> Self {
        Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: env::var("APP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(10),
            jwt_secret: {
                let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
                assert!(
                    secret.len() >= MIN_JWT_SECRET_LEN,
                    "JWT_SECRET must be at least {MIN_JWT_SECRET_LEN} bytes"
                );
                secret
            },
            jwt_ttl_minutes: env::var("JWT_TTL_MINUTES")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(60),
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
