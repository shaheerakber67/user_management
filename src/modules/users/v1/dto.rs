//! Request and response bodies for users API v1.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    modules::users::models::{RoleName, User},
};

const MIN_PASSWORD_LEN: usize = 8;
pub const DEFAULT_PER_PAGE: u32 = 20;
pub const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    #[schema(example = "jane@example.com")]
    pub email: String,
    /// At least 8 characters.
    #[schema(example = "correct-horse-battery")]
    pub password: String,
    #[schema(example = "Jane Doe")]
    pub full_name: String,
    #[schema(example = "+1 555 0100")]
    pub phone: Option<String>,
}

impl CreateUserRequest {
    pub fn validate(&self) -> AppResult<()> {
        validate_email(&self.email)?;
        if self.password.chars().count() < MIN_PASSWORD_LEN {
            return Err(AppError::BadRequest(format!(
                "password must be at least {MIN_PASSWORD_LEN} characters"
            )));
        }
        validate_full_name(&self.full_name)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub full_name: Option<String>,
    pub phone: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdateUserRequest {
    pub fn validate(&self) -> AppResult<()> {
        match &self.full_name {
            Some(name) => validate_full_name(name),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRoleRequest {
    pub role: RoleName,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListUsersQuery {
    /// Only return users that have this role.
    pub role: Option<RoleName>,
    /// 1-based page number (default 1).
    pub page: Option<u32>,
    /// Page size, 1-100 (default 20).
    pub per_page: Option<u32>,
}

impl ListUsersQuery {
    /// 1-based page number.
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub is_active: bool,
    pub roles: Vec<RoleName>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            phone: user.phone,
            is_active: user.is_active,
            // Roles added to the table but not to `RoleName` are skipped.
            roles: user.roles.iter().filter_map(|r| r.parse().ok()).collect(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponse {
    pub data: Vec<UserResponse>,
    pub page: u32,
    pub per_page: u32,
}

fn validate_email(email: &str) -> AppResult<()> {
    let valid = email.len() <= 255
        && matches!(email.split_once('@'), Some((local, domain))
            if !local.is_empty() && domain.contains('.') && !domain.starts_with('.'));
    if valid {
        Ok(())
    } else {
        Err(AppError::BadRequest("invalid email address".into()))
    }
}

fn validate_full_name(name: &str) -> AppResult<()> {
    let len = name.trim().chars().count();
    if (1..=255).contains(&len) {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "full_name must be between 1 and 255 characters".into(),
        ))
    }
}
