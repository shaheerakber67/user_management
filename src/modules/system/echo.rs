use actix_web::{post, HttpResponse, Responder};

/// Returns the request body unchanged.
#[utoipa::path(
    tag = "system",
    request_body(content = String, content_type = "text/plain"),
    responses((status = 200, description = "The request body", body = String, content_type = "text/plain"))
)]
#[post("/echo")]
pub async fn echo(body: String) -> impl Responder {
    HttpResponse::Ok().body(body)
}
