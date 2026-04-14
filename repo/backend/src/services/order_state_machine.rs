use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::errors::AppError;

// ---------------------------------------------------------------------------
// OrderStatus enum
// ---------------------------------------------------------------------------

/// All possible order statuses, mirroring the Postgres `order_status` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    Created,
    Reserved,
    Paid,
    Processing,
    Shipped,
    Delivered,
    Completed,
    Cancelled,
    RefundRequested,
    Refunded,
    ReturnRequested,
    Returned,
    ExchangeRequested,
    Exchanged,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl OrderStatus {
    /// Parses a database string into an `OrderStatus`.
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "Created" => Ok(OrderStatus::Created),
            "Reserved" => Ok(OrderStatus::Reserved),
            "Paid" => Ok(OrderStatus::Paid),
            "Processing" => Ok(OrderStatus::Processing),
            "Shipped" => Ok(OrderStatus::Shipped),
            "Delivered" => Ok(OrderStatus::Delivered),
            "Completed" => Ok(OrderStatus::Completed),
            "Cancelled" => Ok(OrderStatus::Cancelled),
            "RefundRequested" => Ok(OrderStatus::RefundRequested),
            "Refunded" => Ok(OrderStatus::Refunded),
            "ReturnRequested" => Ok(OrderStatus::ReturnRequested),
            "Returned" => Ok(OrderStatus::Returned),
            "ExchangeRequested" => Ok(OrderStatus::ExchangeRequested),
            "Exchanged" => Ok(OrderStatus::Exchanged),
            _ => Err(AppError::BadRequest(format!("Unknown order status: {}", s))),
        }
    }

    /// Returns the database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Created => "Created",
            OrderStatus::Reserved => "Reserved",
            OrderStatus::Paid => "Paid",
            OrderStatus::Processing => "Processing",
            OrderStatus::Shipped => "Shipped",
            OrderStatus::Delivered => "Delivered",
            OrderStatus::Completed => "Completed",
            OrderStatus::Cancelled => "Cancelled",
            OrderStatus::RefundRequested => "RefundRequested",
            OrderStatus::Refunded => "Refunded",
            OrderStatus::ReturnRequested => "ReturnRequested",
            OrderStatus::Returned => "Returned",
            OrderStatus::ExchangeRequested => "ExchangeRequested",
            OrderStatus::Exchanged => "Exchanged",
        }
    }
}

// ---------------------------------------------------------------------------
// Legal transition map
// ---------------------------------------------------------------------------

/// Static map of legal (source -> set of targets) transitions.
static TRANSITIONS: Lazy<HashMap<OrderStatus, Vec<OrderStatus>>> = Lazy::new(|| {
    let mut m: HashMap<OrderStatus, Vec<OrderStatus>> = HashMap::new();

    m.insert(OrderStatus::Created, vec![OrderStatus::Reserved]);
    m.insert(
        OrderStatus::Reserved,
        vec![OrderStatus::Paid, OrderStatus::Cancelled],
    );
    m.insert(
        OrderStatus::Paid,
        vec![
            OrderStatus::Processing,
            OrderStatus::Refunded,
            OrderStatus::Cancelled,
        ],
    );
    m.insert(OrderStatus::Processing, vec![OrderStatus::Shipped]);
    m.insert(OrderStatus::Shipped, vec![OrderStatus::Delivered]);
    m.insert(
        OrderStatus::Delivered,
        vec![
            OrderStatus::Completed,
            OrderStatus::ReturnRequested,
            OrderStatus::ExchangeRequested,
            OrderStatus::Refunded,
        ],
    );
    m.insert(OrderStatus::ReturnRequested, vec![OrderStatus::Returned]);
    m.insert(OrderStatus::Returned, vec![OrderStatus::Refunded]);
    m.insert(
        OrderStatus::ExchangeRequested,
        vec![OrderStatus::Exchanged],
    );

    m
});

/// Transitions that require admin privileges.
static ADMIN_TRANSITIONS: Lazy<Vec<(OrderStatus, OrderStatus)>> = Lazy::new(|| {
    vec![
        (OrderStatus::Paid, OrderStatus::Cancelled),
    ]
});

// ---------------------------------------------------------------------------
// Transition context (for transitions that require business-rule validation)
// ---------------------------------------------------------------------------

/// Optional context supplied by the caller when attempting a transition.
/// Transitions to return/exchange/refund statuses require a reason code and
/// delivery-date validation. The state machine enforces these centrally so
/// that no endpoint can bypass the checks.
pub struct TransitionContext<'a> {
    pub reason_code: Option<&'a str>,
    pub delivered_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Valid return-reason codes matching the DB `return_reason` enum.
pub const VALID_REASON_CODES: &[&str] = &[
    "Defective", "WrongItem", "NotAsDescribed", "ChangedMind", "Other",
];

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Enforces the order lifecycle state machine.
pub struct OrderStateMachine;

impl OrderStateMachine {
    /// Attempts to transition from `current` to `target`.
    ///
    /// Returns the new status on success, or `AppError::BadRequest` if the
    /// transition is illegal.
    pub fn transition(
        current: OrderStatus,
        target: OrderStatus,
    ) -> Result<OrderStatus, AppError> {
        Self::transition_with_context(current, target, None)
    }

    /// Transition with optional business-rule context.
    ///
    /// Transitions to `ReturnRequested`, `ExchangeRequested`, or `Refunded`
    /// (from Delivered) **require** a `TransitionContext` containing a valid
    /// reason code and a delivery date within the 30-day return window.
    pub fn transition_with_context(
        current: OrderStatus,
        target: OrderStatus,
        ctx: Option<&TransitionContext<'_>>,
    ) -> Result<OrderStatus, AppError> {
        // Admin override: Any -> Cancelled
        if target == OrderStatus::Cancelled {
            return Ok(target);
        }

        // Enforce reason-code and 30-day window for return/exchange/refund paths.
        // ALL transitions to Refunded, ReturnRequested, or ExchangeRequested require
        // a reason code and delivery-date validation — no bypass via admin or generic endpoint.
        let needs_return_validation = matches!(
            target,
            OrderStatus::ReturnRequested
                | OrderStatus::ExchangeRequested
                | OrderStatus::Refunded
        );

        if needs_return_validation {
            let ctx = ctx.ok_or_else(|| {
                AppError::BadRequest(
                    "Return, exchange, and refund transitions require a reason code \
                     and must go through the dedicated endpoint."
                        .to_string(),
                )
            })?;

            // Validate reason code
            let reason = ctx.reason_code.ok_or_else(|| {
                AppError::ValidationError(
                    "Reason code is required for return/exchange/refund requests".to_string(),
                )
            })?;
            if !VALID_REASON_CODES.contains(&reason) {
                return Err(AppError::ValidationError(format!(
                    "Invalid reason code '{}'. Must be one of: {}",
                    reason,
                    VALID_REASON_CODES.join(", ")
                )));
            }

            // Validate 30-day delivery window
            let delivered_at = ctx.delivered_at.ok_or_else(|| {
                AppError::BadRequest("Order has not been delivered yet".to_string())
            })?;
            let deadline = delivered_at + chrono::Duration::days(30);
            if chrono::Utc::now() > deadline {
                return Err(AppError::BadRequest(
                    "Return window has expired (30 days from delivery)".to_string(),
                ));
            }
        }

        if let Some(allowed) = TRANSITIONS.get(&current) {
            if allowed.contains(&target) {
                return Ok(target);
            }
        }

        Err(AppError::BadRequest(format!(
            "Invalid order transition: {} -> {}. \
             Allowed transitions from {} are: {}",
            current,
            target,
            current,
            TRANSITIONS
                .get(&current)
                .map(|v| v
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(", "))
                .unwrap_or_else(|| "none (terminal state)".to_string())
        )))
    }

    /// Returns `true` if the given transition requires admin privileges.
    ///
    /// Fulfillment transitions (Processing, Shipped, Delivered, Completed) are
    /// operational and must be performed by admin/fulfillment staff, not by
    /// order owners. Customer-initiated actions (cancel from Reserved, return,
    /// exchange) go through their dedicated endpoints.
    pub fn is_admin_only_transition(from: &OrderStatus, to: &OrderStatus) -> bool {
        // Any -> Cancelled (except Reserved -> Cancelled which is user-initiated)
        if *to == OrderStatus::Cancelled && *from != OrderStatus::Reserved {
            return true;
        }
        // All refund transitions require admin
        if *to == OrderStatus::Refunded {
            return true;
        }
        // Fulfillment transitions — only admin/fulfillment staff
        if matches!(
            to,
            OrderStatus::Processing
                | OrderStatus::Shipped
                | OrderStatus::Delivered
                | OrderStatus::Completed
        ) {
            return true;
        }
        ADMIN_TRANSITIONS
            .iter()
            .any(|(f, t)| f == from && t == to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- OrderStatus parsing --

    #[test]
    fn test_status_from_str_all_valid() {
        let statuses = vec![
            "Created", "Reserved", "Paid", "Processing", "Shipped",
            "Delivered", "Completed", "Cancelled", "RefundRequested",
            "Refunded", "ReturnRequested", "Returned", "ExchangeRequested", "Exchanged",
        ];
        for s in statuses {
            let status = OrderStatus::from_str(s).unwrap();
            assert_eq!(status.as_str(), s);
        }
    }

    #[test]
    fn test_status_from_str_invalid() {
        assert!(OrderStatus::from_str("Invalid").is_err());
        assert!(OrderStatus::from_str("").is_err());
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", OrderStatus::Paid), "Paid");
        assert_eq!(format!("{}", OrderStatus::ReturnRequested), "ReturnRequested");
    }

    // -- Happy-path transitions --

    #[test]
    fn test_full_happy_path() {
        let mut s = OrderStatus::Created;
        s = OrderStateMachine::transition(s, OrderStatus::Reserved).unwrap();
        s = OrderStateMachine::transition(s, OrderStatus::Paid).unwrap();
        s = OrderStateMachine::transition(s, OrderStatus::Processing).unwrap();
        s = OrderStateMachine::transition(s, OrderStatus::Shipped).unwrap();
        s = OrderStateMachine::transition(s, OrderStatus::Delivered).unwrap();
        s = OrderStateMachine::transition(s, OrderStatus::Completed).unwrap();
        assert_eq!(s, OrderStatus::Completed);
    }

    #[test]
    fn test_reserved_to_cancelled() {
        let result = OrderStateMachine::transition(OrderStatus::Reserved, OrderStatus::Cancelled);
        assert!(result.is_ok());
    }

    // -- Illegal transitions --

    #[test]
    fn test_cannot_skip_states() {
        let result = OrderStateMachine::transition(OrderStatus::Created, OrderStatus::Paid);
        assert!(result.is_err());
    }

    #[test]
    fn test_completed_is_terminal() {
        let result = OrderStateMachine::transition(OrderStatus::Completed, OrderStatus::Shipped);
        assert!(result.is_err());
    }

    // -- Return validation --

    #[test]
    fn test_return_requires_context() {
        let result = OrderStateMachine::transition(
            OrderStatus::Delivered,
            OrderStatus::ReturnRequested,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_requires_reason_code() {
        let ctx = TransitionContext {
            reason_code: None,
            delivered_at: Some(chrono::Utc::now()),
        };
        let result = OrderStateMachine::transition_with_context(
            OrderStatus::Delivered,
            OrderStatus::ReturnRequested,
            Some(&ctx),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_rejects_invalid_reason() {
        let ctx = TransitionContext {
            reason_code: Some("InvalidReason"),
            delivered_at: Some(chrono::Utc::now()),
        };
        let result = OrderStateMachine::transition_with_context(
            OrderStatus::Delivered,
            OrderStatus::ReturnRequested,
            Some(&ctx),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_return_within_window_succeeds() {
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
    }

    #[test]
    fn test_return_expired_window_fails() {
        let ctx = TransitionContext {
            reason_code: Some("Defective"),
            delivered_at: Some(chrono::Utc::now() - chrono::Duration::days(31)),
        };
        let result = OrderStateMachine::transition_with_context(
            OrderStatus::Delivered,
            OrderStatus::ReturnRequested,
            Some(&ctx),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_exchange_requires_context() {
        let result = OrderStateMachine::transition(
            OrderStatus::Delivered,
            OrderStatus::ExchangeRequested,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_all_valid_reason_codes() {
        for code in VALID_REASON_CODES {
            let ctx = TransitionContext {
                reason_code: Some(code),
                delivered_at: Some(chrono::Utc::now()),
            };
            let result = OrderStateMachine::transition_with_context(
                OrderStatus::Delivered,
                OrderStatus::ReturnRequested,
                Some(&ctx),
            );
            assert!(result.is_ok(), "Reason code '{}' should be valid", code);
        }
    }

    // -- Admin transitions --

    #[test]
    fn test_admin_only_fulfillment_transitions() {
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Paid, &OrderStatus::Processing));
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Processing, &OrderStatus::Shipped));
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Shipped, &OrderStatus::Delivered));
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Delivered, &OrderStatus::Completed));
    }

    #[test]
    fn test_user_initiated_cancel_not_admin_only() {
        assert!(!OrderStateMachine::is_admin_only_transition(&OrderStatus::Reserved, &OrderStatus::Cancelled));
    }

    #[test]
    fn test_admin_cancel_from_paid() {
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Paid, &OrderStatus::Cancelled));
    }

    #[test]
    fn test_refund_is_admin_only() {
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Delivered, &OrderStatus::Refunded));
        assert!(OrderStateMachine::is_admin_only_transition(&OrderStatus::Returned, &OrderStatus::Refunded));
    }

    // -- Cancel override --

    #[test]
    fn test_cancel_override_from_any_state() {
        // Admin cancel override works from any state
        for status in &[
            OrderStatus::Created, OrderStatus::Reserved, OrderStatus::Paid,
            OrderStatus::Processing, OrderStatus::Shipped,
        ] {
            let result = OrderStateMachine::transition(*status, OrderStatus::Cancelled);
            assert!(result.is_ok(), "Cancel should be allowed from {:?}", status);
        }
    }
}
