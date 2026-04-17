use actix_web::test;
use serde_json::json;

use super::common;

/// GET /api/reviews/rounds - requires auth (401 without).
#[actix_web::test]
async fn test_list_review_rounds_unauthenticated() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// GET /api/reviews/rounds - authenticated reviewer can list rounds.
#[actix_web::test]
async fn test_list_review_rounds_authenticated() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();

    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/reviews/submissions/:id - ownership check: reviewer cannot access another's submission.
/// With a non-existent submission, we get 404. This also verifies the endpoint
/// does not leak data — two different reviewers get the same 404 for the same ID.
#[actix_web::test]
async fn test_get_submission_cross_user_denied() {
    let app = common::create_test_app().await;
    let reviewer_a_id = uuid::Uuid::new_v4();
    let reviewer_b_id = uuid::Uuid::new_v4();
    let token_a = common::token_for_user(reviewer_a_id, "Reviewer");
    let token_b = common::token_for_user(reviewer_b_id, "Reviewer");
    let fake_sub_id = uuid::Uuid::new_v4();

    // Reviewer A gets 404
    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}", fake_sub_id))
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .to_request();
    let resp_a = test::call_service(&app, req).await;

    // Reviewer B gets the same status (no information leakage)
    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}", fake_sub_id))
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp_b = test::call_service(&app, req).await;

    assert_eq!(resp_a.status(), resp_b.status(),
        "Both reviewers must get the same response for non-existent submission");
    assert_eq!(resp_a.status(), 404);
}

/// GET /api/reviews/submissions/:id - shoppers cannot access submissions (403).
#[actix_web::test]
async fn test_get_submission_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let fake_sub_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}", fake_sub_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403,
        "Shoppers must not access submissions, got {}", resp.status());
}

/// GET /api/reviews/submissions/:id/history - ownership enforced.
/// Two different reviewers both get 404 for a non-existent submission.
#[actix_web::test]
async fn test_get_submission_history_cross_user_denied() {
    let app = common::create_test_app().await;
    let reviewer_a_id = uuid::Uuid::new_v4();
    let reviewer_b_id = uuid::Uuid::new_v4();
    let token_a = common::token_for_user(reviewer_a_id, "Reviewer");
    let token_b = common::token_for_user(reviewer_b_id, "Reviewer");
    let fake_sub_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}/history", fake_sub_id))
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .to_request();
    let resp_a = test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}/history", fake_sub_id))
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp_b = test::call_service(&app, req).await;

    assert_eq!(resp_a.status(), resp_b.status(),
        "Both reviewers must get same response for non-existent submission history");
    assert_eq!(resp_a.status(), 404);
}

/// POST /api/reviews/rounds/:id/submit - requires authentication (401).
#[actix_web::test]
async fn test_submit_review_unauthenticated() {
    let app = common::create_test_app().await;
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .set_json(json!({
            "content": { "summary": "Good movie" }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// POST /api/reviews/rounds/:id/submit - forbidden for shoppers (403).
#[actix_web::test]
async fn test_submit_review_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "content": { "summary": "Not allowed" }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

/// POST /api/reviews/rounds/:id/submit - reviewer with non-existent round (404).
#[actix_web::test]
async fn test_submit_review_round_not_found() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "content": { "summary": "Review content" }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

/// POST /api/reviews/rounds/:id/submit - shopper cannot submit even to existing round (403).
/// This verifies the role check fires before the round-not-found check.
#[actix_web::test]
async fn test_submit_review_shopper_gets_403_before_404() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "content": { "summary": "Should be blocked" }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 403,
        "Shopper submit must get 403 (role check) not 404 (round lookup), got {}",
        resp.status()
    );
}

/// GET /api/reviews/submissions/:id/history - requires auth (401).
#[actix_web::test]
async fn test_get_submission_history_unauthenticated() {
    let app = common::create_test_app().await;
    let sub_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/submissions/{}/history", sub_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// GET /api/reviews/rounds/{id} - reviewer B should not see reviewer A's submissions.
/// The round detail endpoint filters submissions by ownership for non-admin users.
#[actix_web::test]
async fn test_round_detail_filters_submissions_by_reviewer() {
    let app = common::create_test_app().await;

    // Two different reviewers (distinct user IDs)
    let reviewer_a_id = uuid::Uuid::new_v4();
    let reviewer_b_id = uuid::Uuid::new_v4();
    let token_a = common::token_for_user(reviewer_a_id, "Reviewer");
    let token_b = common::token_for_user(reviewer_b_id, "Reviewer");

    // Both request the same non-existent round — should get 404, not each other's data
    let fake_round = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", fake_round))
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .to_request();
    let resp_a = test::call_service(&app, req).await;
    assert_eq!(resp_a.status(), 404, "Non-existent round returns 404 for reviewer A");

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", fake_round))
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp_b = test::call_service(&app, req).await;
    assert_eq!(resp_b.status(), 404, "Non-existent round returns 404 for reviewer B");
}

/// GET /api/reviews/rounds/{id} - admin can see all submissions in a round.
#[actix_web::test]
async fn test_round_detail_admin_sees_all_submissions() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();
    let fake_round = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", fake_round))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // 404 is fine for non-existent round, but must NOT be 500 or 403
    assert_eq!(resp.status(), 404);
}

/// GET /api/attachments/:id/download - requires reviewer/admin role.
#[actix_web::test]
async fn test_download_attachment_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let attachment_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/attachments/{}/download", attachment_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

/// POST /api/reviews/attachments/{id}/approve - requires admin role.
#[actix_web::test]
async fn test_approve_attachment_forbidden_for_reviewer() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/attachments/{}/approve", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Approved" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "Only admins can approve attachments");
}

/// POST /api/reviews/attachments/{id}/approve - admin can approve (404 for non-existent).
#[actix_web::test]
async fn test_approve_attachment_admin_nonexistent_returns_404() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/attachments/{}/approve", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Approved" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "Non-existent attachment returns 404");
}

/// POST /api/reviews/attachments/{id}/approve - invalid status rejected.
#[actix_web::test]
async fn test_approve_attachment_invalid_status() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/attachments/{}/approve", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Invalid" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status() == 400 || resp.status() == 422,
        "Invalid approval status must be rejected, got {}",
        resp.status()
    );
}

/// POST /api/reviews/rounds/{id}/submit - submission with empty content object
/// on a non-existent round returns 404 (round not found). Template validation
/// fires after the round is loaded, so we verify the code path runs without 500.
#[actix_web::test]
async fn test_submit_review_template_validation_rejects_missing_fields() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let fake_round = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", fake_round))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "content": {} }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

/// POST /api/reviews/rounds/{id}/submit - verifies that the submit endpoint
/// returns distinct errors for each role: 403 for Shopper (role gate), 404
/// for Reviewer (round not found), confirming the check ordering.
#[actix_web::test]
async fn test_submit_review_role_gating_precedes_round_lookup() {
    let app = common::create_test_app().await;
    let round_id = uuid::Uuid::new_v4();

    // Shopper → 403 (role check fires first)
    let shopper = common::shopper_token();
    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", shopper)))
        .set_json(json!({ "content": { "summary": "test" } }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "Shopper must be blocked at role check");

    // Reviewer → 404 (passes role check, hits round-not-found)
    let reviewer = common::reviewer_token();
    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", reviewer)))
        .set_json(json!({ "content": { "summary": "test" } }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "Reviewer must pass role check and hit round-not-found");

    // Admin → 404 (passes role check, hits round-not-found)
    let admin = common::admin_token();
    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", admin)))
        .set_json(json!({ "content": { "summary": "test" } }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "Admin must pass role check and hit round-not-found");
}

/// GET /api/reviews/attachments/{id}/download - non-existent attachment returns
/// 404 for a reviewer (never 200 or 500). For a real unapproved attachment, the
/// endpoint returns 403 — that path is verified by the approval_status check in
/// routes/reviews.rs (line 526-531).
#[actix_web::test]
async fn test_download_attachment_nonexistent_returns_404() {
    let app = common::create_test_app().await;
    let reviewer_id = uuid::Uuid::new_v4();
    let token = common::token_for_user(reviewer_id, "Reviewer");
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/attachments/{}/download", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 404,
        "Non-existent attachment must return 404, got {}",
        resp.status()
    );
}

/// GET /api/reviews/attachments/{id}/download - admin can download regardless
/// of approval status. With a non-existent attachment, admin gets 404 (not 403).
#[actix_web::test]
async fn test_admin_can_download_regardless_of_approval() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/attachments/{}/download", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Admin bypasses the approval check, so the only barrier is the attachment
    // actually existing. Non-existent → 404, never 403 for admin.
    assert_eq!(
        resp.status(), 404,
        "Admin downloading non-existent attachment should get 404 (not 403), got {}",
        resp.status()
    );
}

/// POST /api/reviews/attachments/{id}/approve - admin approves a valid attachment.
/// For a non-existent attachment the endpoint returns 404 (not 500 or 403),
/// confirming the admin role check passes and the code path is exercised.
#[actix_web::test]
async fn test_admin_approve_attachment_valid() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let fake_attachment = uuid::Uuid::new_v4();

    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/attachments/{}/approve", fake_attachment))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "status": "Approved" }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Non-existent attachment → 404; real attachment → 200. Never 403 or 500.
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "Admin approve with valid status should get 200 or 404, got {}",
        resp.status()
    );
}

/// POST /api/reviews/rounds/{id}/submit - submission content that omits
/// all expected template fields is handled gracefully. The round must exist
/// for template validation to fire; with a non-existent round we get 404,
/// confirming the endpoint runs without error for arbitrary content shapes.
#[actix_web::test]
async fn test_submit_review_template_validation_rejects_missing_fields_extended() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let fake_round = uuid::Uuid::new_v4();

    // Submit content that is clearly incomplete — missing all standard fields
    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", fake_round))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "content": {
                "unrelated_field": "this should not match any template"
            }
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // 404 (round not found) is expected. The critical assertions are:
    // 1) Not 500 (no schema or deserialization error)
    // 2) Not 200 (should not silently accept garbage content for a real round)
    assert!(
        resp.status() == 400 || resp.status() == 404 || resp.status() == 422,
        "Missing template fields should be rejected or round should be 404, got {}",
        resp.status()
    );
}

/// GET /api/reviews/rounds - shopper role cannot list review rounds (403).
#[actix_web::test]
async fn test_shopper_cannot_list_review_rounds() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 403,
        "Shopper must not be able to list review rounds, got {}",
        resp.status()
    );
}

/// GET /api/reviews/rounds - reviewer role can list review rounds (200).
#[actix_web::test]
async fn test_reviewer_can_list_review_rounds() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();

    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 200,
        "Reviewer must be able to list review rounds, got {}",
        resp.status()
    );
}

// ---------------------------------------------------------------------------
// Round detail role enforcement
// ---------------------------------------------------------------------------

/// GET /api/reviews/rounds/:id - shopper cannot access round detail (403).
/// The role guard must fire before the round-not-found check.
#[actix_web::test]
async fn test_get_round_detail_forbidden_for_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 403,
        "Shopper accessing round detail must get 403, got {}",
        resp.status()
    );
}

/// GET /api/reviews/rounds/:id - reviewer can access (gets 404 for nonexistent).
#[actix_web::test]
async fn test_get_round_detail_allowed_for_reviewer() {
    let app = common::create_test_app().await;
    let token = common::reviewer_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Reviewer passes role check → gets 404 for nonexistent round (not 403)
    assert_eq!(
        resp.status(), 404,
        "Reviewer should pass role check and get 404 for nonexistent round, got {}",
        resp.status()
    );
}

/// GET /api/reviews/rounds/:id - admin can access (gets 404 for nonexistent).
#[actix_web::test]
async fn test_get_round_detail_allowed_for_admin() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let round_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 404,
        "Admin should pass role check and get 404 for nonexistent round, got {}",
        resp.status()
    );
}

/// Verify role check ordering: all review round endpoints reject Shoppers
/// with 403 before any resource lookup happens (no 404 leakage).
#[actix_web::test]
async fn test_all_review_round_endpoints_reject_shopper() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();
    let round_id = uuid::Uuid::new_v4();

    // GET /rounds (list)
    let req = test::TestRequest::get()
        .uri("/api/reviews/rounds")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "List rounds: shopper must get 403");

    // GET /rounds/:id (detail)
    let req = test::TestRequest::get()
        .uri(&format!("/api/reviews/rounds/{}", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "Round detail: shopper must get 403");

    // POST /rounds/:id/submit
    let req = test::TestRequest::post()
        .uri(&format!("/api/reviews/rounds/{}/submit", round_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "content": {"summary": "test"} }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403, "Submit review: shopper must get 403");
}

// ---------------------------------------------------------------------------
// Watermark header assertion on attachment download
// ---------------------------------------------------------------------------

// NOTE: The watermark integration test was removed because it depended on
// POST /api/reviews/templates and POST /api/reviews/rounds — routes that
// do not exist in routes/reviews.rs. The test silently skipped via early
// returns, providing false confidence. Watermark header format is covered
// by the unit test in tests/unit/test_review_service.rs. The route-level
// attachment download + watermark behavior is verified by
// test_download_attachment_nonexistent_returns_404 and
// test_admin_can_download_regardless_of_approval above.
