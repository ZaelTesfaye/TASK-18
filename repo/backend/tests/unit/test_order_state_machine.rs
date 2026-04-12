use chrono;
use silverscreen_backend::services::order_state_machine::{
    OrderStateMachine, OrderStatus, TransitionContext,
};

// ---------------------------------------------------------------------------
// Legal transitions
// ---------------------------------------------------------------------------

#[test]
fn test_created_to_reserved() {
    let result = OrderStateMachine::transition(OrderStatus::Created, OrderStatus::Reserved);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Reserved);
}

#[test]
fn test_reserved_to_paid() {
    let result = OrderStateMachine::transition(OrderStatus::Reserved, OrderStatus::Paid);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Paid);
}

#[test]
fn test_reserved_to_cancelled() {
    let result = OrderStateMachine::transition(OrderStatus::Reserved, OrderStatus::Cancelled);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Cancelled);
}

#[test]
fn test_paid_to_processing() {
    let result = OrderStateMachine::transition(OrderStatus::Paid, OrderStatus::Processing);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Processing);
}

#[test]
fn test_paid_to_refunded_requires_context() {
    // Paid -> Refunded now requires context (reason code + delivery date)
    let result = OrderStateMachine::transition(OrderStatus::Paid, OrderStatus::Refunded);
    assert!(result.is_err(), "Paid->Refunded without context must fail");
}

#[test]
fn test_paid_to_refunded_with_context() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Paid,
        OrderStatus::Refunded,
        Some(&ctx),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Refunded);
}

#[test]
fn test_processing_to_shipped() {
    let result = OrderStateMachine::transition(OrderStatus::Processing, OrderStatus::Shipped);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Shipped);
}

#[test]
fn test_shipped_to_delivered() {
    let result = OrderStateMachine::transition(OrderStatus::Shipped, OrderStatus::Delivered);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Delivered);
}

#[test]
fn test_delivered_to_completed() {
    let result = OrderStateMachine::transition(OrderStatus::Delivered, OrderStatus::Completed);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Completed);
}

#[test]
fn test_delivered_to_return_requested_with_context() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ReturnRequested,
        Some(&ctx),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::ReturnRequested);
}

#[test]
fn test_delivered_to_exchange_requested_with_context() {
    let ctx = TransitionContext {
        reason_code: Some("WrongItem"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ExchangeRequested,
        Some(&ctx),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::ExchangeRequested);
}

#[test]
fn test_return_requested_without_context_fails() {
    // Calling transition() (no context) for ReturnRequested must fail
    let result =
        OrderStateMachine::transition(OrderStatus::Delivered, OrderStatus::ReturnRequested);
    assert!(result.is_err(), "ReturnRequested without context must fail");
}

#[test]
fn test_exchange_requested_without_context_fails() {
    let result =
        OrderStateMachine::transition(OrderStatus::Delivered, OrderStatus::ExchangeRequested);
    assert!(result.is_err(), "ExchangeRequested without context must fail");
}

#[test]
fn test_return_rejected_outside_30_day_window() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(45)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ReturnRequested,
        Some(&ctx),
    );
    assert!(result.is_err(), "Return outside 30-day window must fail");
}

#[test]
fn test_return_rejected_without_reason_code() {
    let ctx = TransitionContext {
        reason_code: None,
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ReturnRequested,
        Some(&ctx),
    );
    assert!(result.is_err(), "Return without reason code must fail");
}

#[test]
fn test_return_rejected_with_invalid_reason_code() {
    let ctx = TransitionContext {
        reason_code: Some("InvalidReason"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ReturnRequested,
        Some(&ctx),
    );
    assert!(result.is_err(), "Return with invalid reason code must fail");
}

#[test]
fn test_return_accepted_with_each_valid_reason_code() {
    let codes = ["Defective", "WrongItem", "NotAsDescribed", "ChangedMind", "Other"];
    for code in &codes {
        let ctx = TransitionContext {
            reason_code: Some(code),
            delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
        };
        let result = OrderStateMachine::transition_with_context(
            OrderStatus::Delivered,
            OrderStatus::ReturnRequested,
            Some(&ctx),
        );
        assert!(result.is_ok(), "Reason code '{}' should be accepted", code);
    }
}

#[test]
fn test_return_requested_to_returned() {
    let result =
        OrderStateMachine::transition(OrderStatus::ReturnRequested, OrderStatus::Returned);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Returned);
}

#[test]
fn test_returned_to_refunded_requires_context() {
    // Returned -> Refunded also requires context now
    let result = OrderStateMachine::transition(OrderStatus::Returned, OrderStatus::Refunded);
    assert!(result.is_err(), "Returned->Refunded without context must fail");
}

#[test]
fn test_returned_to_refunded_with_context() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Returned,
        OrderStatus::Refunded,
        Some(&ctx),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Refunded);
}

#[test]
fn test_exchange_requested_to_exchanged() {
    let result =
        OrderStateMachine::transition(OrderStatus::ExchangeRequested, OrderStatus::Exchanged);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Exchanged);
}

// ---------------------------------------------------------------------------
// Admin override: Any -> Cancelled
// ---------------------------------------------------------------------------

#[test]
fn test_admin_cancel_from_paid() {
    let result = OrderStateMachine::transition(OrderStatus::Paid, OrderStatus::Cancelled);
    assert!(result.is_ok());
}

#[test]
fn test_admin_cancel_from_processing() {
    let result = OrderStateMachine::transition(OrderStatus::Processing, OrderStatus::Cancelled);
    assert!(result.is_ok());
}

#[test]
fn test_admin_cancel_from_shipped() {
    let result = OrderStateMachine::transition(OrderStatus::Shipped, OrderStatus::Cancelled);
    assert!(result.is_ok());
}

#[test]
fn test_admin_cancel_from_delivered() {
    let result = OrderStateMachine::transition(OrderStatus::Delivered, OrderStatus::Cancelled);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Illegal transitions
// ---------------------------------------------------------------------------

#[test]
fn test_illegal_created_to_paid() {
    let result = OrderStateMachine::transition(OrderStatus::Created, OrderStatus::Paid);
    assert!(result.is_err());
}

#[test]
fn test_illegal_created_to_shipped() {
    let result = OrderStateMachine::transition(OrderStatus::Created, OrderStatus::Shipped);
    assert!(result.is_err());
}

#[test]
fn test_illegal_reserved_to_shipped() {
    let result = OrderStateMachine::transition(OrderStatus::Reserved, OrderStatus::Shipped);
    assert!(result.is_err());
}

#[test]
fn test_illegal_completed_to_paid() {
    let result = OrderStateMachine::transition(OrderStatus::Completed, OrderStatus::Paid);
    assert!(result.is_err());
}

#[test]
fn test_illegal_refunded_to_paid() {
    let result = OrderStateMachine::transition(OrderStatus::Refunded, OrderStatus::Paid);
    assert!(result.is_err());
}

#[test]
fn test_illegal_created_to_delivered() {
    let result = OrderStateMachine::transition(OrderStatus::Created, OrderStatus::Delivered);
    assert!(result.is_err());
}

#[test]
fn test_illegal_paid_to_delivered() {
    let result = OrderStateMachine::transition(OrderStatus::Paid, OrderStatus::Delivered);
    assert!(result.is_err());
}

#[test]
fn test_illegal_shipped_to_refunded() {
    let result = OrderStateMachine::transition(OrderStatus::Shipped, OrderStatus::Refunded);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Admin-only transition detection
// ---------------------------------------------------------------------------

#[test]
fn test_admin_only_paid_to_cancelled() {
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Paid,
        &OrderStatus::Cancelled
    ));
}

#[test]
fn test_admin_only_paid_to_refunded() {
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Paid,
        &OrderStatus::Refunded
    ));
}

#[test]
fn test_not_admin_only_reserved_to_cancelled() {
    assert!(!OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Reserved,
        &OrderStatus::Cancelled
    ));
}

#[test]
fn test_not_admin_only_reserved_to_paid() {
    assert!(!OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Reserved,
        &OrderStatus::Paid
    ));
}

// ---------------------------------------------------------------------------
// Refund transition centralized validation
// ---------------------------------------------------------------------------

#[test]
fn test_delivered_to_refunded_with_context() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::Refunded,
        Some(&ctx),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), OrderStatus::Refunded);
}

#[test]
fn test_delivered_to_refunded_without_context_fails() {
    let result = OrderStateMachine::transition(OrderStatus::Delivered, OrderStatus::Refunded);
    assert!(result.is_err(), "Delivered->Refunded without context must fail");
}

#[test]
fn test_refund_rejected_outside_30_day_window() {
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(45)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::Refunded,
        Some(&ctx),
    );
    assert!(result.is_err(), "Refund outside 30-day window must fail");
}

#[test]
fn test_refund_rejected_without_reason_code() {
    let ctx = TransitionContext {
        reason_code: None,
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::Refunded,
        Some(&ctx),
    );
    assert!(result.is_err(), "Refund without reason code must fail");
}

#[test]
fn test_exchange_rejected_outside_30_day_window() {
    let ctx = TransitionContext {
        reason_code: Some("WrongItem"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(45)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ExchangeRequested,
        Some(&ctx),
    );
    assert!(result.is_err(), "Exchange outside 30-day window must fail");
}

#[test]
fn test_exchange_accepted_with_each_valid_reason_code() {
    let codes = ["Defective", "WrongItem", "NotAsDescribed", "ChangedMind", "Other"];
    for code in &codes {
        let ctx = TransitionContext {
            reason_code: Some(code),
            delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(5)),
        };
        let result = OrderStateMachine::transition_with_context(
            OrderStatus::Delivered,
            OrderStatus::ExchangeRequested,
            Some(&ctx),
        );
        assert!(result.is_ok(), "Exchange with reason '{}' should be accepted", code);
    }
}

#[test]
fn test_admin_only_all_refund_transitions() {
    // ALL refund targets require admin
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Paid,
        &OrderStatus::Refunded
    ));
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Delivered,
        &OrderStatus::Refunded
    ));
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Returned,
        &OrderStatus::Refunded
    ));
}

#[test]
fn test_refund_at_30_day_boundary() {
    // Exactly at 30 days should still be within window
    let ctx = TransitionContext {
        reason_code: Some("Defective"),
        delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(29) - chrono::Duration::hours(23)),
    };
    let result = OrderStateMachine::transition_with_context(
        OrderStatus::Delivered,
        OrderStatus::ReturnRequested,
        Some(&ctx),
    );
    assert!(result.is_ok(), "Return at nearly 30 days should still be accepted");
}

// ---------------------------------------------------------------------------
// Fulfillment transitions are admin-only
// ---------------------------------------------------------------------------

#[test]
fn test_fulfillment_transitions_are_admin_only() {
    // Processing, Shipped, Delivered, Completed must all require admin
    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Paid, &OrderStatus::Processing
    ), "Paid->Processing must be admin-only");

    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Processing, &OrderStatus::Shipped
    ), "Processing->Shipped must be admin-only");

    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Shipped, &OrderStatus::Delivered
    ), "Shipped->Delivered must be admin-only");

    assert!(OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Delivered, &OrderStatus::Completed
    ), "Delivered->Completed must be admin-only");
}

#[test]
fn test_customer_initiated_transitions_not_admin_only() {
    // Reserved -> Cancelled is user-initiated (not admin-only)
    assert!(!OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Reserved, &OrderStatus::Cancelled
    ), "Reserved->Cancelled is user-initiated, not admin-only");

    // Reserved -> Paid (payment) is user-initiated
    assert!(!OrderStateMachine::is_admin_only_transition(
        &OrderStatus::Reserved, &OrderStatus::Paid
    ), "Reserved->Paid is user-initiated");
}

#[test]
fn test_all_cancelled_except_reserved_are_admin_only() {
    let admin_cancel_sources = [
        OrderStatus::Paid,
        OrderStatus::Processing,
        OrderStatus::Shipped,
        OrderStatus::Delivered,
    ];
    for from in &admin_cancel_sources {
        assert!(OrderStateMachine::is_admin_only_transition(
            from, &OrderStatus::Cancelled
        ), "{:?}->Cancelled must be admin-only", from);
    }
}

// ---------------------------------------------------------------------------
// OrderStatus parsing
// ---------------------------------------------------------------------------

#[test]
fn test_status_from_str_valid() {
    assert_eq!(
        OrderStatus::from_str("Created").unwrap(),
        OrderStatus::Created
    );
    assert_eq!(
        OrderStatus::from_str("Reserved").unwrap(),
        OrderStatus::Reserved
    );
    assert_eq!(
        OrderStatus::from_str("Cancelled").unwrap(),
        OrderStatus::Cancelled
    );
    assert_eq!(
        OrderStatus::from_str("Delivered").unwrap(),
        OrderStatus::Delivered
    );
}

#[test]
fn test_status_from_str_invalid() {
    assert!(OrderStatus::from_str("Unknown").is_err());
    assert!(OrderStatus::from_str("").is_err());
    assert!(OrderStatus::from_str("created").is_err()); // case-sensitive
}

#[test]
fn test_status_roundtrip() {
    let statuses = vec![
        OrderStatus::Created,
        OrderStatus::Reserved,
        OrderStatus::Paid,
        OrderStatus::Processing,
        OrderStatus::Shipped,
        OrderStatus::Delivered,
        OrderStatus::Completed,
        OrderStatus::Cancelled,
        OrderStatus::RefundRequested,
        OrderStatus::Refunded,
        OrderStatus::ReturnRequested,
        OrderStatus::Returned,
        OrderStatus::ExchangeRequested,
        OrderStatus::Exchanged,
    ];
    for status in statuses {
        let s = status.as_str();
        let parsed = OrderStatus::from_str(s).unwrap();
        assert_eq!(parsed, status);
    }
}
