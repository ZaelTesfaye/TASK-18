use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Custom field definition from the `custom_field_definitions` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CustomFieldDefinition {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub field_type: String,
    pub allowed_values: Option<serde_json::Value>,
    pub status: String,
    pub version: i32,
    pub previous_type: Option<String>,
    pub previous_allowed_values: Option<serde_json::Value>,
    pub conflict_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Custom field value per product from the `custom_field_values` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CustomFieldValue {
    pub id: Uuid,
    pub product_id: Uuid,
    pub field_id: Uuid,
    pub value: serde_json::Value,
    pub field_version: i32,
    pub conflict_status: String,
    pub conflict_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateFieldDefinitionRequest {
    pub name: String,
    pub field_type: String,
    pub allowed_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetFieldValueRequest {
    pub value: serde_json::Value,
}
