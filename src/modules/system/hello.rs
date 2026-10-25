use actix_web::{get, HttpResponse, Responder};

#[utoipa::path(
    tag = "system",
    responses((status = 200, description = "Greeting", body = String, content_type = "text/plain"))
)]
#[get("/")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World")
}
