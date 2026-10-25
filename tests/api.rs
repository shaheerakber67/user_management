use actix_web::{body::to_bytes, http::StatusCode, test, App};
use aws_app::startup::configure_app;

#[actix_web::test]
async fn root_returns_hello_world() {
    let app = test::init_service(App::new().configure(configure_app)).await;
    let resp = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(to_bytes(resp.into_body()).await.unwrap(), "Hello World");
}

#[actix_web::test]
async fn echo_returns_request_body() {
    let app = test::init_service(App::new().configure(configure_app)).await;
    let req = test::TestRequest::post()
        .uri("/echo")
        .set_payload("ping")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(to_bytes(resp.into_body()).await.unwrap(), "ping");
}

#[actix_web::test]
async fn health_check_returns_ok() {
    let app = test::init_service(App::new().configure(configure_app)).await;
    let resp = test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_web::test]
async fn unknown_route_returns_json_404() {
    let app = test::init_service(App::new().configure(configure_app)).await;
    let resp = test::call_service(&app, test::TestRequest::get().uri("/nope").to_request()).await;

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "not found");
}

#[actix_web::test]
async fn openapi_spec_lists_all_routes() {
    let app = test::init_service(App::new().configure(configure_app)).await;
    let req = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let spec: serde_json::Value = test::read_body_json(resp).await;
    let mut paths: Vec<&str> = spec["paths"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    paths.sort_unstable();
    assert_eq!(
        paths,
        [
            "/",
            "/api/v1/auth/login",
            "/api/v1/auth/me",
            "/api/v1/users",
            "/api/v1/users/{id}",
            "/api/v1/users/{id}/roles",
            "/api/v1/users/{id}/roles/{role}",
            "/echo",
            "/health",
        ]
    );
}

#[actix_web::test]
async fn swagger_ui_is_served() {
    let app = test::init_service(App::new().configure(configure_app)).await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/swagger-ui").to_request(),
    )
    .await;
    assert!(resp.status().is_redirection());
    assert_eq!(resp.headers().get("location").unwrap(), "/swagger-ui/");

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/swagger-ui/").to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let html = to_bytes(resp.into_body()).await.unwrap();
    assert!(std::str::from_utf8(&html).unwrap().contains("Swagger UI"));
}
