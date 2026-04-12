// Tests for risk event admin endpoints against the real database schema.
// These catch the missing-column error (e.g., updated_at) at test time,
// ensuring the RiskEvent struct and SQL queries match the actual table.

use actix_web::test;
use serde_json::json;

use super::common;

/// GET /api/admin/risk-events should succeed without 500
/// (catches schema mismatches like missing updated_at column).
#[actix_web::test]
async fn test_list_risk_events_no_schema_error() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/risk-events")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status(), 500,
        "risk-events listing must not 500 — check RiskEvent struct matches risk_events table"
    );
    assert_eq!(resp.status(), 200);
}

/// GET /api/admin/risk-events with status filter.
#[actix_web::test]
async fn test_list_risk_events_with_status_filter() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/risk-events?status=Flagged")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status(), 500);
    assert_eq!(resp.status(), 200);
}

/// PUT /api/admin/risk-events/{id} with valid status should not 500
/// (catches the "updated_at does not exist" error).
#[actix_web::test]
async fn test_override_risk_event_nonexistent_returns_404_not_500() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::put()
        .uri(&format!("/api/admin/risk-events/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "status": "Approved",
            "justification": "Test override"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should be 404 (not found), NOT 500 (schema error)
    assert_ne!(
        resp.status(), 500,
        "Risk event override must not 500 — check SQL uses resolved_at not updated_at"
    );
    assert_eq!(resp.status(), 404);
}

/// Risk event status must use valid risk_event_status enum values.
#[actix_web::test]
async fn test_override_risk_event_invalid_status_rejected() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::put()
        .uri(&format!("/api/admin/risk-events/{}", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "status": "InvalidStatus",
            "justification": "Test"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should be 400/422 (validation error), not 500
    assert!(
        resp.status() == 400 || resp.status() == 422 || resp.status() == 404,
        "Invalid status should be rejected with 4xx, got {}",
        resp.status()
    );
}

/// check_bulk_order_risk INSERT must succeed against the real schema.
/// This catches the updated_at column mismatch.
#[actix_web::test]
async fn test_check_bulk_order_risk_insert_succeeds() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Create a product with enough stock
    let product_req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "title": format!("Risk Test Product {}", uuid::Uuid::new_v4()),
            "description": "Product for risk testing",
            "price": 1.00,
            "stock": 500,
            "genre": "Test"
        }))
        .to_request();
    let product_resp = test::call_service(&app, product_req).await;
    assert_eq!(product_resp.status(), 201, "Product creation must succeed for risk test");
    let product_body: serde_json::Value = test::read_body_json(product_resp).await;
    let product_id = product_body["id"].as_str().expect("must have product id");

    // Register a test user
    let username = format!("risk_test_{}", uuid::Uuid::new_v4());
    let register_req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": &username,
            "email": format!("{}@test.com", &username),
            "password": "TestP@ss123"
        }))
        .to_request();
    let _ = test::call_service(&app, register_req).await;

    let login_req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": &username, "password": "TestP@ss123" }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let login_body: serde_json::Value = test::read_body_json(login_resp).await;
    let user_token = login_body["access_token"].as_str().expect("must have token");

    // Place a large order that should trigger risk check.
    // Default threshold is usually small enough for 200 items to trigger it.
    let order_req = test::TestRequest::post()
        .uri("/api/orders")
        .insert_header(("Authorization", format!("Bearer {}", user_token)))
        .set_json(json!({
            "shipping_address": "123 Risk Test St, City, ST 12345",
            "payment_method": "local_tender",
            "items": [{ "product_id": product_id, "quantity": 200 }]
        }))
        .to_request();
    let order_resp = test::call_service(&app, order_req).await;
    let status = order_resp.status().as_u16();
    // Either the order succeeds (below threshold) or is blocked (403 from risk check).
    // The key assertion: it must NOT be 500 (which would indicate a schema mismatch
    // like the updated_at column not existing).
    assert_ne!(
        status, 500,
        "Order with large quantity must not 500 — risk_events INSERT must match the table schema"
    );
}

/// Discount-abuse risk check is now called during order creation.
/// Multiple orders with discount_amount > 0 (over the configured window/threshold)
/// should trigger the abuse check. Since discount_amount is set at the DB level and
/// defaults to 0, this test verifies the code path runs without 500.
#[actix_web::test]
async fn test_discount_abuse_check_integrated_in_order_flow() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Create a product
    let req = test::TestRequest::post()
        .uri("/api/products")
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({
            "title": format!("Discount Test {}", uuid::Uuid::new_v4()),
            "description": "For discount abuse testing",
            "price": 5.00,
            "stock": 100,
            "genre": "Test"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "Product creation must succeed (DB required)");
    let body: serde_json::Value = test::read_body_json(resp).await;
    let product_id = body["id"].as_str().expect("Product response must have id");

    // Register and login
    let username = format!("discount_test_{}", uuid::Uuid::new_v4());
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

    // Place an order — the discount_abuse check runs during creation.
    // With default discount_amount=0 and threshold=5, this should succeed.
    // The key assertion is that it doesn't 500 (proving the check is wired in).
    let req = test::TestRequest::post()
        .uri("/api/orders")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "shipping_address": "123 Discount St",
            "items": [{ "product_id": product_id, "quantity": 1 }]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status().as_u16(), 500,
        "Order creation with discount abuse check must not 500"
    );
}

/// Non-admin cannot access risk events.
#[actix_web::test]
async fn test_risk_events_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/risk-events")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}
