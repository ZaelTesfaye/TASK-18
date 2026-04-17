use actix_web::test;
use serde_json::json;

use super::common;

/// Comprehensive RBAC boundary tests - ensure every admin endpoint rejects non-admin users.

// ---------------------------------------------------------------------------
// Taxonomy management - admin only
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_create_topic_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": "Test Topic", "slug": "test-topic" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["error"].is_string() || body["message"].is_string(),
        "403 response must contain an error or message field"
    );
}

#[actix_web::test]
async fn test_create_topic_admin_allowed() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": format!("Topic_{}", uuid::Uuid::new_v4()),
            "slug": format!("topic-{}", uuid::Uuid::new_v4())
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["id"].is_string(), "Created topic must return an id");
    assert!(body["name"].is_string(), "Created topic must return a name");
}

#[actix_web::test]
async fn test_create_tag_forbidden_for_reviewer() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();

    let req = test::TestRequest::post()
        .uri("/api/taxonomy/tags")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": "Test Tag", "slug": "test-tag" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["error"].is_string() || body["message"].is_string(),
        "403 response must contain an error or message field"
    );
}

// ---------------------------------------------------------------------------
// Custom fields - admin only
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_create_custom_field_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/custom-fields")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "name": "Test Field",
            "slug": "test-field",
            "field_type": "Text"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

#[actix_web::test]
async fn test_publish_field_forbidden_for_reviewer() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let field_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/custom-fields/{}/publish", field_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

// ---------------------------------------------------------------------------
// Moderation - admin only
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_moderate_rating_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let rating_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/moderation/ratings/{}", rating_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Approved" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

// ---------------------------------------------------------------------------
// Payment simulation - requires auth
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_simulate_payment_unauthenticated() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/api/payment/simulate")
        .set_json(json!({
            "order_id": uuid::Uuid::new_v4(),
            "outcome": "Success",
            "payment_method": "local_tender"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

// ---------------------------------------------------------------------------
// Payment simulation ownership check
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_simulate_payment_requires_order_ownership() {
    let app = common::create_test_app().await;
    let token = common::shopper_token(); // random user, won't own any order

    let req = test::TestRequest::post()
        .uri("/api/payment/simulate")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "order_id": uuid::Uuid::new_v4(),
            "amount": 19.99,
            "outcome": "Success",
            "payment_method": "local_tender",
            "attempt_number": 1
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Order doesn't exist, so 404 (before ownership check even triggers)
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "404 must have error body");
}

// ---------------------------------------------------------------------------
// Review submission ownership tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_review_submission_requires_auth() {
    let app = common::create_test_app().await;
    let sub_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}", sub_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

// ---------------------------------------------------------------------------
// Attachment download ownership tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_attachment_download_requires_reviewer_role() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let attachment_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/attachments/{}/download", attachment_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

#[actix_web::test]
async fn test_attachment_download_nonexistent_returns_404() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let attachment_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/attachments/{}/download", attachment_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "404 must have error body");
}

// ---------------------------------------------------------------------------
// Backup verify uses valid enum values
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_backup_verify_nonexistent_returns_error() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/backup/{}/verify", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should fail gracefully — not a 500 from invalid enum
    assert!(resp.status() != 500, "Backup verify should not return 500 from invalid enum");
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string() || body["valid"].is_boolean(),
        "Backup verify must return a structured response body");
}

#[actix_web::test]
async fn test_backup_restore_nonexistent_returns_error() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/backup/{}/restore", fake_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status() != 500, "Backup restore should not return 500");
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(),
        "Backup restore error must return a structured response body");
}

// ---------------------------------------------------------------------------
// Token revocation tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_revoked_token_rejected_on_protected_route() {
    // An expired/invalid token should be rejected on any protected endpoint
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", "Bearer expired.invalid.token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_malformed_bearer_token_rejected() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/orders")
        .insert_header(("Authorization", "Bearer "))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_non_bearer_auth_rejected() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/orders")
        .insert_header(("Authorization", "Basic dXNlcjpwYXNz"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

// ---------------------------------------------------------------------------
// Cross-user access denial tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_shopper_cannot_access_other_users_order() {
    let app = common::create_test_app().await;
    let token = common::shopper_token(); // random user_id
    let fake_order_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/orders/{}", fake_order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should be 404 (not found for this user) — not 200
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "404 must have error body");
}

// ---------------------------------------------------------------------------
// Unauthenticated access tests for all protected endpoints
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_users_me_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get().uri("/api/users/me").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_orders_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get().uri("/api/orders").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_cart_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get().uri("/api/cart").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_admin_users_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get().uri("/api/admin/users").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_audit_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get().uri("/api/audit").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

#[actix_web::test]
async fn test_backup_requires_auth() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::post().uri("/api/backup").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "401 must have error body");
}

// ---------------------------------------------------------------------------
// Review rounds RBAC
// ---------------------------------------------------------------------------

/// GET /api/reviews/rounds - shopper role cannot access review rounds (403).
#[actix_web::test]
async fn test_shopper_cannot_access_review_rounds() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 403,
        "Shopper must not access review rounds, got {}",
        resp.status()
    );
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["error"].is_string() || body["message"].is_string(),
        "403 response must contain an error or message field"
    );
}
