use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_owner_or_admin;
use crate::models::common::PaginatedResponse;
use crate::models::rating::{
    CreateRatingRequest, DimensionScore, Rating, RatingDimension, RatingResponse,
};
use crate::services::audit_service;
use crate::services::rating_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ratings")
            .route("", web::post().to(create_rating))
            .route("/product/{id}", web::get().to(get_product_ratings))
            .route("/{id}", web::get().to(get_rating))
            .route("/{id}", web::put().to(update_rating))
            .route("/{id}", web::delete().to(delete_rating)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RatingListQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

impl RatingListQuery {
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
struct UpdateRatingRequest {
    dimensions: Vec<DimensionScore>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn build_rating_response(
    pool: &PgPool,
    rating: &Rating,
) -> Result<RatingResponse, AppError> {
    let dims = sqlx::query_as::<_, RatingDimension>(
        "SELECT * FROM rating_dimensions WHERE rating_id = $1 ORDER BY dimension_name",
    )
    .bind(rating.id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch dimensions: {}", e)))?;

    let dimensions: Vec<DimensionScore> = dims
        .into_iter()
        .map(|d| DimensionScore {
            dimension_name: d.dimension_name,
            score: d.score,
        })
        .collect();

    let avg = if dimensions.is_empty() {
        0.0
    } else {
        dimensions.iter().map(|d| d.score as f64).sum::<f64>() / dimensions.len() as f64
    };

    Ok(RatingResponse {
        id: rating.id,
        user_id: rating.user_id,
        product_id: rating.product_id,
        dimensions,
        average: avg,
        moderation_status: rating.moderation_status.clone(),
        created_at: rating.created_at,
        updated_at: rating.updated_at,
    })
}

// ---------------------------------------------------------------------------
// POST /api/ratings
// ---------------------------------------------------------------------------

async fn create_rating(
    http_req: HttpRequest,
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateRatingRequest>,
) -> Result<HttpResponse, AppError> {
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.dimensions.is_empty() {
        return Err(AppError::ValidationError(
            "At least one rating dimension is required".to_string(),
        ));
    }

    // Validate dimension scores (1-10)
    for dim in &req.dimensions {
        if dim.score < 1 || dim.score > 10 {
            return Err(AppError::ValidationError(format!(
                "Score for '{}' must be between 1 and 10, got {}",
                dim.dimension_name, dim.score
            )));
        }
    }

    // Check eligibility
    rating_service::check_eligibility(pool.get_ref(), user.user_id, req.product_id).await?;

    // Check for existing rating
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ratings WHERE user_id = $1 AND product_id = $2)",
    )
    .bind(user.user_id)
    .bind(req.product_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check existing rating: {}", e)))?;

    if existing {
        return Err(AppError::Conflict(
            "You have already rated this product. Use PUT to update.".to_string(),
        ));
    }

    let rating_id = Uuid::new_v4();

    let mut tx = pool
        .get_ref()
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Create rating
    let rating = sqlx::query_as::<_, Rating>(
        "INSERT INTO ratings (id, user_id, product_id, moderation_status, created_at, updated_at) \
         VALUES ($1, $2, $3, 'Pending', NOW(), NOW()) RETURNING *",
    )
    .bind(rating_id)
    .bind(user.user_id)
    .bind(req.product_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create rating: {}", e)))?;

    // Insert dimensions
    for dim in &req.dimensions {
        sqlx::query(
            "INSERT INTO rating_dimensions (id, rating_id, dimension_name, score) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(rating_id)
        .bind(&dim.dimension_name)
        .bind(dim.score)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to insert dimension: {}", e)))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    // Update aggregate score
    rating_service::update_product_score(pool.get_ref(), req.product_id).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "rating.create",
        Some(&ip),
        "rating",
        &rating_id.to_string(),
        Some(serde_json::json!({
            "product_id": req.product_id,
            "dimension_count": req.dimensions.len()
        })),
    )
    .await?;

    let response = build_rating_response(pool.get_ref(), &rating).await?;
    Ok(HttpResponse::Created().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/ratings/product/{id}
// ---------------------------------------------------------------------------

async fn get_product_ratings(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    query: web::Query<RatingListQuery>,
) -> Result<HttpResponse, AppError> {
    let product_id = path.into_inner();
    let q = query.into_inner();
    let page = q.page();
    let per_page = q.per_page();
    let offset = q.offset();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ratings WHERE product_id = $1",
    )
    .bind(product_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count ratings: {}", e)))?;

    let ratings = sqlx::query_as::<_, Rating>(
        "SELECT * FROM ratings WHERE product_id = $1 \
         ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(product_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch ratings: {}", e)))?;

    let mut responses = Vec::new();
    for rating in &ratings {
        let resp = build_rating_response(pool.get_ref(), rating).await?;
        responses.push(resp);
    }

    let paginated = PaginatedResponse::new(responses, total, page, per_page);
    Ok(HttpResponse::Ok().json(paginated))
}

// ---------------------------------------------------------------------------
// GET /api/ratings/{id}
// ---------------------------------------------------------------------------

async fn get_rating(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let rating_id = path.into_inner();

    let rating = sqlx::query_as::<_, Rating>(
        "SELECT * FROM ratings WHERE id = $1",
    )
    .bind(rating_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Rating not found".to_string()))?;

    let response = build_rating_response(pool.get_ref(), &rating).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// PUT /api/ratings/{id}
// ---------------------------------------------------------------------------

async fn update_rating(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateRatingRequest>,
) -> Result<HttpResponse, AppError> {
    let rating_id = path.into_inner();
    let req = body.into_inner();

    let rating = sqlx::query_as::<_, Rating>(
        "SELECT * FROM ratings WHERE id = $1",
    )
    .bind(rating_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Rating not found".to_string()))?;

    // Only the owner can update
    require_owner_or_admin(&user, rating.user_id)?;

    // Validate scores
    for dim in &req.dimensions {
        if dim.score < 1 || dim.score > 10 {
            return Err(AppError::ValidationError(format!(
                "Score for '{}' must be between 1 and 10, got {}",
                dim.dimension_name, dim.score
            )));
        }
    }

    let mut tx = pool
        .get_ref()
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Remove old dimensions
    sqlx::query("DELETE FROM rating_dimensions WHERE rating_id = $1")
        .bind(rating_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete dimensions: {}", e)))?;

    // Insert new dimensions
    for dim in &req.dimensions {
        sqlx::query(
            "INSERT INTO rating_dimensions (id, rating_id, dimension_name, score) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(rating_id)
        .bind(&dim.dimension_name)
        .bind(dim.score)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to insert dimension: {}", e)))?;
    }

    // Update rating timestamp
    let updated = sqlx::query_as::<_, Rating>(
        "UPDATE ratings SET updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(rating_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to update rating: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    // Update aggregate score
    rating_service::update_product_score(pool.get_ref(), rating.product_id).await?;

    let response = build_rating_response(pool.get_ref(), &updated).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// DELETE /api/ratings/{id}
// ---------------------------------------------------------------------------

async fn delete_rating(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let rating_id = path.into_inner();

    let rating = sqlx::query_as::<_, Rating>(
        "SELECT * FROM ratings WHERE id = $1",
    )
    .bind(rating_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Rating not found".to_string()))?;

    // Owner or admin
    require_owner_or_admin(&user, rating.user_id)?;

    let mut tx = pool
        .get_ref()
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    sqlx::query("DELETE FROM rating_dimensions WHERE rating_id = $1")
        .bind(rating_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete dimensions: {}", e)))?;

    sqlx::query("DELETE FROM ratings WHERE id = $1")
        .bind(rating_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete rating: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    // Update aggregate score
    rating_service::update_product_score(pool.get_ref(), rating.product_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Rating deleted"})))
}
