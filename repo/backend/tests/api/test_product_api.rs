use actix_web::test;
use serde_json::json;

use super::common;

/// GET /api/products - list products (no auth required).
#[actix_web::test]
async fn test_list_products_public() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get()
        .uri("/api/products?page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array());
    assert!(body["total"].is_number());
    assert!(body["page"].is_number());
}

/// POST /api/products - create product (admin only).
#[actix_web::test]
async fn test_create_product_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Test Movie",
            "description": "A test movie for unit testing",
            "price": 19.99,
            "stock": 100,
            "genre": "Action",
            "release_year": 2024
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
}

/// POST /api/products - create product forbidden for shopper (403).
#[actix_web::test]
async fn test_create_product_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Unauthorized Product",
            "price": 10.0,
            "stock": 5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

/// GET /api/products - faceted filtering.
#[actix_web::test]
async fn test_list_products_with_filters() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/products?genre=Action&min_price=5&max_price=50&page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/products - search filter.
#[actix_web::test]
async fn test_list_products_search() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/products?search=movie&page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/products/:id - not found (404).
#[actix_web::test]
async fn test_get_product_not_found() {
    let app = common::create_test_app().await;
    let fake_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/products/{}", fake_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}
