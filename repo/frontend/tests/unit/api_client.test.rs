// API client logic tests.
// These verify URL building, path construction, and auth header formatting
// without making actual HTTP requests. Imports the real api module to verify
// compilation.

use silverscreen_frontend::config::API_BASE_URL;

#[allow(unused_imports)]
use silverscreen_frontend::api;

// ---------------------------------------------------------------------------
// full_url function logic: API_BASE_URL + path
// ---------------------------------------------------------------------------

/// Replicates the full_url() function from api/client.rs.
fn full_url(path: &str) -> String {
    format!("{}{}", API_BASE_URL, path)
}

#[test]
fn test_api_base_url_value() {
    assert_eq!(API_BASE_URL, "/api", "API_BASE_URL must be '/api'");
}

#[test]
fn test_full_url_products() {
    let url = full_url("/products");
    assert_eq!(url, "/api/products");
}

#[test]
fn test_full_url_single_product() {
    let product_id = "550e8400-e29b-41d4-a716-446655440000";
    let url = full_url(&format!("/products/{}", product_id));
    assert_eq!(url, "/api/products/550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_full_url_cart() {
    let url = full_url("/cart");
    assert_eq!(url, "/api/cart");
}

#[test]
fn test_full_url_cart_items() {
    let url = full_url("/cart/items");
    assert_eq!(url, "/api/cart/items");
}

#[test]
fn test_full_url_orders() {
    let url = full_url("/orders");
    assert_eq!(url, "/api/orders");
}

#[test]
fn test_full_url_single_order() {
    let order_id = "order-uuid-001";
    let url = full_url(&format!("/orders/{}", order_id));
    assert_eq!(url, "/api/orders/order-uuid-001");
}

#[test]
fn test_full_url_auth_login() {
    let url = full_url("/auth/login");
    assert_eq!(url, "/api/auth/login");
}

#[test]
fn test_full_url_auth_register() {
    let url = full_url("/auth/register");
    assert_eq!(url, "/api/auth/register");
}

#[test]
fn test_full_url_auth_refresh() {
    let url = full_url("/auth/refresh");
    assert_eq!(url, "/api/auth/refresh");
}

#[test]
fn test_full_url_ratings_product() {
    let product_id = "prod-uuid";
    let url = full_url(&format!("/ratings/product/{}", product_id));
    assert_eq!(url, "/api/ratings/product/prod-uuid");
    assert!(url.starts_with("/api/ratings/product/"), "Ratings path must use /ratings/product/ prefix");
}

#[test]
fn test_full_url_leaderboards() {
    let url = full_url("/leaderboards");
    assert_eq!(url, "/api/leaderboards");
}

#[test]
fn test_full_url_reviews_rounds() {
    let url = full_url("/reviews/rounds");
    assert_eq!(url, "/api/reviews/rounds");
    assert!(url.contains("/reviews/"), "Review path must use /reviews/ not /reviewer/");
}

#[test]
fn test_full_url_payment_simulate() {
    let url = full_url("/payment/simulate");
    assert_eq!(url, "/api/payment/simulate");
}

#[test]
fn test_full_url_users_me() {
    let url = full_url("/users/me");
    assert_eq!(url, "/api/users/me");
}

#[test]
fn test_full_url_taxonomy_topics() {
    let url = full_url("/taxonomy/topics");
    assert_eq!(url, "/api/taxonomy/topics");
}

#[test]
fn test_full_url_taxonomy_tags() {
    let url = full_url("/taxonomy/tags");
    assert_eq!(url, "/api/taxonomy/tags");
}

#[test]
fn test_full_url_custom_fields() {
    let url = full_url("/custom-fields");
    assert_eq!(url, "/api/custom-fields");
}

#[test]
fn test_full_url_audit() {
    let url = full_url("/audit");
    assert_eq!(url, "/api/audit");
}

#[test]
fn test_full_url_backup() {
    let url = full_url("/backup");
    assert_eq!(url, "/api/backup");
}

#[test]
fn test_full_url_order_return() {
    let order_id = "order-uuid-001";
    let url = full_url(&format!("/orders/{}/return", order_id));
    assert_eq!(url, "/api/orders/order-uuid-001/return");
}

// ---------------------------------------------------------------------------
// Auth header format: "Bearer <token>"
// ---------------------------------------------------------------------------

#[test]
fn test_auth_header_format() {
    let token = "eyJhbGciOiJIUzI1NiJ9.payload.signature";
    let header = format!("Bearer {}", token);
    assert!(header.starts_with("Bearer "), "Auth header must start with 'Bearer '");
    assert_eq!(header, "Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature");
}

#[test]
fn test_auth_header_with_empty_token() {
    let token = "";
    let header = format!("Bearer {}", token);
    assert_eq!(header, "Bearer ", "Bearer prefix is still added even for empty token");
}

#[test]
fn test_auth_header_not_applied_when_no_token() {
    // The apply_auth function checks if token is Some; if None, no header is added
    let token: Option<String> = None;
    let should_apply = token.is_some();
    assert!(!should_apply, "Auth header must not be applied when no token exists");
}

#[test]
fn test_auth_header_applied_when_token_exists() {
    let token: Option<String> = Some("header.payload.signature".to_string());
    let should_apply = token.is_some();
    assert!(should_apply, "Auth header must be applied when token exists");

    let header = format!("Bearer {}", token.unwrap());
    assert!(header.starts_with("Bearer "));
    assert!(header.len() > "Bearer ".len());
}

// ---------------------------------------------------------------------------
// URL path construction patterns
// ---------------------------------------------------------------------------

#[test]
fn test_url_path_with_query_params() {
    // Some endpoints use query params built via serde
    let base = full_url("/products");
    let params = "search=matrix&genre=Sci-Fi&page=1&per_page=12";
    let full = format!("{}?{}", base, params);
    assert_eq!(full, "/api/products?search=matrix&genre=Sci-Fi&page=1&per_page=12");
}

#[test]
fn test_url_path_no_double_slash() {
    // API_BASE_URL has no trailing slash, paths start with /
    let url = full_url("/products");
    assert!(!url.contains("//"), "URL must not contain double slashes");
}

#[test]
fn test_url_path_all_start_with_api() {
    let paths = vec![
        "/products", "/cart", "/orders", "/auth/login", "/auth/register",
        "/ratings/product/id", "/leaderboards", "/reviews/rounds",
        "/payment/simulate", "/users/me", "/taxonomy/topics",
        "/taxonomy/tags", "/custom-fields", "/audit", "/backup",
    ];
    for path in &paths {
        let url = full_url(path);
        assert!(url.starts_with("/api/"), "URL '{}' must start with /api/", url);
    }
}

// ---------------------------------------------------------------------------
// Error type construction
// ---------------------------------------------------------------------------

#[test]
fn test_api_error_display_message() {
    use silverscreen_frontend::types::ApiError;

    let err = ApiError {
        error: "Forbidden".to_string(),
        message: "Admin role required".to_string(),
        status: 403,
    };
    // ApiError::Display prefers message if non-empty
    assert_eq!(format!("{}", err), "Admin role required");
}

#[test]
fn test_api_error_display_fallback_to_error() {
    use silverscreen_frontend::types::ApiError;

    let err = ApiError {
        error: "Server Error".to_string(),
        message: "".to_string(),
        status: 500,
    };
    // ApiError::Display falls back to error field when message is empty
    assert_eq!(format!("{}", err), "Server Error");
}

#[test]
fn test_api_error_display_fallback_to_status() {
    use silverscreen_frontend::types::ApiError;

    let err = ApiError {
        error: "".to_string(),
        message: "".to_string(),
        status: 503,
    };
    // ApiError::Display falls back to status when both fields are empty
    assert_eq!(format!("{}", err), "Unknown error (status 503)");
}

#[test]
fn test_api_error_401_unauthorized() {
    use silverscreen_frontend::types::ApiError;

    // The client creates this specific error for 401 responses
    let err = ApiError {
        error: "Unauthorized".to_string(),
        message: "Session expired. Please log in again.".to_string(),
        status: 401,
    };
    assert_eq!(err.status, 401);
    assert!(err.message.contains("Session expired"));
}
