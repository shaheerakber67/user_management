use actix_web::{get, HttpResponse, Responder};
use serde_json::json;

/// Liveness check.
#[utoipa::path(
    tag = "system",
    responses((status = 200, description = "Service is up", example = json!({ "status": "ok" })))
)]
#[get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}
