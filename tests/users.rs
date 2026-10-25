//! Users API v1 tests. Each `#[sqlx::test]` gets a fresh database with all
//! migrations applied, so `DATABASE_URL` must point at a Postgres server where
//! the user can create databases.

mod common;

use actix_web::{http::StatusCode, test};
use common::{app, bearer, login, manager, new_user, register};
use serde_json::{json, Value};
use sqlx::PgPool;

#[sqlx::test]
async fn roles_are_seeded(pool: PgPool) {
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM roles ORDER BY name")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(names, ["manager", "user"]);
}

#[sqlx::test]
async fn create_user_assigns_default_role_and_hides_password(pool: PgPool) {
    let app = app(pool.clone()).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(new_user("alice@example.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "alice@example.com");
    assert_eq!(body["roles"], json!(["user"]));
    assert!(body.get("password_hash").is_none());

    let stored: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(stored.starts_with("$argon2id$"));
}

#[sqlx::test]
async fn duplicate_email_is_case_insensitive_conflict(pool: PgPool) {
    let app = app(pool).await;
    register(&app, "bob@example.com").await;

    let second = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(new_user("BOB@example.com"))
        .to_request();
    let resp = test::call_service(&app, second).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test]
async fn invalid_payload_is_bad_request(pool: PgPool) {
    let app = app(pool).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/users")
        .set_json(json!({ "email": "nope", "password": "short", "full_name": "X" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "bad request: invalid email address");
}

#[sqlx::test]
async fn protected_routes_require_a_valid_token(pool: PgPool) {
    let app = app(pool).await;
    let user = register(&app, "frank@example.com").await;
    let id = user["id"].as_str().unwrap();

    for req in [
        test::TestRequest::get().uri("/api/v1/users"),
        test::TestRequest::get().uri(&format!("/api/v1/users/{id}")),
        test::TestRequest::delete().uri(&format!("/api/v1/users/{id}")),
        test::TestRequest::get()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer("not-a-jwt")),
    ] {
        let resp = test::call_service(&app, req.to_request()).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(resp.headers().get("www-authenticate").unwrap(), "Bearer");
    }
}

#[sqlx::test]
async fn users_can_only_access_their_own_account(pool: PgPool) {
    let app = app(pool).await;
    let gina = register(&app, "gina@example.com").await;
    let hank = register(&app, "hank@example.com").await;
    let token = login(&app, "gina@example.com").await;
    let own = gina["id"].as_str().unwrap();
    let other = hank["id"].as_str().unwrap();

    let get = |id: &str| {
        test::TestRequest::get()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer(&token))
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, get(own)).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        test::call_service(&app, get(other)).await.status(),
        StatusCode::FORBIDDEN
    );

    let manager_only = [
        test::TestRequest::get().uri("/api/v1/users"),
        test::TestRequest::post()
            .uri(&format!("/api/v1/users/{own}/roles"))
            .set_json(json!({ "role": "manager" })),
        test::TestRequest::delete().uri(&format!("/api/v1/users/{own}/roles/user")),
        test::TestRequest::patch()
            .uri(&format!("/api/v1/users/{own}"))
            .set_json(json!({ "is_active": false })),
        test::TestRequest::delete().uri(&format!("/api/v1/users/{other}")),
    ];
    for req in manager_only {
        let resp = test::call_service(&app, req.insert_header(bearer(&token)).to_request()).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}

#[sqlx::test]
async fn manager_role_can_be_assigned_filtered_and_removed(pool: PgPool) {
    let app = app(pool.clone()).await;
    let (_, boss) = manager(&app, &pool, "boss@example.com").await;
    let carol = register(&app, "carol@example.com").await;
    let id = carol["id"].as_str().unwrap();
    register(&app, "dave@example.com").await;

    let promoted: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/users/{id}/roles"))
            .insert_header(bearer(&boss))
            .set_json(json!({ "role": "manager" }))
            .to_request(),
    )
    .await;
    assert_eq!(promoted["roles"], json!(["manager", "user"]));

    let managers: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::get()
            .uri("/api/v1/users?role=manager")
            .insert_header(bearer(&boss))
            .to_request(),
    )
    .await;
    let mut emails: Vec<&str> = managers["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["email"].as_str().unwrap())
        .collect();
    emails.sort_unstable();
    assert_eq!(emails, ["boss@example.com", "carol@example.com"]);

    let demoted: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/users/{id}/roles/manager"))
            .insert_header(bearer(&boss))
            .to_request(),
    )
    .await;
    assert_eq!(demoted["roles"], json!(["user"]));
}

#[sqlx::test]
async fn users_update_and_delete_their_own_account(pool: PgPool) {
    let app = app(pool.clone()).await;
    let (_, boss) = manager(&app, &pool, "boss@example.com").await;
    let erin = register(&app, "erin@example.com").await;
    let id = erin["id"].as_str().unwrap();
    let token = login(&app, "erin@example.com").await;

    let updated: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer(&token))
            .set_json(json!({ "full_name": "Erin Updated" }))
            .to_request(),
    )
    .await;
    assert_eq!(updated["full_name"], "Erin Updated");
    assert_eq!(updated["phone"], "+15550001111");

    let resp = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer(&token))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let mappings: i64 =
        sqlx::query_scalar("SELECT count(*) FROM user_roles WHERE user_id = $1::uuid")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mappings, 0, "role mappings should cascade on delete");

    // The deleted user's token stops working; the manager sees a 404.
    let get = |token: &str| {
        test::TestRequest::get()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer(token))
            .to_request()
    };
    assert_eq!(
        test::call_service(&app, get(&token)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        test::call_service(&app, get(&boss)).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test]
async fn deactivated_users_are_locked_out(pool: PgPool) {
    let app = app(pool.clone()).await;
    let (_, boss) = manager(&app, &pool, "boss@example.com").await;
    let ivan = register(&app, "ivan@example.com").await;
    let id = ivan["id"].as_str().unwrap();
    let token = login(&app, "ivan@example.com").await;

    let updated: Value = test::call_and_read_body_json(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/v1/users/{id}"))
            .insert_header(bearer(&boss))
            .set_json(json!({ "is_active": false }))
            .to_request(),
    )
    .await;
    assert_eq!(updated["is_active"], false);

    // The existing token is rejected straight away, and so is a new login.
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/v1/auth/me")
            .insert_header(bearer(&token))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .set_json(json!({ "email": "ivan@example.com", "password": common::PASSWORD }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn malformed_id_is_bad_request(pool: PgPool) {
    let app = app(pool.clone()).await;
    let (_, boss) = manager(&app, &pool, "boss@example.com").await;
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/v1/users/not-a-uuid")
            .insert_header(bearer(&boss))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
