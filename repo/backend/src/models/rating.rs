use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database records
// ---------------------------------------------------------------------------

/// Full rating row from the `ratings` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Rating {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub moderation_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual dimension score within a rating.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RatingDimension {
    pub id: Uuid,
    pub rating_id: Uuid,
    pub dimension_name: String,
    pub score: i32,
}

/// Materialized product aggregate scores.
///
/// `average_score` is `DECIMAL(4,2)` in PostgreSQL. Use SQL casts or manual
/// mapping when reading rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductScore {
    pub product_id: Uuid,
    pub average_score: Option<f64>,
    pub total_ratings: i32,
    pub last_rating_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub dimensions: Vec<DimensionScore>,
    pub average: f64,
    pub moderation_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub product_id: Uuid,
    pub product_title: String,
    pub average_score: f64,
    pub total_ratings: i32,
    pub genre: Option<String>,
}

// ---------------------------------------------------------------------------
// Request / query DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRatingRequest {
    pub product_id: Uuid,
    pub dimensions: Vec<DimensionScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension_name: String,
    /// Score between 1 and 10 (inclusive).
    pub score: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LeaderboardQuery {
    /// One of: weekly, monthly, genre.
    pub period: Option<String>,
    pub genre: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl LeaderboardQuery {
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
