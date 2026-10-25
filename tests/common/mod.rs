//! Helpers shared by the database-backed integration tests.

#![allow(dead_code)] // each test binary uses a different subset

use actix_web::{
    dev::{Service, ServiceResponse},
    test, web, App,
};
use aws_app::{modules::auth::JwtKeys, startup::configure_app};
use serde_json::{json, Value};
use sqlx::PgPool;

pub const PASSWORD: &str = "correct horse battery";
pub const JWT_SECRET: &[u8] = b"test-secret-that-is-at-least-32-bytes-long";

pub type Request = actix_http::Request;

pub fn jwt_keys(ttl: chrono::Duration) -> JwtKeys {
    JwtKeys::new(JWT_SECRET, ttl)
}

pub async fn app(
    pool: PgPool,
) -> impl Service<Request, Response = ServiceResponse, Error = actix_web::Error> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(jwt_keys(chrono::Duration::minutes(5))))
            .configure(configure_app),
    )
    .await
}

pub fn new_user(email: &str) -> Value {
    json!({
        "email": email,
        "password": PASSWORD,
        "full_name": "Test User",
        "phone": "+15550001111"
    })
}

/// Registers `email` and returns the created user.
pub async fn register<S>(app: &S, email: &str) -> Value
where
    S: Service<Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    test::call_and_read_body_json(
        app,
        test::TestRequest::post()
            .uri("/api/v1/users")
            .set_json(new_user(email))
            .to_request(),
    )
    .await
}

/// Logs in with [`PASSWORD`] and returns the access token.
pub async fn login<S>(app: &S, email: &str) -> String
where
    S: Service<Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let body: Value = test::call_and_read_body_json(
        app,
        test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({ "email": email, "password": PASSWORD }))
            .to_request(),
    )
    .await;
    body["access_token"]
        .as_str()
        .expect("login should return a token")
        .to_owned()
}

/// Grants `manager` directly in the database, the way the first manager
/// has to be bootstrapped.
pub async fn make_manager(pool: &PgPool, user_id: &str) {
    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) \
         SELECT $1::uuid, id FROM roles WHERE name = 'manager'",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();
}

/// Registers a manager and returns `(user, token)`.
pub async fn manager<S>(app: &S, pool: &PgPool, email: &str) -> (Value, String)
where
    S: Service<Request, Response = ServiceResponse, Error = actix_web::Error>,
{
    let user = register(app, email).await;
    make_manager(pool, user["id"].as_str().unwrap()).await;
    (user, login(app, email).await)
}

pub fn bearer(token: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {token}"))
}
