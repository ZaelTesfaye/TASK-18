use silverscreen_backend::services::auth_service::validate_password_policy;

#[test]
fn test_valid_passwords() {
    assert!(validate_password_policy("MyP@ssw0rd").is_ok());
    assert!(validate_password_policy("1aA!xxxx").is_ok());
    assert!(validate_password_policy("Complex1!Pass").is_ok());
    assert!(validate_password_policy("Str0ng#Password").is_ok());
}

#[test]
fn test_minimum_length_exactly_8() {
    // Exactly 8 chars with all requirements met.
    assert!(validate_password_policy("Aa1!xxxx").is_ok());
}

#[test]
fn test_too_short() {
    assert!(validate_password_policy("Aa1!xxx").is_err()); // 7 chars
    assert!(validate_password_policy("Aa1!").is_err());     // 4 chars
    assert!(validate_password_policy("").is_err());         // 0 chars
}

#[test]
fn test_missing_uppercase() {
    assert!(validate_password_policy("nouppercase1!").is_err());
}

#[test]
fn test_missing_lowercase() {
    assert!(validate_password_policy("NOLOWERCASE1!").is_err());
}

#[test]
fn test_missing_digit() {
    assert!(validate_password_policy("NoDigits!!aB").is_err());
}

#[test]
fn test_missing_special() {
    assert!(validate_password_policy("NoSpecial123aA").is_err());
}

#[test]
fn test_all_categories_boundary() {
    // Just barely meets every requirement.
    assert!(validate_password_policy("aA1!xxxx").is_ok());
}

#[test]
fn test_unicode_special_chars() {
    // Non-alphanumeric unicode characters count as special.
    assert!(validate_password_policy("Passw0rd§").is_ok());
}
