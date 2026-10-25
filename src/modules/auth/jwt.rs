//! HS256 access tokens. A token only identifies the user (`sub`); roles and
//! `is_active` are re-read from the database on every request, so changes to
//! them take effect immediately rather than when the token expires.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    iat: i64,
    exp: i64,
}

/// Signing keys and token lifetime, registered as `web::Data<JwtKeys>`.
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl: Duration,
}

impl JwtKeys {
    pub fn new(secret: &[u8], ttl: Duration) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation: Validation::new(Algorithm::HS256),
            ttl,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Issues a token for `user_id` that expires after the configured TTL.
    pub fn issue(&self, user_id: Uuid) -> AppResult<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            iat: now.timestamp(),
            exp: (now + self.ttl).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| AppError::Internal(Box::new(e)))
    }

    /// Returns the user id if `token` is validly signed and unexpired.
    pub fn verify(&self, token: &str) -> AppResult<Uuid> {
        decode::<Claims>(token, &self.decoding, &self.validation)
            .map(|data| data.claims.sub)
            .map_err(|_| AppError::Unauthorized("invalid or expired token".into()))
    }
}
