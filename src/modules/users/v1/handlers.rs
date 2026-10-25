//! Users API v1 endpoints.
//!
//! Registration is public. Everything else needs a bearer token: users may
//! read, edit and delete their own account, managers may do anything,
//! including listing users, changing roles and (de)activating accounts.

use actix_web::{delete, get, patch, post, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use super::dto::{
    AssignRoleRequest, CreateUserRequest, ListUsersQuery, UpdateUserRequest, UserListResponse,
    UserResponse,
};
use crate::{
    error::{AppError, AppResult, ErrorBody},
    modules::{
        auth::AuthUser,
        users::{
            models::{NewUser, RoleName, UpdateUser},
            repository::UserRepository,
        },
    },
    utils::password,
};

/// Registers a customer account. No login needed. New users always get the
/// `user` role; `manager` is granted separately via `POST /{id}/roles`.
#[utoipa::path(
    tag = "users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserResponse),
        (status = 400, description = "Invalid payload", body = ErrorBody),
        (status = 409, description = "Email is already registered", body = ErrorBody),
    )
)]
#[post("")]
async fn create_user(
    pool: web::Data<PgPool>,
    body: web::Json<CreateUserRequest>,
) -> AppResult<HttpResponse> {
    let body = body.into_inner();
    body.validate()?;

    let password_hash = password::hash(body.password).await?;

    let mut tx = pool.begin().await?;
    let id = UserRepository::create(
        &mut *tx,
        NewUser {
            email: body.email.trim(),
            password_hash: &password_hash,
            full_name: body.full_name.trim(),
            phone: body.phone.as_deref(),
        },
    )
    .await
    .map_err(|err| match AppError::from(err) {
        AppError::Conflict(_) => AppError::Conflict("email is already registered".into()),
        other => other,
    })?;
    UserRepository::assign_role(&mut *tx, id, RoleName::User).await?;
    tx.commit().await?;

    let user = fetch_user(&pool, id).await?;
    Ok(HttpResponse::Created().json(user))
}

/// Lists users, newest first, optionally filtered by role. Managers only.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(ListUsersQuery),
    responses(
        (status = 200, description = "One page of users", body = UserListResponse),
        (status = 400, description = "Invalid query", body = ErrorBody),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Not a manager", body = ErrorBody),
    )
)]
#[get("")]
async fn list_users(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    query: web::Query<ListUsersQuery>,
) -> AppResult<HttpResponse> {
    auth.require_manager()?;
    let (page, per_page) = (query.page(), query.per_page());
    let offset = i64::from(page - 1) * i64::from(per_page);

    let users = UserRepository::list(pool.get_ref(), query.role, per_page.into(), offset).await?;

    Ok(HttpResponse::Ok().json(UserListResponse {
        data: users.into_iter().map(UserResponse::from).collect(),
        page,
        per_page,
    }))
}

/// Fetches one user. Your own account, or any account as a manager.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 200, description = "The user", body = UserResponse),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Someone else's account", body = ErrorBody),
        (status = 404, description = "No such user", body = ErrorBody),
    )
)]
#[get("/{id}")]
async fn get_user(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    id: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = id.into_inner();
    auth.require_self_or_manager(id)?;
    let user = fetch_user(&pool, id).await?;
    Ok(HttpResponse::Ok().json(user))
}

/// Updates the given fields; omitted fields are left unchanged. Your own
/// account, or any account as a manager. Only managers may set `is_active`.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "User id")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "The updated user", body = UserResponse),
        (status = 400, description = "Invalid payload", body = ErrorBody),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Someone else's account, or `is_active` set by a non-manager", body = ErrorBody),
        (status = 404, description = "No such user", body = ErrorBody),
    )
)]
#[patch("/{id}")]
async fn update_user(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    id: web::Path<Uuid>,
    body: web::Json<UpdateUserRequest>,
) -> AppResult<HttpResponse> {
    let id = id.into_inner();
    auth.require_self_or_manager(id)?;
    if body.is_active.is_some() && !auth.is_manager() {
        return Err(AppError::Forbidden(
            "only managers can change is_active".into(),
        ));
    }
    body.validate()?;

    let changes = UpdateUser {
        full_name: body.full_name.as_deref().map(str::trim),
        phone: body.phone.as_deref(),
        is_active: body.is_active,
    };
    if !UserRepository::update(pool.get_ref(), id, changes).await? {
        return Err(AppError::NotFound);
    }

    let user = fetch_user(&pool, id).await?;
    Ok(HttpResponse::Ok().json(user))
}

/// Deletes a user and their role assignments. Your own account, or any
/// account as a manager.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "User id")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Someone else's account", body = ErrorBody),
        (status = 404, description = "No such user", body = ErrorBody),
    )
)]
#[delete("/{id}")]
async fn delete_user(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    id: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let id = id.into_inner();
    auth.require_self_or_manager(id)?;
    if !UserRepository::delete(pool.get_ref(), id).await? {
        return Err(AppError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}

/// Grants a role. Granting a role the user already has is a no-op.
/// Managers only.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "User id")),
    request_body = AssignRoleRequest,
    responses(
        (status = 200, description = "The user with the role added", body = UserResponse),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Not a manager", body = ErrorBody),
        (status = 404, description = "No such user", body = ErrorBody),
    )
)]
#[post("/{id}/roles")]
async fn assign_role(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    id: web::Path<Uuid>,
    body: web::Json<AssignRoleRequest>,
) -> AppResult<HttpResponse> {
    auth.require_manager()?;
    let id = id.into_inner();
    // Check first so a missing user is a 404 rather than an FK violation.
    fetch_user(&pool, id).await?;

    UserRepository::assign_role(pool.get_ref(), id, body.role).await?;

    let user = fetch_user(&pool, id).await?;
    Ok(HttpResponse::Ok().json(user))
}

/// Revokes a role. Managers only.
#[utoipa::path(
    tag = "users",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "User id"),
        ("role" = RoleName, Path, description = "Role to remove"),
    ),
    responses(
        (status = 200, description = "The user with the role removed", body = UserResponse),
        (status = 401, description = "Not logged in", body = ErrorBody),
        (status = 403, description = "Not a manager", body = ErrorBody),
        (status = 404, description = "No such user, or user lacks the role", body = ErrorBody),
    )
)]
#[delete("/{id}/roles/{role}")]
async fn remove_role(
    auth: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<(Uuid, RoleName)>,
) -> AppResult<HttpResponse> {
    auth.require_manager()?;
    let (id, role) = path.into_inner();
    if !UserRepository::remove_role(pool.get_ref(), id, role).await? {
        return Err(AppError::NotFound);
    }

    let user = fetch_user(&pool, id).await?;
    Ok(HttpResponse::Ok().json(user))
}

async fn fetch_user(pool: &PgPool, id: Uuid) -> AppResult<UserResponse> {
    UserRepository::find_by_id(pool, id)
        .await?
        .map(UserResponse::from)
        .ok_or(AppError::NotFound)
}
