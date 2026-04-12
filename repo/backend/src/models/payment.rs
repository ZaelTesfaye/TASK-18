use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Full payment_events row from the `payment_events` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PaymentEvent {
    pub id: Uuid,
    pub order_id: Uuid,
    pub idempotency_key: String,
    #[sqlx(try_from = "bigdecimal::BigDecimal")]
    pub amount: f64,
    pub status: String,
    pub payment_method: String,
    pub response_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Request body for the local payment simulator.
#[derive(Debug, Clone, Deserialize)]
pub struct SimulatePaymentRequest {
    pub order_id: Uuid,
    pub amount: f64,
    pub payment_method: Option<String>,
    /// The desired outcome: "Success", "Failed", or "Timeout".
    pub outcome: String,
    /// Attempt number for idempotency key generation.
    pub attempt_number: i32,
}

/// Full invoice row from the `invoices` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub order_id: Uuid,
    pub invoice_number: String,
    #[sqlx(try_from = "bigdecimal::BigDecimal")]
    pub total_amount: f64,
    pub line_items: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
