//! Auth API v1 endpoints.

use actix_web::{get, post, web, HttpResponse};
use sqlx::PgPool;

use super::dto::{LoginRequest, TokenResponse};
use crate::{
    error::{AppError, AppResult, ErrorBody},
    modules::{
        auth::{AuthUser, JwtKeys},
        users::{repository::UserRepository, v1::dto::UserResponse},
    },
    utils::password,
};

/// Exchanges email and password for a bearer token. In Swagger, paste the
/// returned `access_token` into **Authorize**.
#[utoipa::path(
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in", body = TokenResponse),
        (status = 401, description = "Wrong email or password", body = ErrorBody),
        (status = 403, description = "Account is disabled", body = ErrorBody),
    )
)]
#[post("/login")]
async fn login(
    pool: web::Data<PgPool>,
    keys: web::Data<JwtKeys>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    let LoginRequest { email, password } = body.into_inner();
    // Same error for unknown email and wrong password.
    let invalid = || AppError::Unauthorized("invalid email or password".into());

    let user = UserRepository::find_by_email(pool.get_ref(), email.trim())
        .await?
        .ok_or_else(invalid)?;
    if !password::verify(password, user.password_hash.clone()).await? {
        return Err(invalid());
    }
    if !user.is_active {
        return Err(AppError::Forbidden("account is disabled".into()));
    }

    Ok(HttpResponse::Ok().json(TokenResponse {
        access_token: keys.issue(user.id)?,
        token_type: "Bearer",
        expires_in: keys.ttl().num_seconds(),
        user: user.into(),
    }))
}

/// Returns the logged-in user.
#[utoipa::path(
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "The current user", body = UserResponse),
        (status = 401, description = "Missing, invalid or expired token", body = ErrorBody),
        (status = 403, description = "Account is disabled", body = ErrorBody),
    )
)]
#[get("/me")]
async fn me(auth: AuthUser) -> HttpResponse {
    HttpResponse::Ok().json(UserResponse::from(auth.user))
}
