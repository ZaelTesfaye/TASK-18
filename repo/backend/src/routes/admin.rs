use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::common::PaginatedResponse;
use crate::models::user::{UpdateRoleRequest, User, UserResponse};
use crate::services::audit_service;
use crate::services::auth_service;
use crate::services::retention_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("/users", web::get().to(list_users))
            .route("/users/{id}/role", web::put().to(change_role))
            .route(
                "/users/{id}/reset-password",
                web::post().to(reset_password),
            )
            .route("/users/{id}/unlock", web::post().to(unlock_account))
            .route("/risk-events", web::get().to(list_risk_events))
            .route("/risk-events/{id}", web::put().to(override_risk_event))
            .route(
                "/moderation/ratings/{id}",
                web::post().to(moderate_rating),
            )
            .route("/retention/run", web::post().to(run_retention))
            .route(
                "/retention/legal-hold/{order_id}",
                web::post().to(set_legal_hold),
            ),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UserListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

impl UserListQuery {
    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
    fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Debug, Deserialize)]
struct RiskEventQuery {
    status: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

impl RiskEventQuery {
    fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
    fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
    fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Debug, Deserialize)]
struct OverrideRiskEventRequest {
    status: String,
    justification: String,
}

#[derive(Debug, Deserialize)]
struct ModerateRatingRequest {
    status: String, // "Approved" or "Rejected"
}

#[derive(Debug, Deserialize)]
struct LegalHoldRequest {
    hold: bool,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct RiskEvent {
    id: Uuid,
    user_id: Uuid,
    event_type: String,
    details: Option<serde_json::Value>,
    status: String,
    override_justification: Option<String>,
    overridden_by: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_user_response(user: &User) -> UserResponse {
    UserResponse {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        phone_masked: user.phone_encrypted.as_deref().map(|_| "(XXX) XXX-XXXX".to_string()),
        address_masked: user.address_encrypted.as_deref().map(|_| "*****".to_string()),
        verified_possession: user.verified_possession,
        is_locked: user.is_locked,
        legal_hold: user.legal_hold,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}

// ---------------------------------------------------------------------------
// GET /api/admin/users
// ---------------------------------------------------------------------------

async fn list_users(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    query: web::Query<UserListQuery>,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let q = query.into_inner();
    let page = q.page();
    let per_page = q.per_page();
    let offset = q.offset();

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to count users: {}", e)))?;

    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch users: {}", e)))?;

    let items: Vec<UserResponse> = users.iter().map(build_user_response).collect();
    let paginated = PaginatedResponse::new(items, total, page, per_page);

    Ok(HttpResponse::Ok().json(paginated))
}

// ---------------------------------------------------------------------------
// PUT /api/admin/users/{id}/role
// ---------------------------------------------------------------------------

async fn change_role(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateRoleRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let target_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let valid_roles = ["Shopper", "Reviewer", "Admin"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid role '{}'. Must be one of: Shopper, Reviewer, Admin",
            req.role
        )));
    }

    let updated = sqlx::query_as::<_, User>(
        "UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2 RETURNING *",
    )
    .bind(&req.role)
    .bind(target_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.change_role",
        Some(&ip),
        "user",
        &target_id.to_string(),
        Some(serde_json::json!({"new_role": req.role})),
    )
    .await?;

    let response = build_user_response(&updated);
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/admin/users/{id}/reset-password
// ---------------------------------------------------------------------------

async fn reset_password(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let target_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();

    // Verify target user exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)",
    )
    .bind(target_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check user: {}", e)))?;

    if !exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    // Generate a one-time reset token — NO internal password is generated or set.
    // The user must call POST /api/auth/reset-password with this token to set a new password.
    let reset_token = Uuid::new_v4().to_string();
    let token_hash = auth_service::hash_password(&reset_token)?;

    // Store the hashed reset token in dedicated columns without modifying
    // password_hash — the user can still log in normally until the reset is
    // completed. Only when the user submits the token + new password does
    // password_hash get updated.
    let token_expiry = chrono::Utc::now() + chrono::Duration::hours(24);
    sqlx::query(
        "UPDATE users SET reset_token_hash = $1, reset_token_expires_at = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(&token_hash)
    .bind(token_expiry)
    .bind(target_id)
    .execute(pool.get_ref())
    .await?;

    // Audit log — token value is NOT logged
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.reset_password",
        Some(&ip),
        "user",
        &target_id.to_string(),
        Some(serde_json::json!({
            "action": "password_reset_token_issued",
            "expires_at": token_expiry.to_rfc3339()
        })),
    )
    .await?;

    // Return only the reset token. The admin delivers this through a secure channel.
    // The user then calls POST /api/auth/reset-password { "token": "...", "new_password": "..." }
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Reset token generated. Provide to user through a secure channel.",
        "reset_token": reset_token,
        "expires_at": token_expiry.to_rfc3339(),
        "note": "User must call POST /api/auth/reset-password with this token and a new password."
    })))
}

// ---------------------------------------------------------------------------
// POST /api/admin/users/{id}/unlock
// ---------------------------------------------------------------------------

async fn unlock_account(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let target_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();

    let rows = sqlx::query(
        "UPDATE users SET is_locked = FALSE, failed_login_attempts = 0, \
         locked_until = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(target_id)
    .execute(pool.get_ref())
    .await?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    // Also clear login attempts for this user
    let username = sqlx::query_scalar::<_, String>(
        "SELECT username FROM users WHERE id = $1",
    )
    .bind(target_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch username: {}", e)))?;

    if let Some(ref uname) = username {
        sqlx::query(
            "DELETE FROM login_attempts WHERE username = $1 AND success = FALSE",
        )
        .bind(uname)
        .execute(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to clear login attempts: {}", e)))?;
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.unlock_account",
        Some(&ip),
        "user",
        &target_id.to_string(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Account unlocked"})))
}

// ---------------------------------------------------------------------------
// GET /api/admin/risk-events
// ---------------------------------------------------------------------------

async fn list_risk_events(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    query: web::Query<RiskEventQuery>,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let q = query.into_inner();
    let page = q.page();
    let per_page = q.per_page();
    let offset = q.offset();

    let (count_sql, data_sql) = if q.status.is_some() {
        (
            "SELECT COUNT(*) FROM risk_events WHERE status = $1".to_string(),
            "SELECT * FROM risk_events WHERE status = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                .to_string(),
        )
    } else {
        (
            "SELECT COUNT(*) FROM risk_events".to_string(),
            "SELECT * FROM risk_events \
             ORDER BY created_at DESC LIMIT $1 OFFSET $2"
                .to_string(),
        )
    };

    let total = if let Some(ref status) = q.status {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(status)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count risk events: {}", e)))?
    } else {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count risk events: {}", e)))?
    };

    let events = if let Some(ref status) = q.status {
        sqlx::query_as::<_, RiskEvent>(&data_sql)
            .bind(status)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch risk events: {}", e)))?
    } else {
        sqlx::query_as::<_, RiskEvent>(&data_sql)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch risk events: {}", e)))?
    };

    let paginated = PaginatedResponse::new(events, total, page, per_page);
    Ok(HttpResponse::Ok().json(paginated))
}

// ---------------------------------------------------------------------------
// PUT /api/admin/risk-events/{id}
// ---------------------------------------------------------------------------

async fn override_risk_event(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<OverrideRiskEventRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let event_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.justification.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Justification is required to override a risk event".to_string(),
        ));
    }

    let valid_statuses = ["Approved", "Dismissed"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid status '{}'. Must be one of: Approved, Dismissed",
            req.status
        )));
    }

    let rows = sqlx::query(
        "UPDATE risk_events SET status = $1::risk_event_status, \
         override_justification = $2, overridden_by = $3, resolved_at = NOW() \
         WHERE id = $4",
    )
    .bind(&req.status)
    .bind(&req.justification)
    .bind(user.user_id)
    .bind(event_id)
    .execute(pool.get_ref())
    .await?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Risk event not found".to_string()));
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.risk_event_override",
        Some(&ip),
        "risk_event",
        &event_id.to_string(),
        Some(serde_json::json!({
            "new_status": req.status,
            "justification": req.justification
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Risk event updated",
        "status": req.status
    })))
}

// ---------------------------------------------------------------------------
// POST /api/admin/moderation/ratings/{id}
// ---------------------------------------------------------------------------

async fn moderate_rating(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ModerateRatingRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let rating_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let valid_statuses = ["Approved", "Rejected"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid moderation status '{}'. Must be 'Approved' or 'Rejected'",
            req.status
        )));
    }

    let rows = sqlx::query(
        "UPDATE ratings SET moderation_status = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&req.status)
    .bind(rating_id)
    .execute(pool.get_ref())
    .await?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Rating not found".to_string()));
    }

    // If rejected, update product score to exclude it
    let product_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT product_id FROM ratings WHERE id = $1",
    )
    .bind(rating_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch rating: {}", e)))?;

    crate::services::rating_service::update_product_score(pool.get_ref(), product_id).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.moderate_rating",
        Some(&ip),
        "rating",
        &rating_id.to_string(),
        Some(serde_json::json!({"moderation_status": req.status})),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Rating moderation updated",
        "moderation_status": req.status
    })))
}

// ---------------------------------------------------------------------------
// POST /api/admin/retention/run
// ---------------------------------------------------------------------------

async fn run_retention(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let config = Config::get();
    let result = retention_service::run_retention_job(pool.get_ref(), config).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.retention_run",
        Some(&ip),
        "system",
        "retention",
        Some(serde_json::json!({
            "orders_archived": result.orders_archived,
            "auth_logs_deleted": result.auth_logs_deleted
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(result))
}

// ---------------------------------------------------------------------------
// POST /api/admin/retention/legal-hold/{order_id}
// ---------------------------------------------------------------------------

async fn set_legal_hold(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<LegalHoldRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let order_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    retention_service::set_legal_hold(pool.get_ref(), order_id, req.hold).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "admin.legal_hold",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({"hold": req.hold})),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": if req.hold { "Legal hold set" } else { "Legal hold removed" },
        "order_id": order_id,
        "hold": req.hold
    })))
}
