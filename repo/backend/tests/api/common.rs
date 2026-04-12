use actix_web::{test, web, App};
use sqlx::PgPool;

use silverscreen_backend::config::Config;
use silverscreen_backend::middleware::rate_limit::RateLimiter;
use silverscreen_backend::routes::configure_routes;
use silverscreen_backend::services::auth_service;

/// Creates a test application with a database pool and all routes configured.
///
/// NOTE: These tests require a running PostgreSQL instance.
/// In CI/Docker, the database is automatically available.
/// For local development, set DATABASE_URL environment variable.
pub async fn create_test_app(
) -> impl actix_web::dev::Service<actix_http::Request, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error>
{
    let config = Config::get();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let rate_limiter = web::Data::new(RateLimiter::new());

    test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(rate_limiter)
            .configure(configure_routes),
    )
    .await
}

/// Generates an admin JWT for testing protected endpoints.
pub fn admin_token() -> String {
    let config = Config::get();
    auth_service::generate_access_token(
        uuid::Uuid::new_v4(),
        "Admin",
        &config.jwt_secret,
        30,
    )
    .unwrap()
}

/// Generates a shopper JWT for testing.
pub fn shopper_token() -> String {
    let config = Config::get();
    auth_service::generate_access_token(
        uuid::Uuid::new_v4(),
        "Shopper",
        &config.jwt_secret,
        30,
    )
    .unwrap()
}

/// Generates a reviewer JWT for testing.
pub fn reviewer_token() -> String {
    let config = Config::get();
    auth_service::generate_access_token(
        uuid::Uuid::new_v4(),
        "Reviewer",
        &config.jwt_secret,
        30,
    )
    .unwrap()
}

/// Generates a token for a specific user_id with a given role.
pub fn token_for_user(user_id: uuid::Uuid, role: &str) -> String {
    let config = Config::get();
    auth_service::generate_access_token(user_id, role, &config.jwt_secret, 30).unwrap()
}

/// Generates an expired JWT for testing 401 responses.
pub fn expired_token() -> String {
    let config = Config::get();
    // Generate with negative expiry to create already-expired token.
    auth_service::generate_access_token(
        uuid::Uuid::new_v4(),
        "Shopper",
        &config.jwt_secret,
        -1,
    )
    .unwrap_or_default()
}

/// Result of registering and logging in a test user.
pub struct TestUser {
    pub user_id: String,
    pub access_token: String,
    pub refresh_token: String,
}

/// Registers a new user with a unique username, logs them in, and returns
/// their access token, refresh token, and user ID. Returns None if the
/// database is unavailable.
pub async fn register_and_login(
    app: &impl actix_web::dev::Service<actix_http::Request, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error>,
    role_suffix: &str,
) -> Option<TestUser> {
    let username = format!("test_{}_{}", role_suffix, uuid::Uuid::new_v4());
    let email = format!("{}@test.com", username);
    let password = "SecureP@ss123";

    // Register
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(serde_json::json!({
            "username": username,
            "email": email,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() != 201 {
        return None; // DB not available
    }

    // Login
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() != 200 {
        return None;
    }
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Get user ID from /users/me
    let access = body["access_token"].as_str()?.to_string();
    let refresh = body["refresh_token"].as_str()?.to_string();

    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access)))
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() != 200 {
        return None;
    }
    let me: serde_json::Value = test::read_body_json(resp).await;
    let user_id = me["id"].as_str()?.to_string();

    Some(TestUser {
        user_id,
        access_token: access,
        refresh_token: refresh,
    })
}

/// Creates a real order for a user via the API. Returns the order ID if
/// successful, or None if there are no products in stock.
pub async fn create_order_for_user(
    app: &impl actix_web::dev::Service<actix_http::Request, Response = actix_web::dev::ServiceResponse, Error = actix_web::Error>,
    access_token: &str,
) -> Option<String> {
    // Find a product with stock
    let req = test::TestRequest::get()
        .uri("/api/products?page=1&per_page=1")
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() != 200 { return None; }
    let body: serde_json::Value = test::read_body_json(resp).await;
    let items = body["items"].as_array()?;
    if items.is_empty() { return None; }
    let product_id = items[0]["id"].as_str()?;

    // Create order
    let req = test::TestRequest::post()
        .uri("/api/orders")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(serde_json::json!({
            "shipping_address": "123 Test St, Test City, TS 12345",
            "payment_method": "CreditCard",
            "items": [{ "product_id": product_id, "quantity": 1 }]
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    if resp.status() != 201 { return None; }
    let order: serde_json::Value = test::read_body_json(resp).await;
    order["id"].as_str().map(|s| s.to_string())
}
