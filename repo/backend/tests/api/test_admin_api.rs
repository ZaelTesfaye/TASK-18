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
    // 404 (user not found) or 200; NOT 401/403.
    assert!(resp.status() != 401 && resp.status() != 403);
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
}
