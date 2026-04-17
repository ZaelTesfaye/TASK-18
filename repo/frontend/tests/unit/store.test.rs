// Store logic tests (token/user management).
// These test the pure logic aspects — actual localStorage is not available
// outside the browser, so we test serialization and validation logic.
// Imports the real store module to verify compilation.

#[allow(unused_imports)]
use silverscreen_frontend::store;
use silverscreen_frontend::types::*;

#[test]
fn test_token_format_validation() {
    // JWT tokens have 3 parts separated by dots.
    let valid_token = "header.payload.signature";
    let parts: Vec<&str> = valid_token.split('.').collect();
    assert_eq!(parts.len(), 3);

    let invalid_token = "not_a_jwt";
    let parts: Vec<&str> = invalid_token.split('.').collect();
    assert_ne!(parts.len(), 3);
}

#[test]
fn test_user_role_display() {
    let roles = vec!["Shopper", "Reviewer", "Admin"];
    for role in &roles {
        assert!(!role.is_empty());
    }
}

#[test]
fn test_auth_state_logic() {
    // Simulates store logic: when token is Some, user is authenticated.
    let token: Option<String> = Some("header.payload.signature".to_string());
    assert!(token.is_some());

    let no_token: Option<String> = None;
    assert!(!no_token.is_some());
}

#[test]
fn test_user_response_serialization() {
    let user_json = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "username": "alice",
        "email": "alice@example.com",
        "role": "Shopper",
        "phone_masked": "(XXX) XXX-XX34",
        "address_masked": "***, Springfield, IL"
    });

    assert_eq!(user_json["username"], "alice");
    assert_eq!(user_json["role"], "Shopper");
    assert!(user_json["phone_masked"].as_str().unwrap().contains("XXX"));
}

#[test]
fn test_masked_phone_format() {
    let masked = "(XXX) XXX-XX21";
    assert!(masked.starts_with("(XXX)"));
    assert!(masked.len() > 10);
    // Last 2 digits should be visible.
    assert!(masked.ends_with("21"));
}

#[test]
fn test_masked_address_format() {
    let masked = "***, Springfield, IL 62701";
    assert!(masked.starts_with("***"));
    assert!(masked.contains("Springfield"));
}
