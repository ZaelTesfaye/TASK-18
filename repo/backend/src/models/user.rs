use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Database record
// ---------------------------------------------------------------------------

/// Full user row from the `users` table.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub phone_encrypted: Option<String>,
    pub address_encrypted: Option<String>,
    pub verified_possession: bool,
    pub is_locked: bool,
    pub locked_until: Option<DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub last_failed_login: Option<DateTime<Utc>>,
    pub reset_token_hash: Option<String>,
    pub reset_token_expires_at: Option<DateTime<Utc>>,
    pub legal_hold: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Safe public response (no password hash, masked PII)
// ---------------------------------------------------------------------------

/// Public-facing user representation that never exposes `password_hash`
/// or raw encrypted fields. Phone and address are masked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: String,
    pub phone_masked: Option<String>,
    pub address_masked: Option<String>,
    pub verified_possession: bool,
    pub is_locked: bool,
    pub legal_hold: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserResponse {
    /// Converts a full `User` record into a safe response, masking sensitive fields.
    pub fn from_user(user: &User, decrypted_phone: Option<&str>, decrypted_address: Option<&str>) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            phone_masked: decrypted_phone.map(mask_phone),
            address_masked: decrypted_address.map(mask_address),
            verified_possession: user.verified_possession,
            is_locked: user.is_locked,
            legal_hold: user.legal_hold,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// Masks a phone number, showing only the last four digits.
fn mask_phone(phone: &str) -> String {
    if phone.len() <= 4 {
        return "****".to_string();
    }
    let visible = &phone[phone.len() - 4..];
    format!("***-***-{}", visible)
}

/// Masks an address, showing only the first 5 characters followed by ellipsis.
fn mask_address(address: &str) -> String {
    if address.len() <= 5 {
        return "*****".to_string();
    }
    format!("{}...", &address[..5])
}

// ---------------------------------------------------------------------------
// Request DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: String,
}
