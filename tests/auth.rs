//! Auth API v1 tests. See `tests/users.rs` for the database requirements.

mod common;

use actix_web::{http::StatusCode, test};
use common::{app, bearer, jwt_keys, login, register, PASSWORD};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn login_returns_a_token_for_me(pool: PgPool) {
    let app = app(pool).await;
    let user = register(&app, "judy@example.com").await;

    // Email matching is case-insensitive, like registration.
    let body: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({ "email": "JUDY@example.com", "password": PASSWORD }))
            .to_request(),
    )
    .await;
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 300);
    assert_eq!(body["user"]["id"], user["id"]);
    let token = body["access_token"].as_str().unwrap();

    let me: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(bearer(token))
            .to_request(),
    )
    .await;
    assert_eq!(me["email"], "judy@example.com");
    assert_eq!(me["roles"], json!(["user"]));
}

#[sqlx::test]
async fn wrong_password_and_unknown_email_look_the_same(pool: PgPool) {
    let app = app(pool).await;
    register(&app, "kim@example.com").await;

    for (email, password) in [
        ("kim@example.com", "wrong password!"),
        ("nobody@example.com", PASSWORD),
    ] {
        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/api/v1/auth/login")
                .set_json(json!({ "email": email, "password": password }))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "unauthorized: invalid email or password");
    }
}

#[sqlx::test]
async fn expired_and_forged_tokens_are_rejected(pool: PgPool) {
    let app = app(pool).await;
    let user = register(&app, "leo@example.com").await;
    let id: Uuid = user["id"].as_str().unwrap().parse().unwrap();

    // Past the validator's 60 s clock-skew leeway.
    let expired = jwt_keys(chrono::Duration::minutes(-5)).issue(id).unwrap();
    let forged = aws_app::modules::auth::JwtKeys::new(
        b"some-other-secret-that-is-32-bytes-long",
        chrono::Duration::minutes(5),
    )
    .issue(id)
    .unwrap();
    // A valid token with its last signature character changed.
    let mut tampered = login(&app, "leo@example.com").await;
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'A' { 'B' } else { 'A' });

    for token in [expired, forged, tampered] {
        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/v1/auth/me")
                .insert_header(bearer(&token))
                .to_request(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "unauthorized: invalid or expired token");
    }
}
