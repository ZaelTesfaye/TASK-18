use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database records
// ---------------------------------------------------------------------------

/// Review template defining expected fields for a round.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewTemplate {
    pub id: Uuid,
    pub name: String,
    pub version: i32,
    pub schema: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A review round scoped to a product and template.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewRound {
    pub id: Uuid,
    pub product_id: Uuid,
    pub template_id: Uuid,
    pub round_number: i32,
    pub deadline: DateTime<Utc>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Individual review submission within a round.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewSubmission {
    pub id: Uuid,
    pub round_id: Uuid,
    pub reviewer_id: Uuid,
    pub template_version: i32,
    pub content: serde_json::Value,
    pub version: i32,
    pub status: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Historical version snapshot of a submission.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewSubmissionHistory {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub version: i32,
    pub content: serde_json::Value,
    pub submitted_at: DateTime<Utc>,
}

/// File attachment linked to a review submission.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewAttachment {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    /// Binary file data; omitted from most API responses.
    #[serde(skip_serializing)]
    pub file_data: Vec<u8>,
    pub approval_status: String,
    pub uploaded_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRoundResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub template_id: Uuid,
    pub template_name: String,
    pub template_schema: Option<serde_json::Value>,
    pub round_number: i32,
    pub deadline: DateTime<Utc>,
    pub is_active: bool,
    pub submissions: Vec<SubmissionResponse>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    pub id: Uuid,
    pub round_id: Uuid,
    pub reviewer_id: Uuid,
    pub reviewer_username: Option<String>,
    pub template_version: i32,
    pub content: serde_json::Value,
    pub version: i32,
    pub status: String,
    pub attachments: Vec<AttachmentInfo>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight attachment metadata (no binary data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub approval_status: String,
    pub uploaded_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSubmissionRequest {
    pub round_id: Uuid,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitReviewRequest {
    pub content: serde_json::Value,
}
