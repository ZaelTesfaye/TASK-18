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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // -- PaginatedResponse --

    #[test]
    fn test_paginated_response_total_pages() {
        let resp = PaginatedResponse::new(vec![1, 2, 3], 25, 1, 10);
        assert_eq!(resp.total_pages, 3); // ceil(25/10)
    }

    #[test]
    fn test_paginated_response_exact_multiple() {
        let resp = PaginatedResponse::new(vec![1, 2], 20, 2, 10);
        assert_eq!(resp.total_pages, 2);
    }

    #[test]
    fn test_paginated_response_zero_per_page() {
        let resp = PaginatedResponse::<i32>::new(vec![], 100, 1, 0);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn test_paginated_response_zero_total() {
        let resp = PaginatedResponse::<i32>::new(vec![], 0, 1, 10);
        assert_eq!(resp.total_pages, 0);
    }

    #[test]
    fn test_paginated_response_single_item() {
        let resp = PaginatedResponse::new(vec!["a"], 1, 1, 10);
        assert_eq!(resp.total_pages, 1);
    }

    #[test]
    fn test_paginated_response_serializes_to_json() {
        let resp = PaginatedResponse::new(vec![42], 100, 3, 20);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 100);
        assert_eq!(json["page"], 3);
        assert_eq!(json["per_page"], 20);
        assert_eq!(json["total_pages"], 5);
        assert_eq!(json["items"][0], 42);
    }

    // -- Decimal --

    #[test]
    fn test_decimal_from_bigdecimal() {
        let bd = BigDecimal::from_str("19.99").unwrap();
        let d = Decimal::try_from(bd).unwrap();
        assert!((d.0 - 19.99).abs() < 0.001);
    }

    #[test]
    fn test_decimal_deref_to_f64() {
        let d = Decimal(42.5);
        let result: f64 = *d * 2.0;
        assert!((result - 85.0).abs() < 0.001);
    }

    #[test]
    fn test_decimal_from_f64() {
        let d = Decimal::from(3.14);
        assert_eq!(d.0, 3.14);
    }

    #[test]
    fn test_decimal_into_f64() {
        let d = Decimal(9.99);
        let f: f64 = d.into();
        assert_eq!(f, 9.99);
    }

    #[test]
    fn test_decimal_serde_roundtrip() {
        let d = Decimal(19.99);
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, "19.99");
        let d2: Decimal = serde_json::from_str(&json).unwrap();
        assert_eq!(d, d2);
    }

    #[test]
    fn test_decimal_transparent_in_struct() {
        #[derive(Serialize, Deserialize)]
        struct Price { amount: Decimal }
        let p = Price { amount: Decimal(29.99) };
        let json = serde_json::to_value(&p).unwrap();
        // amount must be a JSON number, not a string
        assert_eq!(json["amount"], 29.99);
    }

    #[test]
    fn test_decimal_equality() {
        assert_eq!(Decimal(1.0), Decimal(1.0));
        assert_ne!(Decimal(1.0), Decimal(2.0));
    }

    #[test]
    fn test_decimal_copy() {
        let d = Decimal(5.0);
        let d2 = d; // Copy
        assert_eq!(d, d2); // both still usable
    }
}
