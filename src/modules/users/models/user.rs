use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// A `users` row joined with the names of its assigned roles.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub roles: Vec<String>,
}

/// Fields required to insert a user.
#[derive(Debug)]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub password_hash: &'a str,
    pub full_name: &'a str,
    pub phone: Option<&'a str>,
}

/// Partial update; `None` leaves the column unchanged.
#[derive(Debug, Default)]
pub struct UpdateUser<'a> {
    pub full_name: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub is_active: Option<bool>,
}
