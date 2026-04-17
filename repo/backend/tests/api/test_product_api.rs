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

// ---------------------------------------------------------------------------
// DELETE /api/products/:id — admin creates a product then deletes it
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_delete_product_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    // Create product
    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": format!("DeleteMe {}", uuid::Uuid::new_v4()),
            "description": "Product to be deleted",
            "price": 9.99,
            "stock": 10,
            "genre": "Action"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let product_id = body["id"].as_str().unwrap();

    // Delete product
    let req = test::TestRequest::delete()
        .uri(&format!("/api/products/{}", product_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // GET the deleted product — should return 404 (soft-deleted)
    let req = test::TestRequest::get()
        .uri(&format!("/api/products/{}", product_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "Deleted product must return 404");
}

// ---------------------------------------------------------------------------
// PUT /api/products/:id — admin creates a product then updates fields
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_update_product_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    // Create product
    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Original Title",
            "description": "Original description",
            "price": 19.99,
            "stock": 50,
            "genre": "Drama",
            "release_year": 2024
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let product_id = body["id"].as_str().unwrap();

    // Update product
    let req = test::TestRequest::put()
        .uri(&format!("/api/products/{}", product_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Updated Title",
            "description": "Updated description",
            "price": 24.99,
            "stock": 75
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let upd_body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(upd_body["title"].as_str().unwrap(), "Updated Title");
    assert_eq!(upd_body["description"].as_str().unwrap(), "Updated description");
    assert_eq!(upd_body["stock"].as_i64().unwrap(), 75);
}
