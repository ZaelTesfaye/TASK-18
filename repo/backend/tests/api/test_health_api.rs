use actix_web::{test, web, App};
use silverscreen_backend::config::Config;
use silverscreen_backend::middleware::rate_limit::RateLimiter;
use silverscreen_backend::routes::configure_routes;
use sqlx::PgPool;

/// Builds a test app that mirrors main.rs exactly: production health handler
/// + configure_routes. This is NOT a test-local substitute — it calls
/// `silverscreen_backend::health`, the same function main.rs mounts.
async fn create_test_app_with_health(
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let config = Config::get();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let rate_limiter = web::Data::new(RateLimiter::new());

    test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(rate_limiter)
            .route("/health", web::get().to(silverscreen_backend::health))
            .configure(configure_routes),
    )
    .await
}

/// GET /health — returns 200 with {"status":"ok"} from the production handler.
#[actix_web::test]
async fn test_health_endpoint() {
    let app = create_test_app_with_health().await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"].as_str().unwrap(), "ok", "Health must return status:ok");
    assert!(body.as_object().unwrap().contains_key("status"), "Response must have a 'status' key");
}

/// GET /health — does not require authentication (no Bearer header).
#[actix_web::test]
async fn test_health_endpoint_no_auth_needed() {
    let app = create_test_app_with_health().await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status().as_u16(), 401, "Health endpoint must not require authentication");
    assert_eq!(resp.status(), 200);
}
