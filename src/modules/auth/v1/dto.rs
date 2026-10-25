//! Request and response bodies for auth API v1.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::users::v1::dto::UserResponse;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "jane@example.com")]
    pub email: String,
    #[schema(example = "correct-horse-battery")]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    /// Send as `Authorization: Bearer <access_token>`.
    pub access_token: String,
    #[schema(example = "Bearer")]
    pub token_type: &'static str,
    /// Seconds until the token expires.
    #[schema(example = 3600)]
    pub expires_in: i64,
    pub user: UserResponse,
}
