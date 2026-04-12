use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::custom_field::CustomFieldValue;
use super::taxonomy::{TagResponse, TopicResponse};

// ---------------------------------------------------------------------------
// Database record
// ---------------------------------------------------------------------------

/// Full product row from the `products` table.
///
/// Note: `price` is stored as `DECIMAL(12,2)` in PostgreSQL. When reading via
/// `sqlx::query_as!` the macro handles the conversion automatically.  For
/// manual `query_as::<_, Product>` calls, map the column with an explicit cast
/// (`price::FLOAT8`) or construct the struct by hand.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: f64,
    pub stock: i32,
    pub image_url: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Rich response (includes related data)
// ---------------------------------------------------------------------------

/// Product response enriched with taxonomy, custom fields, and aggregate score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: f64,
    pub stock: i32,
    pub image_url: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i32>,
    pub is_active: bool,
    pub topics: Vec<TopicResponse>,
    pub tags: Vec<TagResponse>,
    pub custom_fields: Vec<CustomFieldValue>,
    pub average_score: Option<f64>,
    pub total_ratings: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductRequest {
    pub title: String,
    pub description: Option<String>,
    pub price: f64,
    pub stock: Option<i32>,
    pub image_url: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i32>,
    pub topic_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProductRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub stock: Option<i32>,
    pub image_url: Option<String>,
    pub genre: Option<String>,
    pub release_year: Option<i32>,
    pub is_active: Option<bool>,
    pub topic_ids: Option<Vec<Uuid>>,
    pub tag_ids: Option<Vec<Uuid>>,
}

// ---------------------------------------------------------------------------
// Faceted filtering
// ---------------------------------------------------------------------------

/// Query parameters for product listing with faceted filtering and pagination.
#[derive(Debug, Clone, Deserialize)]
pub struct ProductFilter {
    pub topic_id: Option<Uuid>,
    pub tag_id: Option<Uuid>,
    pub genre: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub search: Option<String>,
    pub custom_field_name: Option<String>,
    pub custom_field_value: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl ProductFilter {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}
