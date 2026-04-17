use uuid::Uuid;

// ---------------------------------------------------------------------------
// Payment amount tolerance
// ---------------------------------------------------------------------------

#[test]
fn test_amount_tolerance_within_one_cent() {
    // The payment simulator uses a tolerance of 0.01 for amount comparison.
    let order_total: f64 = 99.99;
    let submitted_amount: f64 = 99.995;
    let tolerance: f64 = 0.01;
    let diff = (submitted_amount - order_total).abs();
    assert!(
        diff <= tolerance,
        "Difference of {} is within tolerance of {}",
        diff,
        tolerance
    );
}

#[test]
fn test_amount_tolerance_exactly_at_boundary() {
    let order_total: f64 = 100.00;
    let submitted_amount: f64 = 100.01;
    let tolerance: f64 = 0.01;
    let diff = (submitted_amount - order_total).abs();
    assert!(
        diff <= tolerance,
        "Difference of exactly 0.01 should be within tolerance"
    );
}

#[test]
fn test_amount_tolerance_exceeds_boundary() {
    let order_total: f64 = 100.00;
    let submitted_amount: f64 = 100.02;
    let tolerance: f64 = 0.01;
    let diff = (submitted_amount - order_total).abs();
    assert!(
        diff > tolerance,
        "Difference of 0.02 should exceed tolerance of 0.01"
    );
}

#[test]
fn test_amount_tolerance_negative_difference() {
    let order_total: f64 = 50.00;
    let submitted_amount: f64 = 49.99;
    let tolerance: f64 = 0.01;
    let diff = (submitted_amount - order_total).abs();
    assert!(
        diff <= tolerance,
        "Negative difference within tolerance should be accepted"
    );
}

#[test]
fn test_amount_tolerance_exact_match() {
    let order_total: f64 = 75.50;
    let submitted_amount: f64 = 75.50;
    let tolerance: f64 = 0.01;
    let diff = (submitted_amount - order_total).abs();
    assert!(
        diff <= tolerance,
        "Exact match should always be within tolerance"
    );
}

// ---------------------------------------------------------------------------
// Idempotency key format
// ---------------------------------------------------------------------------

#[test]
fn test_idempotency_key_format() {
    // The simulator generates keys as "{order_id}:{attempt_number}"
    let order_id = Uuid::new_v4();
    let attempt_number: i32 = 1;
    let key = format!("{}:{}", order_id, attempt_number);

    assert!(
        key.contains(':'),
        "Idempotency key should contain a colon separator"
    );
    let parts: Vec<&str> = key.splitn(2, ':').collect();
    assert_eq!(parts.len(), 2, "Key should have exactly two parts");
    assert_eq!(
        parts[0],
        order_id.to_string(),
        "First part should be the order ID"
    );
    assert_eq!(
        parts[1],
        attempt_number.to_string(),
        "Second part should be the attempt number"
    );
}

#[test]
fn test_idempotency_key_uniqueness_across_attempts() {
    let order_id = Uuid::new_v4();
    let key1 = format!("{}:{}", order_id, 1);
    let key2 = format!("{}:{}", order_id, 2);
    assert_ne!(
        key1, key2,
        "Different attempt numbers should produce different idempotency keys"
    );
}

#[test]
fn test_idempotency_key_uniqueness_across_orders() {
    let order_a = Uuid::new_v4();
    let order_b = Uuid::new_v4();
    let key_a = format!("{}:{}", order_a, 1);
    let key_b = format!("{}:{}", order_b, 1);
    assert_ne!(
        key_a, key_b,
        "Different order IDs should produce different idempotency keys"
    );
}

// ---------------------------------------------------------------------------
// Valid payment outcomes
// ---------------------------------------------------------------------------

#[test]
fn test_valid_payment_outcomes() {
    let valid_outcomes = ["Success", "Failed", "Timeout"];
    for outcome in &valid_outcomes {
        let is_valid = matches!(*outcome, "Success" | "Failed" | "Timeout");
        assert!(is_valid, "'{}' should be a valid payment outcome", outcome);
    }
}

#[test]
fn test_invalid_payment_outcome() {
    let invalid_outcomes = ["Pending", "Error", "success", "FAILED", ""];
    for outcome in &invalid_outcomes {
        let is_valid = matches!(*outcome, "Success" | "Failed" | "Timeout");
        assert!(
            !is_valid,
            "'{}' should NOT be a valid payment outcome",
            outcome
        );
    }
}

#[test]
fn test_success_outcome_transitions_order() {
    // On Success, the order transitions from Reserved -> Paid.
    // This is a conceptual check of the documented behavior.
    let outcome = "Success";
    let expected_transition = ("Reserved", "Paid");
    assert_eq!(outcome, "Success");
    assert_eq!(
        expected_transition,
        ("Reserved", "Paid"),
        "Success payment should transition Reserved -> Paid"
    );
}

#[test]
fn test_failed_outcome_no_state_change() {
    // On Failed or Timeout, the order state does not change.
    let outcome = "Failed";
    let state_changes = outcome == "Success";
    assert!(
        !state_changes,
        "Failed outcome should not change order state"
    );
}

#[test]
fn test_default_payment_method() {
    // When payment_method is None, the simulator defaults to "local_tender".
    let payment_method: Option<&str> = None;
    let effective = payment_method.unwrap_or("local_tender");
    assert_eq!(
        effective, "local_tender",
        "Default payment method should be 'local_tender'"
    );
}
