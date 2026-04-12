use actix_web::test;
use serde_json::json;

use super::common;

/// GET /api/cart - requires authentication (401).
#[actix_web::test]
async fn test_get_cart_unauthenticated() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get()
        .uri("/api/cart")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// GET /api/cart - authenticated user can access cart.
#[actix_web::test]
async fn test_get_cart_authenticated() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/cart")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // 200 (has cart) or 200 with empty items.
    assert_eq!(resp.status(), 200);
}

/// POST /api/cart/items - add item to cart (requires auth).
#[actix_web::test]
async fn test_add_to_cart_unauthenticated() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/api/cart/items")
        .set_json(json!({
            "product_id": uuid::Uuid::new_v4(),
            "quantity": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// DELETE /api/cart - clear cart (requires auth).
#[actix_web::test]
async fn test_clear_cart_unauthenticated() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::delete()
        .uri("/api/cart")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// POST /api/cart/items - adding more than available stock when item already
/// exists in cart should fail with 409 (Conflict), not silently exceed stock.
#[actix_web::test]
async fn test_add_to_cart_exceeds_stock_on_increment() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Create a product with limited stock
    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "title": format!("Cart Stock Test {}", uuid::Uuid::new_v4()),
            "description": "Limited stock product",
            "price": 5.00,
            "stock": 3,
            "genre": "Test"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "Product creation must succeed (DB required)");
    let body: serde_json::Value = test::read_body_json(resp).await;
    let product_id = body["id"].as_str().expect("Product response must have id");

    // Register a test user
    let username = format!("cart_stock_{}", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": &username,
            "email": format!("{}@test.com", &username),
            "password": "TestP@ss123"
        }))
        .to_request();
    let _ = test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": &username, "password": "TestP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let login_body: serde_json::Value = test::read_body_json(resp).await;
    let token = login_body["access_token"].as_str().expect("Login must return access_token");

    // First add: 2 items (OK, stock = 3)
    let req = test::TestRequest::post()
        .uri("/api/cart/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "product_id": product_id, "quantity": 2 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "First add of 2 should succeed (stock=3)");

    // Second add: 2 more items (should fail: 2 + 2 = 4 > 3 stock)
    let req = test::TestRequest::post()
        .uri("/api/cart/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "product_id": product_id, "quantity": 2 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 409,
        "Adding 2 more to cart (already has 2, stock=3) must fail with 409 Conflict"
    );
}

/// POST /api/cart/items - invalid product (404 or 400).
#[actix_web::test]
async fn test_add_nonexistent_product_to_cart() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/cart/items")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "product_id": uuid::Uuid::new_v4(),
            "quantity": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status() == 404 || resp.status() == 400);
}
