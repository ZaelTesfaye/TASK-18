use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Full audit_log row from the `audit_log` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor: String,
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub change_summary: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for filtering and paginating audit logs.
///
/// `from_date` and `to_date` accept both RFC3339 datetimes (`2024-01-01T00:00:00Z`)
/// and bare dates (`2024-01-01`). Bare dates are internally converted to
/// start-of-day / end-of-day UTC bounds.
#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl AuditQuery {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }

    /// Parses from_date flexibly: accepts RFC3339 or bare YYYY-MM-DD (→ start of day UTC).
    pub fn parsed_from_date(&self) -> Option<DateTime<Utc>> {
        self.from_date.as_deref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                })
        })
    }

    /// Parses to_date flexibly: accepts RFC3339 or bare YYYY-MM-DD (→ end of day UTC).
    pub fn parsed_to_date(&self) -> Option<DateTime<Utc>> {
        self.to_date.as_deref().and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
                .or_else(|| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .and_then(|d| d.and_hms_opt(23, 59, 59))
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                })
        })
    }
}
