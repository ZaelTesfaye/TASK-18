use silverscreen_backend::services::auth_service;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Password hashing
// ---------------------------------------------------------------------------

#[test]
fn test_hash_and_verify_password() {
    let password = "SecureP@ss123";
    let hash = auth_service::hash_password(password).unwrap();

    // Hash should be a valid Argon2 hash string.
    assert!(hash.starts_with("$argon2"));

    // Verification should succeed for the correct password.
    assert!(auth_service::verify_password(password, &hash).unwrap());
}

#[test]
fn test_verify_wrong_password() {
    let password = "SecureP@ss123";
    let hash = auth_service::hash_password(password).unwrap();

    // Wrong password should fail verification.
    assert!(!auth_service::verify_password("WrongPass1!", &hash).unwrap());
}

#[test]
fn test_hash_different_salts() {
    let password = "SamePassword1!";
    let hash1 = auth_service::hash_password(password).unwrap();
    let hash2 = auth_service::hash_password(password).unwrap();

    // Two hashes of the same password should differ (different salts).
    assert_ne!(hash1, hash2);

    // Both should verify correctly.
    assert!(auth_service::verify_password(password, &hash1).unwrap());
    assert!(auth_service::verify_password(password, &hash2).unwrap());
}

// ---------------------------------------------------------------------------
// Password policy
// ---------------------------------------------------------------------------

#[test]
fn test_password_policy_valid() {
    assert!(auth_service::validate_password_policy("Str0ng!Pass").is_ok());
    assert!(auth_service::validate_password_policy("MyP@ssw0rd").is_ok());
    assert!(auth_service::validate_password_policy("1aA!xxxx").is_ok());
}

#[test]
fn test_password_policy_too_short() {
    let result = auth_service::validate_password_policy("Ab1!");
    assert!(result.is_err());
}

#[test]
fn test_password_policy_no_uppercase() {
    let result = auth_service::validate_password_policy("lowercase1!");
    assert!(result.is_err());
}

#[test]
fn test_password_policy_no_lowercase() {
    let result = auth_service::validate_password_policy("UPPERCASE1!");
    assert!(result.is_err());
}

#[test]
fn test_password_policy_no_digit() {
    let result = auth_service::validate_password_policy("NoDigits!!");
    assert!(result.is_err());
}

#[test]
fn test_password_policy_no_special() {
    let result = auth_service::validate_password_policy("NoSpecial1x");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// JWT generation and validation
// ---------------------------------------------------------------------------

#[test]
fn test_generate_and_validate_access_token() {
    let user_id = Uuid::new_v4();
    let secret = "test_jwt_secret_that_is_long_enough";
    let role = "Shopper";

    let token = auth_service::generate_access_token(user_id, role, secret, 30).unwrap();
    assert!(!token.is_empty());

    let claims = auth_service::validate_token(&token, secret).unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.role, "Shopper");
    assert!(!claims.jti.is_empty());
}

#[test]
fn test_generate_and_validate_refresh_token() {
    let user_id = Uuid::new_v4();
    let secret = "test_jwt_secret_that_is_long_enough";

    let token = auth_service::generate_refresh_token(user_id, secret, 7).unwrap();
    assert!(!token.is_empty());

    let claims = auth_service::validate_token(&token, secret).unwrap();
    assert_eq!(claims.sub, user_id);
}

#[test]
fn test_validate_token_wrong_secret() {
    let user_id = Uuid::new_v4();
    let secret = "correct_secret_for_signing_tokens";
    let wrong = "wrong_secret_for_validation_test";

    let token = auth_service::generate_access_token(user_id, "Admin", secret, 30).unwrap();
    let result = auth_service::validate_token(&token, wrong);
    assert!(result.is_err());
}

#[test]
fn test_token_contains_unique_jti() {
    let user_id = Uuid::new_v4();
    let secret = "test_jwt_secret_that_is_long_enough";

    let token1 = auth_service::generate_access_token(user_id, "Shopper", secret, 30).unwrap();
    let token2 = auth_service::generate_access_token(user_id, "Shopper", secret, 30).unwrap();

    let claims1 = auth_service::validate_token(&token1, secret).unwrap();
    let claims2 = auth_service::validate_token(&token2, secret).unwrap();

    assert_ne!(claims1.jti, claims2.jti);
}

#[test]
fn test_token_expiry_set_correctly() {
    let user_id = Uuid::new_v4();
    let secret = "test_jwt_secret_that_is_long_enough";

    let token = auth_service::generate_access_token(user_id, "Shopper", secret, 30).unwrap();
    let claims = auth_service::validate_token(&token, secret).unwrap();

    // Expiry should be roughly 30 minutes from now.
    let now = chrono::Utc::now().timestamp();
    let diff = claims.exp - now;
    assert!(diff > 29 * 60 && diff <= 30 * 60);
}

#[test]
fn test_refresh_token_expiry() {
    let user_id = Uuid::new_v4();
    let secret = "test_jwt_secret_that_is_long_enough";

    let token = auth_service::generate_refresh_token(user_id, secret, 7).unwrap();
    let claims = auth_service::validate_token(&token, secret).unwrap();

    let now = chrono::Utc::now().timestamp();
    let diff = claims.exp - now;
    // Should be roughly 7 days (604800 seconds).
    assert!(diff > 6 * 86400 && diff <= 7 * 86400);
}
