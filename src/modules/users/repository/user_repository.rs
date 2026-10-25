//! SQL for users and their role mappings.
//!
//! Every function takes a generic executor so it can run on the pool or inside
//! a transaction (`&mut *tx`).

use sqlx::PgExecutor;
use uuid::Uuid;

use crate::modules::users::models::{NewUser, RoleName, UpdateUser, User};

/// Expands to a literal query selecting users with their role names
/// aggregated into `roles`, followed by `$tail` (which must include
/// `GROUP BY u.id`). A macro keeps the SQL a compile-time literal, as sqlx
/// requires.
macro_rules! select_user {
    ($tail:literal) => {
        concat!(
            r#"
            SELECT u.id, u.email, u.password_hash, u.full_name, u.phone, u.is_active,
                   u.created_at, u.updated_at,
                   COALESCE(
                       array_agg(r.name ORDER BY r.name) FILTER (WHERE r.name IS NOT NULL),
                       '{}'
                   ) AS roles
            FROM users u
            LEFT JOIN user_roles ur ON ur.user_id = u.id
            LEFT JOIN roles r ON r.id = ur.role_id
            "#,
            $tail
        )
    };
}

pub struct UserRepository;

impl UserRepository {
    pub async fn create<'e, E: PgExecutor<'e>>(db: E, user: NewUser<'_>) -> sqlx::Result<Uuid> {
        sqlx::query_scalar(
            r#"
            INSERT INTO users (email, password_hash, full_name, phone)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(user.email)
        .bind(user.password_hash)
        .bind(user.full_name)
        .bind(user.phone)
        .fetch_one(db)
        .await
    }

    pub async fn find_by_id<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<User>> {
        sqlx::query_as(select_user!("WHERE u.id = $1 GROUP BY u.id"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn find_by_email<'e, E: PgExecutor<'e>>(
        db: E,
        email: &str,
    ) -> sqlx::Result<Option<User>> {
        sqlx::query_as(select_user!(
            "WHERE lower(u.email) = lower($1) GROUP BY u.id"
        ))
        .bind(email)
        .fetch_optional(db)
        .await
    }

    /// Newest first. When `role` is set, only users holding that role.
    pub async fn list<'e, E: PgExecutor<'e>>(
        db: E,
        role: Option<RoleName>,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<User>> {
        sqlx::query_as(select_user!(
            r#"
            WHERE $1::text IS NULL OR u.id IN (
                SELECT ur2.user_id FROM user_roles ur2
                JOIN roles r2 ON r2.id = ur2.role_id
                WHERE r2.name = $1
            )
            GROUP BY u.id
            ORDER BY u.created_at DESC, u.id
            LIMIT $2 OFFSET $3"#
        ))
        .bind(role.map(|r| r.as_str()))
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
    }

    /// Returns `false` if no user has this id.
    pub async fn update<'e, E: PgExecutor<'e>>(
        db: E,
        id: Uuid,
        changes: UpdateUser<'_>,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE users SET
                full_name = COALESCE($2, full_name),
                phone     = COALESCE($3, phone),
                is_active = COALESCE($4, is_active)
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(changes.full_name)
        .bind(changes.phone)
        .bind(changes.is_active)
        .execute(db)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Returns `false` if no user has this id. Role mappings cascade.
    pub async fn delete<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Idempotent: assigning a role the user already has is a no-op.
    pub async fn assign_role<'e, E: PgExecutor<'e>>(
        db: E,
        user_id: Uuid,
        role: RoleName,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_id)
            SELECT $1, id FROM roles WHERE name = $2
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(role.as_str())
        .execute(db)
        .await?;
        Ok(())
    }

    /// Returns `false` if the user did not have the role.
    pub async fn remove_role<'e, E: PgExecutor<'e>>(
        db: E,
        user_id: Uuid,
        role: RoleName,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM user_roles
            WHERE user_id = $1
              AND role_id = (SELECT id FROM roles WHERE name = $2)
            "#,
        )
        .bind(user_id)
        .bind(role.as_str())
        .execute(db)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
