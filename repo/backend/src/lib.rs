pub mod config;
pub mod db;
pub mod errors;
pub mod logging;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;

/// Health-check endpoint handler.
///
/// Mounted at `/health` by main.rs. Exported here so integration tests can
/// exercise the real production handler instead of a test-local substitute.
pub async fn health() -> actix_web::HttpResponse {
    actix_web::HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}
