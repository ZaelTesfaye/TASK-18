use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Summary of a retention job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionResult {
    pub orders_archived: u32,
    pub auth_logs_deleted: u32,
}

// ---------------------------------------------------------------------------
// Retention job
// ---------------------------------------------------------------------------

/// Runs the data retention job:
///
/// 1. Archives orders older than `RETENTION_ORDERS_YEARS`, skipping those
///    with `legal_hold = true`.
/// 2. Deletes auth_logs older than `RETENTION_AUTH_LOGS_YEARS`.
/// 3. Audit-logs each action with `SYSTEM` as the actor.
pub async fn run_retention_job(
    pool: &PgPool,
    config: &Config,
) -> Result<RetentionResult, AppError> {
    let order_cutoff = Utc::now() - Duration::days(config.retention_orders_years as i64 * 365);
    let auth_cutoff = Utc::now() - Duration::days(config.retention_auth_logs_years as i64 * 365);

    log::info!(
        "Starting retention job: order_cutoff={}, auth_cutoff={}",
        order_cutoff,
        auth_cutoff
    );

    let mut orders_archived: u32 = 0;

    // Archive old orders (skip legal_hold)
    let old_orders: Vec<OrderIdRow> = sqlx::query_as(
        "SELECT id FROM orders \
         WHERE created_at < $1 AND legal_hold = FALSE \
         AND id NOT IN (SELECT id FROM archived_orders)",
    )
    .bind(order_cutoff)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch old orders: {}", e))
    })?;

    for row in &old_orders {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to begin transaction: {}", e))
            })?;

        // Serialize order data to JSON
        let order_json: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT row_to_json(o) FROM (SELECT * FROM orders WHERE id = $1) o",
        )
        .bind(row.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to export order: {}", e))
        })?;

        if let Some(data) = order_json {
            // Insert into archive
            sqlx::query(
                "INSERT INTO archived_orders (id, original_data, archived_at) \
                 VALUES ($1, $2, NOW()) ON CONFLICT (id) DO NOTHING",
            )
            .bind(row.id)
            .bind(&data)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to archive order: {}", e))
            })?;

            // Delete order items then order
            sqlx::query("DELETE FROM order_items WHERE order_id = $1")
                .bind(row.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to delete order items: {}", e))
                })?;

            sqlx::query("DELETE FROM orders WHERE id = $1")
                .bind(row.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to delete order: {}", e))
                })?;

            // Audit log
            sqlx::query(
                "INSERT INTO audit_log (id, actor, action, timestamp, target_type, target_id, change_summary) \
                 VALUES ($1, 'SYSTEM', 'retention.order_archived', NOW(), 'order', $2, $3)",
            )
            .bind(Uuid::new_v4())
            .bind(row.id.to_string())
            .bind(serde_json::json!({
                "reason": "Retention policy",
                "cutoff": order_cutoff.to_rfc3339()
            }))
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to write audit log: {}", e))
            })?;

            tx.commit()
                .await
                .map_err(|e| {
                    AppError::InternalError(format!("Failed to commit: {}", e))
                })?;

            orders_archived += 1;
        }
    }

    // Delete old auth logs
    let deleted = sqlx::query("DELETE FROM auth_logs WHERE created_at < $1")
        .bind(auth_cutoff)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to delete auth logs: {}", e))
        })?;

    let auth_logs_deleted = deleted.rows_affected() as u32;

    // Audit the auth log cleanup
    if auth_logs_deleted > 0 {
        sqlx::query(
            "INSERT INTO audit_log (id, actor, action, timestamp, target_type, target_id, change_summary) \
             VALUES ($1, 'SYSTEM', 'retention.auth_logs_deleted', NOW(), 'auth_logs', 'batch', $2)",
        )
        .bind(Uuid::new_v4())
        .bind(serde_json::json!({
            "count": auth_logs_deleted,
            "cutoff": auth_cutoff.to_rfc3339()
        }))
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to write audit log: {}", e))
        })?;
    }

    let result = RetentionResult {
        orders_archived,
        auth_logs_deleted,
    };

    log::info!(
        "Retention job complete: orders_archived={}, auth_logs_deleted={}",
        orders_archived,
        auth_logs_deleted
    );

    Ok(result)
}

#[derive(Debug, sqlx::FromRow)]
struct OrderIdRow {
    id: Uuid,
}

// ---------------------------------------------------------------------------
// Legal hold
// ---------------------------------------------------------------------------

/// Sets or clears the legal hold flag on an order, preventing it from
/// being archived by the retention job.
pub async fn set_legal_hold(
    pool: &PgPool,
    order_id: Uuid,
    hold: bool,
) -> Result<(), AppError> {
    let rows = sqlx::query(
        "UPDATE orders SET legal_hold = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(hold)
    .bind(order_id)
    .execute(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to set legal hold: {}", e))
    })?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Order not found".to_string()));
    }

    log::info!(
        "Legal hold updated: order_id={}, hold={}",
        order_id,
        hold
    );

    Ok(())
}
