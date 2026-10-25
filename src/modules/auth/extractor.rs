//! Add `auth: AuthUser` to a handler's arguments to require a valid
//! `Authorization: Bearer <token>` header. Put it first so a missing token is
//! reported before any other extractor error.

use std::{future::Future, pin::Pin};

use actix_web::{dev::Payload, http::header, web, FromRequest, HttpRequest};
use sqlx::PgPool;
use uuid::Uuid;

use super::JwtKeys;
use crate::{
    error::AppError,
    modules::users::{
        models::{RoleName, User},
        repository::UserRepository,
    },
};

/// The active user the request's token belongs to, freshly loaded from the
/// database.
#[derive(Debug)]
pub struct AuthUser {
    pub user: User,
}

impl AuthUser {
    pub fn is_manager(&self) -> bool {
        self.user
            .roles
            .iter()
            .any(|r| r == RoleName::Manager.as_str())
    }

    pub fn require_manager(&self) -> Result<(), AppError> {
        if self.is_manager() {
            Ok(())
        } else {
            Err(AppError::Forbidden("manager role required".into()))
        }
    }

    /// Allows acting on your own account, or on any account as a manager.
    pub fn require_self_or_manager(&self, user_id: Uuid) -> Result<(), AppError> {
        if self.user.id == user_id || self.is_manager() {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "not allowed to access this user".into(),
            ))
        }
    }
}

impl FromRequest for AuthUser {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let token = bearer_token(req);
        let keys = req.app_data::<web::Data<JwtKeys>>().cloned();
        let pool = req.app_data::<web::Data<PgPool>>().cloned();

        Box::pin(async move {
            let (Some(keys), Some(pool)) = (keys, pool) else {
                return Err(AppError::Internal(
                    "AuthUser needs JwtKeys and PgPool app data".into(),
                ));
            };
            let token =
                token.ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;
            let id = keys.verify(&token)?;

            // A deleted user's token is treated like any other invalid token.
            let user = UserRepository::find_by_id(pool.get_ref(), id)
                .await?
                .ok_or_else(|| AppError::Unauthorized("invalid or expired token".into()))?;
            if !user.is_active {
                return Err(AppError::Forbidden("account is disabled".into()));
            }
            Ok(AuthUser { user })
        })
    }
}

fn bearer_token(req: &HttpRequest) -> Option<String> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme
        .eq_ignore_ascii_case("bearer")
        .then(|| token.trim().to_owned())
}
