use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Token claims
// ---------------------------------------------------------------------------

/// JWT claims embedded in both access and refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject -- the user ID.
    pub sub: Uuid,
    /// User role (e.g. "Shopper", "Reviewer", "Admin").
    pub role: String,
    /// Token type: "access" or "refresh".
    pub typ: String,
    /// Expiration time (UTC epoch seconds).
    pub exp: i64,
    /// Issued-at time (UTC epoch seconds).
    pub iat: i64,
    /// Unique token identifier for revocation tracking.
    pub jti: String,
}

// ---------------------------------------------------------------------------
// Password hashing
// ---------------------------------------------------------------------------

/// Hashes a plaintext password using Argon2id with a random salt.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::InternalError(format!("Password hashing failed: {}", e)))?;
    Ok(hash.to_string())
}

/// Verifies a plaintext password against a stored Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::InternalError(format!("Invalid password hash format: {}", e)))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ---------------------------------------------------------------------------
// Password policy
// ---------------------------------------------------------------------------

/// Validates that a password meets the platform policy:
/// - minimum 8 characters
/// - at least one uppercase letter
/// - at least one lowercase letter
/// - at least one digit
/// - at least one special character
pub fn validate_password_policy(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::ValidationError(
            "Password must be at least 8 characters long".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one lowercase letter".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one digit".to_string(),
        ));
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(AppError::ValidationError(
            "Password must contain at least one special character".to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// JWT generation
// ---------------------------------------------------------------------------

/// Generates a short-lived access token (JWT) with role-based claims.
pub fn generate_access_token(
    user_id: Uuid,
    role: &str,
    secret: &str,
    expiry_minutes: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = TokenClaims {
        sub: user_id,
        role: role.to_string(),
        typ: "access".to_string(),
        exp: (now + Duration::minutes(expiry_minutes)).timestamp(),
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("Token generation failed: {}", e)))?;
    log::info!("Generated access token for user_id={}", user_id);
    Ok(token)
}

/// Generates a longer-lived refresh token.
pub fn generate_refresh_token(
    user_id: Uuid,
    secret: &str,
    expiry_days: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = TokenClaims {
        sub: user_id,
        role: String::new(),
        typ: "refresh".to_string(),
        exp: (now + Duration::days(expiry_days)).timestamp(),
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalError(format!("Refresh token generation failed: {}", e)))?;
    log::info!("Generated refresh token for user_id={}", user_id);
    Ok(token)
}

// ---------------------------------------------------------------------------
// JWT validation
// ---------------------------------------------------------------------------

/// Validates a JWT and returns the decoded claims.
pub fn validate_token(token: &str, secret: &str) -> Result<TokenClaims, AppError> {
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

// ---------------------------------------------------------------------------
// Account lockout
// ---------------------------------------------------------------------------

/// Returns `true` if the account should be locked out (10+ failed attempts
/// within the last 15 minutes).
pub async fn check_account_lockout(
    pool: &PgPool,
    username: &str,
) -> Result<bool, AppError> {
    let window = Utc::now() - Duration::minutes(15);
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM login_attempts \
         WHERE username = $1 AND success = FALSE AND attempted_at > $2",
    )
    .bind(username)
    .bind(window)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check lockout: {}", e)))?;

    Ok(count >= 10)
}

/// Records a login attempt (successful or failed) for audit and lockout tracking.
pub async fn record_login_attempt(
    pool: &PgPool,
    username: &str,
    ip: &str,
    success: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO login_attempts (id, username, ip_address, attempted_at, success) \
         VALUES ($1, $2, $3, NOW(), $4)",
    )
    .bind(Uuid::new_v4())
    .bind(username)
    .bind(ip)
    .bind(success)
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to record login attempt: {}", e)))?;

    log::info!(
        "Login attempt recorded: username={}, ip=[REDACTED], success={}",
        username,
        success
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Token revocation
// ---------------------------------------------------------------------------

/// Checks whether a token (identified by its JTI) has been revoked.
pub async fn is_token_revoked(pool: &PgPool, jti: &str) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM revoked_tokens WHERE token_jti = $1)",
    )
    .bind(jti)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check token revocation: {}", e)))?;

    Ok(exists)
}

/// Revokes a token by inserting its JTI into the revoked_tokens table.
pub async fn revoke_token(
    pool: &PgPool,
    jti: &str,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO revoked_tokens (id, token_jti, user_id, revoked_at, expires_at) \
         VALUES ($1, $2, $3, NOW(), $4) ON CONFLICT (token_jti) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(jti)
    .bind(user_id)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to revoke token: {}", e)))?;

    log::info!("Token revoked: jti=[REDACTED], user_id={}", user_id);
    Ok(())
}
