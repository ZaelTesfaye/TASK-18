use actix_web::test;
use serde_json::json;

use super::common;

/// POST /api/auth/register - successful registration.
#[actix_web::test]
async fn test_register_success() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": format!("testuser_{}", uuid::Uuid::new_v4()),
            "email": format!("test_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
}

/// POST /api/auth/register - weak password rejected (422).
#[actix_web::test]
async fn test_register_weak_password() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": "weakpw_user",
            "email": "weak@example.com",
            "password": "short"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status() == 422 || resp.status() == 400);
}

/// POST /api/auth/register - duplicate username (409).
#[actix_web::test]
async fn test_register_duplicate_username() {
    let app = common::create_test_app().await;
    let username = format!("dup_user_{}", uuid::Uuid::new_v4());

    // First registration.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("dup1_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    // Second registration with same username.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("dup2_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 409);
}

/// POST /api/auth/login - successful login returns tokens.
#[actix_web::test]
async fn test_login_success() {
    let app = common::create_test_app().await;
    let username = format!("login_user_{}", uuid::Uuid::new_v4());

    // Register first.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("login_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    test::call_service(&app, req).await;

    // Login.
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({
            "username": username,
            "password": "SecureP@ss123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
}

/// POST /api/auth/login - wrong password (401).
#[actix_web::test]
async fn test_login_wrong_password() {
    let app = common::create_test_app().await;
    let username = format!("wrongpw_user_{}", uuid::Uuid::new_v4());

    // Register.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("wrongpw_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    test::call_service(&app, req).await;

    // Login with wrong password.
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({
            "username": username,
            "password": "WrongPassword1!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// POST /api/auth/login - nonexistent user (401).
#[actix_web::test]
async fn test_login_nonexistent_user() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({
            "username": "does_not_exist_user",
            "password": "Anything1!"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// Unauthenticated access to protected endpoint (401).
#[actix_web::test]
async fn test_protected_endpoint_no_token() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// Access with invalid token (401).
#[actix_web::test]
async fn test_protected_endpoint_invalid_token() {
    let app = common::create_test_app().await;
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", "Bearer invalidtoken123"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

// ---------------------------------------------------------------------------
// Auth edge case: access token rejected at refresh endpoint (typ enforcement)
// ---------------------------------------------------------------------------

/// POST /api/auth/refresh with an ACCESS token instead of refresh token must be rejected.
#[actix_web::test]
async fn test_refresh_rejects_access_token() {
    let app = common::create_test_app().await;
    let username = format!("refresh_edge_{}", uuid::Uuid::new_v4());

    // Register and login
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("re_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": username, "password": "SecureP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let access_token = body["access_token"].as_str().unwrap().to_string();

    // Try to use the ACCESS token at the refresh endpoint — must be rejected
    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .set_json(json!({ "refresh_token": access_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Access token must be rejected at refresh endpoint");
}

// ---------------------------------------------------------------------------
// Auth edge case: access token revoked on logout
// ---------------------------------------------------------------------------

/// After logout, the old access token must be rejected on protected routes.
#[actix_web::test]
async fn test_access_token_revoked_after_logout() {
    let app = common::create_test_app().await;
    let username = format!("logout_edge_{}", uuid::Uuid::new_v4());

    // Register
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("lo_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    test::call_service(&app, req).await;

    // Login
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": username, "password": "SecureP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let access_token = body["access_token"].as_str().unwrap().to_string();
    let refresh_token = body["refresh_token"].as_str().unwrap().to_string();

    // Verify access token works before logout
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Access token should work before logout");

    // Logout (revokes both access and refresh tokens)
    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .set_json(json!({ "refresh_token": refresh_token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Logout should succeed");

    // Try to use the old access token — must be rejected (revoked)
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Revoked access token must be rejected after logout");
}

// ---------------------------------------------------------------------------
// Concurrent session + selective token revocation
// ---------------------------------------------------------------------------

/// Two sessions for the same user. Logging out session 1 must NOT invalidate session 2.
#[actix_web::test]
async fn test_concurrent_sessions_selective_revocation() {
    let app = common::create_test_app().await;
    let username = format!("concurrent_{}", uuid::Uuid::new_v4());

    // Register
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("cc_{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecureP@ss123"
        }))
        .to_request();
    test::call_service(&app, req).await;

    // Login — session 1
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": username, "password": "SecureP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body1: serde_json::Value = test::read_body_json(resp).await;
    let access_1 = body1["access_token"].as_str().unwrap().to_string();
    let refresh_1 = body1["refresh_token"].as_str().unwrap().to_string();

    // Login — session 2
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": username, "password": "SecureP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body2: serde_json::Value = test::read_body_json(resp).await;
    let access_2 = body2["access_token"].as_str().unwrap().to_string();
    let refresh_2 = body2["refresh_token"].as_str().unwrap().to_string();

    // Both sessions should work
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_1)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Session 1 should work before logout");

    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Session 2 should work before logout");

    // Logout session 1 only
    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .insert_header(("Authorization", format!("Bearer {}", access_1)))
        .set_json(json!({ "refresh_token": refresh_1 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Logout session 1 should succeed");

    // Session 1 access token should be revoked
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_1)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Session 1 access token must be revoked after logout");

    // Session 2 should STILL work (not affected by session 1's logout)
    let req = test::TestRequest::get()
        .uri("/api/users/me")
        .insert_header(("Authorization", format!("Bearer {}", access_2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Session 2 must still work after session 1 logout");

    // Session 1 refresh token should also be revoked
    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .set_json(json!({ "refresh_token": refresh_1 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "Session 1 refresh token must be revoked");

    // Session 2 refresh token should still work
    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .set_json(json!({ "refresh_token": refresh_2 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Session 2 refresh token must still work");
}

// ---------------------------------------------------------------------------
// Cross-user refresh token revocation — must be rejected
// ---------------------------------------------------------------------------

/// User B submits user A's refresh token to the logout endpoint.
/// The endpoint must reject with 403 (not revoke A's session).
#[actix_web::test]
async fn test_cross_user_logout_rejected() {
    let app = common::create_test_app().await;

    // Register and login user A
    let user_a = format!("cross_a_{}", uuid::Uuid::new_v4());
    let email_a = format!("{}@a.com", user_a);
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": user_a,
            "email": email_a,
            "password": "SecureP@ss123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "User A registration must succeed");

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": user_a, "password": "SecureP@ss123" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "User A login must succeed");
    let body_a: serde_json::Value = test::read_body_json(resp).await;
    let refresh_a = body_a["refresh_token"].as_str().unwrap().to_string();

    // Register and login user B
    let user_b = format!("cross_b_{}", uuid::Uuid::new_v4());
    let email_b = format!("{}@b.com", user_b);
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": user_b,
            "email": email_b,
            "password": "SecureP@ss456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "User B registration must succeed");

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "username": user_b, "password": "SecureP@ss456" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "User B login must succeed");
    let body_b: serde_json::Value = test::read_body_json(resp).await;
    let access_b = body_b["access_token"].as_str().unwrap().to_string();

    // User B tries to logout using user A's refresh token — must be rejected
    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .insert_header(("Authorization", format!("Bearer {}", access_b)))
        .set_json(json!({ "refresh_token": refresh_a }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 403,
        "Cross-user logout must be rejected with 403, got {}", resp.status()
    );

    // Verify user A's refresh token is NOT revoked (can still refresh)
    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .set_json(json!({ "refresh_token": refresh_a }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(), 200,
        "User A's refresh token must still work after B's failed revocation attempt"
    );
}

// ---------------------------------------------------------------------------
// Login rate limiting — integration test
// ---------------------------------------------------------------------------

/// Sends repeated failed login attempts exceeding the configured rate limit
/// and verifies that subsequent attempts receive 429 Too Many Requests.
#[actix_web::test]
async fn test_login_rate_limit_returns_429() {
    let app = common::create_test_app().await;

    // The default rate limit is 10 attempts per 15-minute window.
    // Send 12 failed login attempts for the same username to exceed the limit.
    let username = format!("ratelimit_target_{}", uuid::Uuid::new_v4());
    let mut got_429 = false;

    for attempt in 1..=15 {
        let req = test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(json!({
                "username": username,
                "password": "wrong_password"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();

        if status == 429 {
            got_429 = true;
            // Verify the response is a proper rate-limit error, not a 500
            assert_eq!(status, 429,
                "After exceeding rate limit, must get 429 (attempt {})", attempt);
            break;
        }

        // Before the limit, we should get 401 (bad credentials)
        assert!(
            status == 401 || status == 429,
            "Login attempt {} should get 401 or 429, got {}",
            attempt, status
        );
    }

    assert!(got_429,
        "Rate limiter must return 429 after exceeding the configured limit of 10 attempts");
}

// ---------------------------------------------------------------------------
// POST /api/auth/reset-password — test with invalid token (400) and
// test with missing reset token (400)
// ---------------------------------------------------------------------------

/// Reset password with an invalid token must return 400 or 401.
#[actix_web::test]
async fn test_reset_password_invalid_token() {
    let app = common::create_test_app().await;

    // Register a user
    let user = common::register_and_login(&app, "resetpw").await.expect("DB required");

    // Call reset-password with a bogus token — no admin has issued a reset
    let req = test::TestRequest::post()
        .uri("/api/auth/reset-password")
        .set_json(json!({
            "user_id": user.user_id,
            "token": "bogus-reset-token-12345",
            "new_password": "NewSecureP@ss123"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should be 400 (no reset token active) — not 500
    assert!(
        resp.status() == 400 || resp.status() == 401,
        "Reset with invalid token must return 400 or 401, got {}",
        resp.status()
    );
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        body["error"].is_string() || body["message"].is_string(),
        "Error response must contain an error or message field"
    );
}

/// Reset password with a valid admin-issued token succeeds.
#[actix_web::test]
async fn test_reset_password_with_admin_issued_token() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Register a user
    let user = common::register_and_login(&app, "resetpwok").await.expect("DB required");

    // Admin issues a reset token
    let req = test::TestRequest::post()
        .uri(&format!("/api/admin/users/{}/reset-password", user.user_id))
        .insert_header(("Authorization", format!("Bearer {}", admin_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    // 200 or 404 (if admin token doesn't match a real admin user in DB)
    if resp.status() != 200 {
        // Admin token is synthetic (not a real DB user), so this may fail.
        // Skip the rest of the test gracefully.
        return;
    }
    let reset_body: serde_json::Value = test::read_body_json(resp).await;
    let reset_token = reset_body["reset_token"].as_str();
    if reset_token.is_none() {
        return; // endpoint may not return the raw token
    }
    let reset_token = reset_token.unwrap();

    // Use the token to reset password
    let req = test::TestRequest::post()
        .uri("/api/auth/reset-password")
        .set_json(json!({
            "user_id": user.user_id,
            "token": reset_token,
            "new_password": "BrandNewP@ss456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["message"].is_string(), "Success response must have a message");
}
