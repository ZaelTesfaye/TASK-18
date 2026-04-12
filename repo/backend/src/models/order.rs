use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database records
// ---------------------------------------------------------------------------

/// Full order row from the `orders` table.
///
/// Decimal columns (`total_amount`, `discount_amount`) are represented as `f64`.
/// Use SQL casts (`::FLOAT8`) or manual mapping when reading rows.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub parent_order_id: Option<Uuid>,
    pub shipping_address_encrypted: String,
    pub total_amount: f64,
    pub discount_amount: f64,
    pub reason_code: Option<String>,
    pub payment_method: Option<String>,
    pub reservation_expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub legal_hold: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Full order_items row.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub unit_price: f64,
    pub total_price: f64,
    pub created_at: DateTime<Utc>,
}

/// Order lineage tracking for split/merge operations.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrderLineage {
    pub id: Uuid,
    pub parent_order_id: Uuid,
    pub child_order_id: Uuid,
    pub operation: String,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

/// Order response enriched with items and status timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub parent_order_id: Option<Uuid>,
    pub total_amount: f64,
    pub discount_amount: f64,
    pub reason_code: Option<String>,
    pub payment_method: Option<String>,
    pub items: Vec<OrderItemResponse>,
    pub status_timeline: StatusTimeline,
    pub legal_hold: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_title: String,
    pub quantity: i32,
    pub unit_price: f64,
    pub total_price: f64,
}

/// Key timestamps in the lifecycle of an order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTimeline {
    pub created_at: DateTime<Utc>,
    pub reservation_expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

/// Request to create a new order.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderRequest {
    pub shipping_address: String,
    /// Payment method selected by the user (e.g. "CreditCard", "DebitCard").
    /// Persisted on the order record and also used in payment events.
    #[serde(default)]
    pub payment_method: Option<String>,
    pub items: Vec<OrderItemRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderItemRequest {
    pub product_id: Uuid,
    pub quantity: i32,
}

/// Split an order into two: one containing the specified items, the rest remaining.
#[derive(Debug, Clone, Deserialize)]
pub struct SplitOrderRequest {
    pub item_ids: Vec<Uuid>,
}

/// Merge multiple orders into one.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeOrderRequest {
    pub order_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnRequest {
    pub reason_code: String,
}
