use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::audit::{AuditLogEntry, AuditQuery};
use crate::models::common::PaginatedResponse;

// ---------------------------------------------------------------------------
// Log action
// ---------------------------------------------------------------------------

/// Inserts a new immutable entry into the audit log.
pub async fn log_action(
    pool: &PgPool,
    actor: &str,
    action: &str,
    ip: Option<&str>,
    target_type: &str,
    target_id: &str,
    change_summary: Option<serde_json::Value>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_log (id, actor, action, timestamp, ip_address, \
         target_type, target_id, change_summary) \
         VALUES ($1, $2, $3, NOW(), $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(actor)
    .bind(action)
    .bind(ip)
    .bind(target_type)
    .bind(target_id)
    .bind(&change_summary)
    .execute(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to write audit log: {}", e)))?;

    log::info!(
        "Audit: actor={}, action={}, target={}:{}",
        actor,
        action,
        target_type,
        target_id
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Query audit log
// ---------------------------------------------------------------------------

/// Queries the audit log with optional filters and pagination.
pub async fn query_audit_log(
    pool: &PgPool,
    query: AuditQuery,
) -> Result<PaginatedResponse<AuditLogEntry>, AppError> {
    let page = query.page();
    let per_page = query.per_page();
    let offset = query.offset();

    // Build dynamic WHERE clauses
    let mut conditions = Vec::new();
    let mut param_idx = 1u32;

    if query.actor.is_some() {
        conditions.push(format!("actor = ${}", param_idx));
        param_idx += 1;
    }
    if query.action.is_some() {
        conditions.push(format!("action = ${}", param_idx));
        param_idx += 1;
    }
    if query.target_type.is_some() {
        conditions.push(format!("target_type = ${}", param_idx));
        param_idx += 1;
    }
    if query.target_id.is_some() {
        conditions.push(format!("target_id = ${}", param_idx));
        param_idx += 1;
    }
    // Parse date strings flexibly (RFC3339 or bare YYYY-MM-DD)
    let parsed_from = query.parsed_from_date();
    let parsed_to = query.parsed_to_date();
    if parsed_from.is_some() {
        conditions.push(format!("timestamp >= ${}", param_idx));
        param_idx += 1;
    }
    if parsed_to.is_some() {
        conditions.push(format!("timestamp <= ${}", param_idx));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM audit_log {}", where_clause);
    let data_sql = format!(
        "SELECT * FROM audit_log {} ORDER BY timestamp DESC LIMIT ${} OFFSET ${}",
        where_clause,
        param_idx,
        param_idx + 1
    );

    let total = build_count_query(&count_sql, &query, parsed_from, parsed_to, pool).await?;
    let items = build_data_query(&data_sql, &query, parsed_from, parsed_to, per_page, offset, pool).await?;

    Ok(PaginatedResponse::new(items, total, page, per_page))
}

async fn build_count_query(
    sql: &str,
    query: &AuditQuery,
    parsed_from: Option<chrono::DateTime<chrono::Utc>>,
    parsed_to: Option<chrono::DateTime<chrono::Utc>>,
    pool: &PgPool,
) -> Result<i64, AppError> {
    let mut q = sqlx::query_scalar::<_, i64>(sql);

    if let Some(ref actor) = query.actor {
        q = q.bind(actor);
    }
    if let Some(ref action) = query.action {
        q = q.bind(action);
    }
    if let Some(ref target_type) = query.target_type {
        q = q.bind(target_type);
    }
    if let Some(ref target_id) = query.target_id {
        q = q.bind(target_id);
    }
    if let Some(from) = parsed_from {
        q = q.bind(from);
    }
    if let Some(to) = parsed_to {
        q = q.bind(to);
    }

    q.fetch_one(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to count audit logs: {}", e)))
}

async fn build_data_query(
    sql: &str,
    query: &AuditQuery,
    parsed_from: Option<chrono::DateTime<chrono::Utc>>,
    parsed_to: Option<chrono::DateTime<chrono::Utc>>,
    per_page: i64,
    offset: i64,
    pool: &PgPool,
) -> Result<Vec<AuditLogEntry>, AppError> {
    let mut q = sqlx::query_as::<_, AuditLogEntry>(sql);

    if let Some(ref actor) = query.actor {
        q = q.bind(actor);
    }
    if let Some(ref action) = query.action {
        q = q.bind(action);
    }
    if let Some(ref target_type) = query.target_type {
        q = q.bind(target_type);
    }
    if let Some(ref target_id) = query.target_id {
        q = q.bind(target_id);
    }
    if let Some(from) = parsed_from {
        q = q.bind(from);
    }
    if let Some(to) = parsed_to {
        q = q.bind(to);
    }

    q = q.bind(per_page).bind(offset);

    q.fetch_all(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to query audit logs: {}", e)))
}
