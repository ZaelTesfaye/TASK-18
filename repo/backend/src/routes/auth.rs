use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rate_limit::{self, RateLimiter};
use crate::models::user::{CreateUserRequest, LoginRequest, User, UserResponse};
use crate::services::audit_service;
use crate::services::auth_service;
use crate::services::encryption_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh))
            .route("/logout", web::post().to(logout))
            .route("/reset-password", web::post().to(reset_password_with_token)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AuthTokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
}

#[derive(Debug, Serialize)]
struct AccessTokenResponse {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LogoutRequest {
    refresh_token: String,
}

// ---------------------------------------------------------------------------
// POST /api/auth/register
// ---------------------------------------------------------------------------

async fn register(
    pool: web::Data<PgPool>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();

    // Validate inputs
    if req.username.trim().is_empty() {
        return Err(AppError::ValidationError("Username is required".to_string()));
    }
    if req.email.trim().is_empty() {
        return Err(AppError::ValidationError("Email is required".to_string()));
    }

    // Validate password policy
    auth_service::validate_password_policy(&req.password)?;

    // Hash password
    let password_hash = auth_service::hash_password(&req.password)?;

    let user_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Insert user with Shopper role
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, email, password_hash, role, verified_possession, \
         is_locked, failed_login_attempts, legal_hold, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 'Shopper', FALSE, FALSE, 0, FALSE, $5, $5) \
         RETURNING *",
    )
    .bind(user_id)
    .bind(&req.username)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;

    let response = UserResponse::from_user(&user, None, None);

    Ok(HttpResponse::Created().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/auth/login
// ---------------------------------------------------------------------------

async fn login(
    pool: web::Data<PgPool>,
    limiter: web::Data<RateLimiter>,
    body: web::Json<LoginRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let login_req = body.into_inner();
    let config = Config::get();

    let ip = req
        .connection_info()
        .peer_addr()
        .unwrap_or("unknown")
        .to_string();

    // Check rate limits
    rate_limit::check_login_rate_limits(&limiter, &login_req.username, &ip, config)?;

    // Record the attempt in the rate limiter
    rate_limit::record_login_attempt(&limiter, &login_req.username, &ip);

    // Check account lockout
    let locked = auth_service::check_account_lockout(pool.get_ref(), &login_req.username).await?;
    if locked {
        auth_service::record_login_attempt(pool.get_ref(), &login_req.username, &ip, false).await?;
        return Err(AppError::Unauthorized(
            "Account is temporarily locked due to too many failed login attempts".to_string(),
        ));
    }

    // Fetch user by username
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username = $1",
    )
    .bind(&login_req.username)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Database error: {}", e)))?;

    let user = match user {
        Some(u) => u,
        None => {
            auth_service::record_login_attempt(pool.get_ref(), &login_req.username, &ip, false)
                .await?;
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }
    };

    // Check if account is explicitly locked
    if user.is_locked {
        auth_service::record_login_attempt(pool.get_ref(), &login_req.username, &ip, false).await?;
        return Err(AppError::Unauthorized("Account is locked".to_string()));
    }

    // Verify password
    let valid = auth_service::verify_password(&login_req.password, &user.password_hash)?;
    if !valid {
        auth_service::record_login_attempt(pool.get_ref(), &login_req.username, &ip, false).await?;
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // Record successful login
    auth_service::record_login_attempt(pool.get_ref(), &login_req.username, &ip, true).await?;

    // Generate tokens
    let access_token = auth_service::generate_access_token(
        user.id,
        &user.role,
        &config.jwt_secret,
        config.jwt_access_expiry_minutes,
    )?;

    let refresh_token = auth_service::generate_refresh_token(
        user.id,
        &config.jwt_secret,
        config.jwt_refresh_expiry_days,
    )?;

    Ok(HttpResponse::Ok().json(AuthTokenResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// POST /api/auth/refresh
// ---------------------------------------------------------------------------

async fn refresh(
    pool: web::Data<PgPool>,
    body: web::Json<RefreshRequest>,
) -> Result<HttpResponse, AppError> {
    let config = Config::get();
    let req = body.into_inner();

    // Validate the refresh token
    let claims = auth_service::validate_token(&req.refresh_token, &config.jwt_secret)?;

    // Enforce that only refresh-type tokens are accepted here
    if claims.typ != "refresh" {
        return Err(AppError::Unauthorized(
            "Only refresh tokens are accepted at this endpoint".to_string(),
        ));
    }

    // Check if token is revoked
    if auth_service::is_token_revoked(pool.get_ref(), &claims.jti).await? {
        return Err(AppError::Unauthorized("Refresh token has been revoked".to_string()));
    }

    // Fetch user to get current role
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(claims.sub)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Generate new access token
    let access_token = auth_service::generate_access_token(
        user.id,
        &user.role,
        &config.jwt_secret,
        config.jwt_access_expiry_minutes,
    )?;

    Ok(HttpResponse::Ok().json(AccessTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// POST /api/auth/logout
// ---------------------------------------------------------------------------

async fn logout(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<LogoutRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let config = Config::get();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    // Validate the refresh token to get its claims
    let refresh_claims = auth_service::validate_token(&req.refresh_token, &config.jwt_secret)?;

    // Verify the refresh token belongs to the authenticated user — prevents
    // cross-user session revocation where user B submits user A's token.
    if refresh_claims.typ != "refresh" {
        return Err(AppError::BadRequest(
            "Submitted token is not a refresh token".to_string(),
        ));
    }
    if refresh_claims.sub != user.user_id {
        return Err(AppError::Forbidden(
            "Cannot revoke another user's refresh token".to_string(),
        ));
    }

    // Compute expiry from claims
    let expires_at = chrono::DateTime::from_timestamp(refresh_claims.exp, 0)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(7));

    // Revoke the refresh token
    auth_service::revoke_token(pool.get_ref(), &refresh_claims.jti, refresh_claims.sub, expires_at).await?;

    // Also revoke the current access token so it can't be used for the rest of its window
    let access_expiry = chrono::Utc::now() + chrono::Duration::minutes(
        Config::get().jwt_access_expiry_minutes,
    );
    auth_service::revoke_token(pool.get_ref(), &user.jti, user.user_id, access_expiry).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "auth.logout",
        Some(&ip),
        "user",
        &user.user_id.to_string(),
        Some(serde_json::json!({
            "action": "tokens_revoked",
            "access_jti_revoked": user.jti,
            "refresh_jti_revoked": refresh_claims.jti
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Logged out successfully"})))
}

// ---------------------------------------------------------------------------
// POST /api/auth/reset-password  (public — token-based)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ResetPasswordWithTokenRequest {
    user_id: Uuid,
    token: String,
    new_password: String,
}

/// Allows a user to set a new password using a one-time reset token
/// issued by an admin via POST /api/admin/users/{id}/reset-password.
async fn reset_password_with_token(
    pool: web::Data<PgPool>,
    body: web::Json<ResetPasswordWithTokenRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    // Validate the new password meets policy
    auth_service::validate_password_policy(&req.new_password)?;

    // Fetch the user — the admin stored the hashed token in reset_token_hash
    // and the expiry in reset_token_expires_at (password_hash is untouched)
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(req.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Verify a reset token exists and hasn't expired
    let token_hash = user.reset_token_hash.as_deref().ok_or_else(|| {
        AppError::BadRequest("No reset token is active for this user.".to_string())
    })?;
    if let Some(expiry) = user.reset_token_expires_at {
        if chrono::Utc::now() > expiry {
            return Err(AppError::BadRequest(
                "Reset token has expired. Request a new one from an admin.".to_string(),
            ));
        }
    } else {
        return Err(AppError::BadRequest(
            "No reset token is active for this user.".to_string(),
        ));
    }

    // Verify the token matches the stored hash
    let token_valid = auth_service::verify_password(&req.token, token_hash)?;
    if !token_valid {
        return Err(AppError::Unauthorized("Invalid reset token.".to_string()));
    }

    // Set the new password and clear the reset token — the user can now log in
    // with the new password. Reset token is consumed (set to NULL).
    let new_hash = auth_service::hash_password(&req.new_password)?;
    sqlx::query(
        "UPDATE users SET password_hash = $1, reset_token_hash = NULL, \
         reset_token_expires_at = NULL, is_locked = FALSE, \
         failed_login_attempts = 0, updated_at = NOW() WHERE id = $2",
    )
    .bind(&new_hash)
    .bind(req.user_id)
    .execute(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &req.user_id.to_string(),
        "auth.reset_password_completed",
        Some(&ip),
        "user",
        &req.user_id.to_string(),
        Some(serde_json::json!({"action": "password_reset_with_token"})),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Password has been reset successfully. You can now log in with your new password."
    })))
}
