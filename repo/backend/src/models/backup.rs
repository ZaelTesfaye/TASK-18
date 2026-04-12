use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Full backup row from the `backups` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Backup {
    pub id: Uuid,
    pub filename: String,
    pub checksum_sha256: String,
    pub size_bytes: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}
