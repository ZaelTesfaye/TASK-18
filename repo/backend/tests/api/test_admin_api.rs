use actix_web::test;
use serde_json::json;

use super::common;

/// GET /api/admin/users - requires admin role (403 for shopper).
#[actix_web::test]
async fn test_list_users_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/users?page=1&per_page=10")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// GET /api/admin/users - admin can list users.
#[actix_web::test]
async fn test_list_users_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/users?page=1&per_page=10")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array(), "User list must return paginated items array");
    assert!(body["total"].is_number(), "User list must include total count");
}

/// PUT /api/admin/users/:id/role - admin can change roles.
#[actix_web::test]
async fn test_change_user_role_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let user_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::put()
        .uri(&format!("/api/admin/users/{}/role", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "role": "Reviewer" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    // 404 (user not found) or 200; NOT 401/403.
    assert!(status != 401 && status != 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    if status == 404 {
        assert!(body["error"].is_string() || body["message"].is_string(), "404 must have error body");
    } else {
        assert!(body["id"].is_string() || body["role"].is_string(), "200 must return user data");
    }
}

/// PUT /api/admin/users/:id/role - forbidden for reviewer (403).
#[actix_web::test]
async fn test_change_user_role_forbidden_for_reviewer() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let user_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::put()
        .uri(&format!("/api/admin/users/{}/role", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "role": "Admin" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// POST /api/admin/users/:id/reset-password - admin only.
#[actix_web::test]
async fn test_reset_password_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let user_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/reset-password", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// POST /api/admin/users/:id/unlock - admin only.
#[actix_web::test]
async fn test_unlock_user_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let user_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/unlock", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// GET /api/admin/risk-events - admin only (403 for shopper).
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
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// GET /api/admin/risk-events - admin can access.
#[actix_web::test]
async fn test_risk_events_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/admin/risk-events")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array() || body.is_array(), "Risk events must return a list");
}

/// GET /api/audit - admin only (403 for shopper).
#[actix_web::test]
async fn test_audit_log_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/audit?page=1&per_page=10")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// GET /api/audit - admin can access.
#[actix_web::test]
async fn test_audit_log_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/audit?page=1&per_page=10")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array(), "Audit log must return paginated items array");
    assert!(body["total"].is_number(), "Audit log must include total count");
}

/// GET /api/reports - admin only.
#[actix_web::test]
async fn test_reports_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/reports?start_date=2024-01-01&end_date=2024-12-31&report_type=summary")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// POST /api/backup - admin only.
#[actix_web::test]
async fn test_backup_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/backup")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// POST /api/admin/retention/run - admin only.
#[actix_web::test]
async fn test_retention_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/admin/retention/run")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

// ---------------------------------------------------------------------------
// POST /api/admin/retention/legal-hold/{order_id}
// ---------------------------------------------------------------------------

/// POST /api/admin/retention/legal-hold/{order_id} - shopper forbidden (403).
#[actix_web::test]
async fn test_legal_hold_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let order_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/retention/legal-hold/{}", order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "hold": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "403 must have error body");
}

/// POST /api/admin/retention/legal-hold/{order_id} - nonexistent order returns 404.
#[actix_web::test]
async fn test_legal_hold_nonexistent_order() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_order_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/retention/legal-hold/{}", fake_order_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "hold": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "Legal hold on nonexistent order must return 404");
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string() || body["message"].is_string(), "404 must have error body");
}

/// POST /api/admin/retention/legal-hold/{order_id} - admin success on real order.
#[actix_web::test]
async fn test_legal_hold_admin_success() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Create a real user and order
    let user = common::register_and_login(&app, "legalhold").await;
    if user.is_none() { return; }
    let user = user.unwrap();
    let order_id = common::create_order_for_user(&app, &user.access_token).await;
    if order_id.is_none() { return; }
    let order_id = order_id.unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/retention/legal-hold/{}", order_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .set_json(json!({ "hold": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Admin should be able to set legal hold");
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["message"].is_string() || body["legal_hold"].is_boolean(),
        "Response must contain confirmation of legal hold action");
}

// ---------------------------------------------------------------------------
// GET /api/backup — admin can list backups (returns 200 with a list)
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_list_backups_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    let req = test::TestRequest::get()
        .uri("/api/backup")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body.is_array(), "GET /api/backup must return an array (even if empty)");
}
