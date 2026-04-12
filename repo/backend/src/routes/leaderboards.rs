use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::common::PaginatedResponse;
use crate::models::rating::LeaderboardQuery;
use crate::services::rating_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/leaderboards")
            .route("", web::get().to(get_leaderboard)),
    );
}

// ---------------------------------------------------------------------------
// GET /api/leaderboards
// ---------------------------------------------------------------------------

async fn get_leaderboard(
    pool: web::Data<PgPool>,
    query: web::Query<LeaderboardQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let page = q.page();
    let per_page = q.per_page();

    // Validate period if provided
    if let Some(ref period) = q.period {
        if !["weekly", "monthly"].contains(&period.as_str()) {
            return Err(AppError::ValidationError(format!(
                "Invalid period '{}'. Must be 'weekly' or 'monthly'.",
                period
            )));
        }
    }

    // Get total count for pagination
    let period_filter = match q.period.as_deref() {
        Some("weekly") => "AND ps.last_rating_at >= NOW() - INTERVAL '7 days'",
        Some("monthly") => "AND ps.last_rating_at >= NOW() - INTERVAL '30 days'",
        _ => "",
    };

    let genre_filter = if q.genre.is_some() {
        "AND p.genre = $1"
    } else {
        ""
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM product_scores ps \
         JOIN products p ON p.id = ps.product_id \
         WHERE ps.average_score IS NOT NULL {} {}",
        period_filter, genre_filter
    );

    let total = if let Some(ref genre) = q.genre {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(genre)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count leaderboard: {}", e)))?
    } else {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count leaderboard: {}", e)))?
    };

    let entries = rating_service::get_leaderboard(pool.get_ref(), q).await?;

    let response = PaginatedResponse::new(entries, total, page, per_page);
    Ok(HttpResponse::Ok().json(response))
}
