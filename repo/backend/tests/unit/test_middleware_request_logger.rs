use silverscreen_backend::middleware::redact_sensitive;

// ---------------------------------------------------------------------------
// redact_sensitive: values longer than 4 chars show first 2 + "****"
// ---------------------------------------------------------------------------

#[test]
fn test_redact_long_value() {
    let result = redact_sensitive("mysecretpassword");
    assert_eq!(result, "my****", "Should show first 2 chars then ****");
}

#[test]
fn test_redact_exactly_five_chars() {
    let result = redact_sensitive("hello");
    assert_eq!(result, "he****", "5-char value should show first 2 chars + ****");
}

#[test]
fn test_redact_six_chars() {
    let result = redact_sensitive("secret");
    assert_eq!(result, "se****");
}

// ---------------------------------------------------------------------------
// redact_sensitive: values of 4 chars or fewer are fully masked
// ---------------------------------------------------------------------------

#[test]
fn test_redact_four_chars() {
    let result = redact_sensitive("abcd");
    assert_eq!(result, "****", "4-char value should be fully masked");
}

#[test]
fn test_redact_three_chars() {
    let result = redact_sensitive("abc");
    assert_eq!(result, "****", "3-char value should be fully masked");
}

#[test]
fn test_redact_two_chars() {
    let result = redact_sensitive("ab");
    assert_eq!(result, "****", "2-char value should be fully masked");
}

#[test]
fn test_redact_one_char() {
    let result = redact_sensitive("x");
    assert_eq!(result, "****", "1-char value should be fully masked");
}

#[test]
fn test_redact_empty_string() {
    let result = redact_sensitive("");
    assert_eq!(result, "****", "Empty string should be fully masked");
}

// ---------------------------------------------------------------------------
// Format consistency
// ---------------------------------------------------------------------------

#[test]
fn test_redact_always_ends_with_stars() {
    let inputs = ["", "a", "ab", "abc", "abcd", "abcde", "abcdefgh"];
    for input in &inputs {
        let result = redact_sensitive(input);
        assert!(
            result.ends_with("****"),
            "Redacted value for '{}' should end with '****', got '{}'",
            input,
            result
        );
    }
}

#[test]
fn test_redact_preserves_first_two_chars_for_long() {
    let result = redact_sensitive("Password123!");
    assert!(result.starts_with("Pa"), "Should preserve 'Pa' from 'Password123!'");
    assert_eq!(result, "Pa****");
}

#[test]
fn test_redact_email_like_value() {
    let result = redact_sensitive("user@example.com");
    assert_eq!(result, "us****", "Email should show first 2 chars + ****");
}

#[test]
fn test_redact_numeric_string() {
    let result = redact_sensitive("1234567890");
    assert_eq!(result, "12****", "Numeric string should show first 2 digits + ****");
}
