use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("", web::get().to(generate_report)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ReportQuery {
    /// Accepts a date string (YYYY-MM-DD). Defaults to 30 days ago if omitted.
    start_date: Option<chrono::NaiveDate>,
    /// Accepts a date string (YYYY-MM-DD). Defaults to today if omitted.
    end_date: Option<chrono::NaiveDate>,
    #[serde(rename = "type")]
    report_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportResponse {
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
    report_type: String,
    orders: OrderStats,
    revenue: RevenueStats,
    users: UserStats,
    ratings: RatingStats,
}

#[derive(Debug, Serialize)]
struct OrderStats {
    total: i64,
    by_status: Vec<StatusCount>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
struct StatusCount {
    status: String,
    count: i64,
}

#[derive(Debug, Serialize)]
struct RevenueStats {
    total_revenue: f64,
    total_discount: f64,
    net_revenue: f64,
    average_order_value: f64,
}

#[derive(Debug, Serialize)]
struct UserStats {
    total_users: i64,
    new_users_in_period: i64,
    active_shoppers: i64,
}

#[derive(Debug, Serialize)]
struct RatingStats {
    total_ratings: i64,
    new_ratings_in_period: i64,
    average_score: Option<f64>,
}

// ---------------------------------------------------------------------------
// GET /api/reports
// ---------------------------------------------------------------------------

async fn generate_report(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    query: web::Query<ReportQuery>,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let q = query.into_inner();
    let report_type = q.report_type.unwrap_or_else(|| "summary".to_string());

    if !["summary", "detailed"].contains(&report_type.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid report type '{}'. Must be 'summary' or 'detailed'.",
            report_type
        )));
    }

    let today = chrono::Utc::now().date_naive();
    let default_start = today - chrono::Duration::days(30);
    let start_date = q.start_date.unwrap_or(default_start);
    let end_date = q.end_date.unwrap_or(today);
    let start_dt = start_date.and_hms_opt(0, 0, 0).unwrap();
    let end_dt = end_date.and_hms_opt(23, 59, 59).unwrap();
    let start = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(start_dt, chrono::Utc);
    let end = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(end_dt, chrono::Utc);

    // Order stats
    let total_orders = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM orders WHERE created_at >= $1 AND created_at <= $2",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count orders: {}", e)))?;

    let by_status = sqlx::query_as::<_, StatusCount>(
        "SELECT status, COUNT(*)::bigint AS count FROM orders \
         WHERE created_at >= $1 AND created_at <= $2 \
         GROUP BY status ORDER BY count DESC",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch order stats: {}", e)))?;

    // Revenue stats
    let revenue_row = sqlx::query_as::<_, RevenueRow>(
        "SELECT COALESCE(SUM(total_amount::float8), 0) AS total_revenue, \
         COALESCE(SUM(discount_amount::float8), 0) AS total_discount \
         FROM orders \
         WHERE created_at >= $1 AND created_at <= $2 \
         AND status NOT IN ('Cancelled', 'Refunded')",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch revenue: {}", e)))?;

    let paid_orders = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM orders \
         WHERE created_at >= $1 AND created_at <= $2 \
         AND status NOT IN ('Cancelled', 'Refunded', 'Reserved', 'Created')",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count paid orders: {}", e)))?;

    let average_order_value = if paid_orders > 0 {
        revenue_row.total_revenue / paid_orders as f64
    } else {
        0.0
    };

    // User stats
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to count users: {}", e)))?;

    let new_users = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE created_at >= $1 AND created_at <= $2",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count new users: {}", e)))?;

    let active_shoppers = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM orders \
         WHERE created_at >= $1 AND created_at <= $2",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count active shoppers: {}", e)))?;

    // Rating stats
    let total_ratings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ratings",
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count ratings: {}", e)))?;

    let new_ratings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ratings WHERE created_at >= $1 AND created_at <= $2",
    )
    .bind(start)
    .bind(end)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count new ratings: {}", e)))?;

    let avg_score = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(ps.average_score::float8) FROM product_scores ps \
         WHERE ps.average_score IS NOT NULL",
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch avg score: {}", e)))?;

    let response = ReportResponse {
        start_date: start_date,
        end_date: end_date,
        report_type,
        orders: OrderStats {
            total: total_orders,
            by_status,
        },
        revenue: RevenueStats {
            total_revenue: revenue_row.total_revenue,
            total_discount: revenue_row.total_discount,
            net_revenue: revenue_row.total_revenue - revenue_row.total_discount,
            average_order_value,
        },
        users: UserStats {
            total_users,
            new_users_in_period: new_users,
            active_shoppers,
        },
        ratings: RatingStats {
            total_ratings,
            new_ratings_in_period: new_ratings,
            average_score: avg_score,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, sqlx::FromRow)]
struct RevenueRow {
    total_revenue: f64,
    total_discount: f64,
}
