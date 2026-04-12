use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Bulk order risk check
// ---------------------------------------------------------------------------

/// Checks if a user is placing suspiciously many items across recent orders.
///
/// If the total item quantity across orders placed within the configured
/// window exceeds the threshold, a risk event is recorded and the request
/// is blocked.
pub async fn check_bulk_order_risk(
    pool: &PgPool,
    user_id: Uuid,
    new_quantity: u32,
    config: &Config,
) -> Result<(), AppError> {
    let window_start =
        Utc::now() - Duration::minutes(config.risk_bulk_order_window_minutes as i64);

    let recent_quantity = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(oi.quantity), 0) \
         FROM order_items oi \
         JOIN orders o ON o.id = oi.order_id \
         WHERE o.user_id = $1 AND o.created_at > $2",
    )
    .bind(user_id)
    .bind(window_start)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check bulk order risk: {}", e)))?
    .unwrap_or(0);

    let total = recent_quantity as u32 + new_quantity;

    if total > config.risk_bulk_order_threshold {
        // Record risk event
        sqlx::query(
            "INSERT INTO risk_events (id, user_id, event_type, details, status, created_at) \
             VALUES ($1, $2, 'BulkOrder', $3, 'Flagged', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(serde_json::json!({
            "recent_quantity": recent_quantity,
            "new_quantity": new_quantity,
            "threshold": config.risk_bulk_order_threshold,
            "window_minutes": config.risk_bulk_order_window_minutes
        }))
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to record risk event: {}", e))
        })?;

        return Err(AppError::Forbidden(format!(
            "Order flagged for risk review: {} total items exceeds threshold of {} within {} minutes",
            total, config.risk_bulk_order_threshold, config.risk_bulk_order_window_minutes
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Discount abuse risk check
// ---------------------------------------------------------------------------

/// Checks if a user is abusing discount codes by applying too many within
/// the configured window.
pub async fn check_discount_abuse_risk(
    pool: &PgPool,
    user_id: Uuid,
    config: &Config,
) -> Result<(), AppError> {
    let window_start =
        Utc::now() - Duration::minutes(config.risk_discount_abuse_window_minutes as i64);

    let discount_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM orders \
         WHERE user_id = $1 AND created_at > $2 AND discount_amount > 0",
    )
    .bind(user_id)
    .bind(window_start)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to check discount abuse: {}", e))
    })?;

    if discount_count >= config.risk_discount_abuse_threshold as i64 {
        // Record risk event
        sqlx::query(
            "INSERT INTO risk_events (id, user_id, event_type, details, status, created_at) \
             VALUES ($1, $2, 'DiscountAbuse', $3, 'Flagged', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(serde_json::json!({
            "discount_count": discount_count,
            "threshold": config.risk_discount_abuse_threshold,
            "window_minutes": config.risk_discount_abuse_window_minutes
        }))
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to record risk event: {}", e))
        })?;

        return Err(AppError::Forbidden(format!(
            "Order flagged for discount abuse: {} discounted orders exceeds threshold of {} within {} minutes",
            discount_count,
            config.risk_discount_abuse_threshold,
            config.risk_discount_abuse_window_minutes
        )));
    }

    Ok(())
}
