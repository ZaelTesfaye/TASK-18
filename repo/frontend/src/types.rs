use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Generic wrapper ──────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiError {
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub status: u16,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.message.is_empty() {
            write!(f, "{}", self.message)
        } else if !self.error.is_empty() {
            write!(f, "{}", self.error)
        } else {
            write!(f, "Unknown error (status {})", self.status)
        }
    }
}

// ── Auth / Users ─────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub role: String,
    #[serde(default, alias = "is_locked")]
    pub locked: bool,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub role: String,
    #[serde(default, alias = "is_locked")]
    pub locked: bool,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub user: Option<UserResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RefreshResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
}

// ── Products ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Product {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub genre: String,
    #[serde(default)]
    pub topics: Vec<TopicRef>,
    #[serde(default)]
    pub tags: Vec<TagRef>,
    #[serde(default)]
    pub custom_fields: serde_json::Value,
    #[serde(default, alias = "average_score")]
    pub aggregate_score: Option<f64>,
    #[serde(default)]
    pub stock: Option<u32>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TopicRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TagRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ProductFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_field_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProductResponse {
    #[serde(flatten)]
    pub product: Product,
    #[serde(default)]
    pub ratings: Vec<Rating>,
    #[serde(default)]
    pub rating_count: u64,
}

// ── Cart ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Cart {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub items: Vec<CartItem>,
    #[serde(default, alias = "total_amount")]
    pub total: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CartItem {
    pub id: String,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub product_title: String,
    #[serde(default, alias = "product_price")]
    pub unit_price: f64,
    #[serde(default)]
    pub quantity: u32,
    #[serde(default)]
    pub line_total: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddToCartRequest {
    pub product_id: String,
    #[serde(default = "default_qty")]
    pub quantity: u32,
}

fn default_qty() -> u32 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateCartItemRequest {
    pub quantity: u32,
}

// ── Orders ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct StatusTimeline {
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub reservation_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub shipped_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub delivered_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub cancelled_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub refunded_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Order {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub items: Vec<OrderItem>,
    #[serde(default, alias = "total_amount")]
    pub total: f64,
    #[serde(default)]
    pub shipping_address: Option<ShippingAddress>,
    #[serde(default)]
    pub payment_method: Option<String>,
    #[serde(default)]
    pub status_timeline: Option<StatusTimeline>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderItem {
    pub id: String,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub product_title: String,
    #[serde(default)]
    pub quantity: u32,
    #[serde(default)]
    pub unit_price: f64,
    #[serde(default, alias = "total_price")]
    pub line_total: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ShippingAddress {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub street: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub zip: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderResponse {
    #[serde(flatten)]
    pub order: Order,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderItemRequest {
    pub product_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    /// Encrypted on the backend; frontend sends plaintext address string.
    pub shipping_address: String,
    /// Payment method selected by the user. Persisted on the order record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
    #[serde(default)]
    pub items: Vec<OrderItemRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReturnRequest {
    pub reason_code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaymentResponse {
    pub id: String,
    #[serde(default)]
    pub order_id: String,
    #[serde(default)]
    pub idempotency_key: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub payment_method: String,
    #[serde(default)]
    pub response_data: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

// ── Ratings ──────────────────────────────────────────────────────

/// Matches backend models/rating.rs RatingResponse.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rating {
    pub id: String,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub dimensions: Vec<DimensionScore>,
    #[serde(default)]
    pub average: f64,
    #[serde(default)]
    pub moderation_status: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DimensionScore {
    #[serde(alias = "dimension")]
    pub dimension_name: String,
    pub score: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRatingRequest {
    pub product_id: String,
    pub dimensions: Vec<DimensionScore>,
}

// ── Leaderboards ─────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardEntry {
    #[serde(default)]
    pub rank: u64,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub product_title: String,
    #[serde(default)]
    pub average_score: f64,
    #[serde(default)]
    pub total_ratings: u64,
    #[serde(default, alias = "last_rating_at")]
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LeaderboardQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
}

// ── Reviews ──────────────────────────────────────────────────────

/// Matches backend models/review.rs ReviewRoundResponse.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReviewRound {
    pub id: String,
    #[serde(default)]
    pub product_id: String,
    #[serde(default)]
    pub template_id: String,
    #[serde(default)]
    pub template_name: String,
    #[serde(default)]
    pub template_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub round_number: i32,
    #[serde(default)]
    pub deadline: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub submissions: Vec<ReviewSubmission>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReviewSubmission {
    pub id: String,
    #[serde(default)]
    pub round_id: String,
    #[serde(default)]
    pub reviewer_id: String,
    #[serde(default)]
    pub reviewer_username: Option<String>,
    #[serde(default)]
    pub template_version: i32,
    #[serde(default)]
    pub content: serde_json::Value,
    #[serde(default)]
    pub attachments: Vec<AttachmentInfo>,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub submitted_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AttachmentInfo {
    pub id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub approval_status: String,
    #[serde(default)]
    pub uploaded_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitReviewRequest {
    pub content: serde_json::Value,
}

// ── Taxonomy ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Topic {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub children: Vec<Topic>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

// ── Custom Fields ────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CustomFieldDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub field_type: String,
    #[serde(default)]
    pub allowed_values: Option<serde_json::Value>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub conflict_count: i32,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateFieldRequest {
    pub name: String,
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateFieldRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
}

// ── Admin ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditLogEntry {
    pub id: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default, alias = "change_summary")]
    pub details: serde_json::Value,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuditLogQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RiskEvent {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub override_justification: Option<String>,
    #[serde(default)]
    pub overridden_by: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ReportQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// Matches backend routes/reports.rs ReportResponse exactly.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReportResponse {
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub report_type: String,
    #[serde(default)]
    pub orders: ReportOrderStats,
    #[serde(default)]
    pub revenue: ReportRevenueStats,
    #[serde(default)]
    pub users: ReportUserStats,
    #[serde(default)]
    pub ratings: ReportRatingStats,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportOrderStats {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub by_status: Vec<StatusCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StatusCount {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportRevenueStats {
    #[serde(default)]
    pub total_revenue: f64,
    #[serde(default)]
    pub total_discount: f64,
    #[serde(default)]
    pub net_revenue: f64,
    #[serde(default)]
    pub average_order_value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportUserStats {
    #[serde(default)]
    pub total_users: u64,
    #[serde(default)]
    pub new_users_in_period: u64,
    #[serde(default)]
    pub active_shoppers: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportRatingStats {
    #[serde(default)]
    pub total_ratings: u64,
    #[serde(default)]
    pub new_ratings_in_period: u64,
    #[serde(default)]
    pub average_score: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BackupResponse {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulatePaymentRequest {
    pub order_id: String,
    pub amount: f64,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
    #[serde(default = "default_attempt")]
    pub attempt_number: i32,
}

fn default_attempt() -> i32 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTopicRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteTopicRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}
