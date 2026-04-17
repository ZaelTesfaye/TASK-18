use actix_web::test;
use serde_json::json;

use super::common;

// ---------------------------------------------------------------------------
// GET /api/users/:id — register a user, fetch them by ID as admin
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_get_user_by_id_as_admin() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Register a user
    let user = common::register_and_login(&app, "getuserbyid").await.expect("DB required");

    // Fetch the user by ID using admin token
    let req = test::TestRequest::get()
        .uri(&format!("/api/users/{}", user.user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), user.user_id);
    assert!(body["username"].is_string());
    assert!(body["email"].is_string());
    assert!(body["role"].is_string());
}

// ---------------------------------------------------------------------------
// PUT /api/users/me — login, update profile fields
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_update_user_profile() {
    let app = common::create_test_app().await;

    let user = common::register_and_login(&app, "updateme").await.expect("DB required");

    // Update phone and address
    let req = test::TestRequest::put()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", user.access_token)))
        .set_json(json!({
            "phone": "555-123-4567",
            "address": "123 Main St, Springfield, IL 62701"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), user.user_id);
    // Phone and address should be masked in the response
    assert!(body["phone_masked"].is_string(), "phone_masked must be present after update");
    assert!(body["address_masked"].is_string(), "address_masked must be present after update");
}

// ---------------------------------------------------------------------------
// POST /api/users/me/unmask — login, call unmask, assert sensitive fields
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_unmask_user_pii() {
    let app = common::create_test_app().await;

    let user = common::register_and_login(&app, "unmaskme").await.expect("DB required");

    // First set phone and address
    let req = test::TestRequest::put()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", user.access_token)))
        .set_json(json!({
            "phone": "555-987-6543",
            "address": "456 Oak Ave, Portland, OR 97201"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Unmask PII
    let req = test::TestRequest::post()
        .uri("/api/users/me/unmask")
        .insert_header(("Authorization", format!("Bearer {}", user.access_token)))
        .set_json(json!({ "justification": "Testing unmask endpoint" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), user.user_id);
    assert!(body["username"].is_string());
    assert!(body["email"].is_string());
    // Unmasked fields should contain the actual values (not masked)
    let phone = body["phone"].as_str().unwrap_or("");
    assert!(phone.contains("555"), "Unmasked phone should contain the actual number");
    let address = body["address"].as_str().unwrap_or("");
    assert!(address.contains("Portland"), "Unmasked address should contain actual city");
}
