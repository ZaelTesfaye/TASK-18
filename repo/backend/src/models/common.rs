use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Generic paginated response wrapper.
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// Constructs a paginated response, computing `total_pages` automatically.
    pub fn new(items: Vec<T>, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = if per_page > 0 {
            (total + per_page - 1) / per_page
        } else {
            0
        };
        Self {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}

/// Thin f64 wrapper that can be decoded from SQL NUMERIC columns via BigDecimal.
///
/// Use with `#[sqlx(try_from = "bigdecimal::BigDecimal")]` on struct fields
/// that map to NUMERIC/DECIMAL database columns.
///
/// - Serializes/deserializes as a plain JSON number (`#[serde(transparent)]`)
/// - Derefs to `f64` so arithmetic and comparisons work transparently
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Decimal(pub f64);

impl TryFrom<BigDecimal> for Decimal {
    type Error = String;
    fn try_from(bd: BigDecimal) -> Result<Self, Self::Error> {
        bd.to_f64()
            .map(Decimal)
            .ok_or_else(|| "BigDecimal value out of f64 range".to_string())
    }
}

impl Deref for Decimal {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl From<f64> for Decimal {
    fn from(v: f64) -> Self {
        Decimal(v)
    }
}

impl From<Decimal> for f64 {
    fn from(d: Decimal) -> f64 {
        d.0
    }
}
