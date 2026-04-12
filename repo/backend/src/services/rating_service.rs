use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::rating::{LeaderboardEntry, LeaderboardQuery, ProductScore};

// ---------------------------------------------------------------------------
// Eligibility check
// ---------------------------------------------------------------------------

/// Verifies that a user is eligible to rate a product.
///
/// Eligibility requires either:
/// - A Delivered or Completed order containing the product, **or**
/// - `verified_possession = true` on the user record.
pub async fn check_eligibility(
    pool: &PgPool,
    user_id: Uuid,
    product_id: Uuid,
) -> Result<(), AppError> {
    // Check verified_possession flag
    let verified = sqlx::query_scalar::<_, bool>(
        "SELECT verified_possession FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check user: {}", e)))?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if verified {
        return Ok(());
    }

    // Check for a qualifying order
    let has_order = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS( \
            SELECT 1 FROM orders o \
            JOIN order_items oi ON oi.order_id = o.id \
            WHERE o.user_id = $1 \
              AND oi.product_id = $2 \
              AND o.status IN ('Delivered', 'Completed') \
         )",
    )
    .bind(user_id)
    .bind(product_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check eligibility: {}", e)))?;

    if !has_order {
        return Err(AppError::Forbidden(
            "You must have a delivered or completed order for this product, \
             or have verified possession"
                .to_string(),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Aggregate calculation
// ---------------------------------------------------------------------------

/// Calculates the aggregate score for a product.
///
/// Algorithm: for each non-rejected rating, compute the average of its
/// dimension scores. Then average those per-rating averages across all
/// qualifying ratings, rounded to 2 decimal places.
pub async fn calculate_aggregate(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<ProductScore, AppError> {
    let row = sqlx::query_as::<_, AggregateRow>(
        "SELECT \
            ROUND(AVG(per_rating_avg)::numeric, 2)::float8 AS average_score, \
            COUNT(*)::int4 AS total_ratings, \
            MAX(created_at) AS last_rating_at \
         FROM ( \
            SELECT r.id, r.created_at, AVG(rd.score)::float8 AS per_rating_avg \
            FROM ratings r \
            JOIN rating_dimensions rd ON rd.rating_id = r.id \
            WHERE r.product_id = $1 \
              AND r.moderation_status != 'Rejected' \
            GROUP BY r.id, r.created_at \
         ) sub",
    )
    .bind(product_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to calculate aggregate: {}", e))
    })?;

    Ok(ProductScore {
        product_id,
        average_score: row.average_score,
        total_ratings: row.total_ratings.unwrap_or(0),
        last_rating_at: row.last_rating_at,
        updated_at: chrono::Utc::now(),
    })
}

#[derive(Debug, sqlx::FromRow)]
struct AggregateRow {
    average_score: Option<f64>,
    total_ratings: Option<i32>,
    last_rating_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// Update product score
// ---------------------------------------------------------------------------

/// Recalculates the aggregate and upserts it into the `product_scores` table.
pub async fn update_product_score(
    pool: &PgPool,
    product_id: Uuid,
) -> Result<(), AppError> {
    let score = calculate_aggregate(pool, product_id).await?;

    sqlx::query(
        "INSERT INTO product_scores (product_id, average_score, total_ratings, last_rating_at, updated_at) \
         VALUES ($1, $2, $3, $4, NOW()) \
         ON CONFLICT (product_id) DO UPDATE SET \
            average_score = EXCLUDED.average_score, \
            total_ratings = EXCLUDED.total_ratings, \
            last_rating_at = EXCLUDED.last_rating_at, \
            updated_at = NOW()",
    )
    .bind(product_id)
    .bind(score.average_score)
    .bind(score.total_ratings)
    .bind(score.last_rating_at)
    .execute(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to upsert product score: {}", e))
    })?;

    log::info!(
        "Product score updated: product_id={}, avg={:?}, count={}",
        product_id,
        score.average_score,
        score.total_ratings
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Leaderboard
// ---------------------------------------------------------------------------

/// Returns a leaderboard of products ranked by average score.
///
/// Supports optional filtering by period (weekly/monthly) and genre.
/// Tie-break: `total_ratings DESC`, then `last_rating_at DESC`.
pub async fn get_leaderboard(
    pool: &PgPool,
    query: LeaderboardQuery,
) -> Result<Vec<LeaderboardEntry>, AppError> {
    let limit = query.per_page();
    let offset = query.offset();

    // Build the period filter
    let period_filter = match query.period.as_deref() {
        Some("weekly") => "AND ps.last_rating_at >= NOW() - INTERVAL '7 days'",
        Some("monthly") => "AND ps.last_rating_at >= NOW() - INTERVAL '30 days'",
        _ => "",
    };

    let genre_filter = if query.genre.is_some() {
        "AND p.genre = $3"
    } else {
        ""
    };

    let sql = format!(
        "SELECT p.id AS product_id, p.title AS product_title, \
                ps.average_score::float8 AS average_score, \
                ps.total_ratings, p.genre \
         FROM product_scores ps \
         JOIN products p ON p.id = ps.product_id \
         WHERE ps.average_score IS NOT NULL \
           {} {} \
         ORDER BY ps.average_score DESC, ps.total_ratings DESC, ps.last_rating_at DESC NULLS LAST \
         LIMIT $1 OFFSET $2",
        period_filter, genre_filter
    );

    let entries = if query.genre.is_some() {
        sqlx::query_as::<_, LeaderboardEntry>(&sql)
            .bind(limit)
            .bind(offset)
            .bind(query.genre.as_deref().unwrap_or(""))
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, LeaderboardEntry>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
    }
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch leaderboard: {}", e))
    })?;

    Ok(entries)
}
