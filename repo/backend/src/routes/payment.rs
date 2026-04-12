use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::payment::SimulatePaymentRequest;
use crate::services::payment_simulator;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/payment")
            .route("/simulate", web::post().to(simulate_payment)),
    );
}

// ---------------------------------------------------------------------------
// POST /api/payment/simulate
// ---------------------------------------------------------------------------

/// Simulate a payment (mock service).
///
/// Idempotency is handled by the payment simulator service using the
/// combination of `order_id` and `attempt_number` to form an idempotency key.
/// If a payment event with the same key already exists, the previous result
/// is returned without creating a duplicate.
async fn simulate_payment(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<SimulatePaymentRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();

    // Validate inputs
    if req.amount <= 0.0 {
        return Err(AppError::ValidationError(
            "Payment amount must be greater than zero".to_string(),
        ));
    }

    if req.attempt_number < 1 {
        return Err(AppError::ValidationError(
            "Attempt number must be at least 1".to_string(),
        ));
    }

    // Verify the order exists AND belongs to the authenticated user
    let order_owner = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM orders WHERE id = $1",
    )
    .bind(req.order_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check order: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("Order {} not found", req.order_id)))?;

    if order_owner != user.user_id && user.role != "Admin" {
        return Err(AppError::Forbidden(
            "You can only simulate payments for your own orders".to_string(),
        ));
    }

    let event = payment_simulator::simulate_payment(pool.get_ref(), req).await?;

    Ok(HttpResponse::Ok().json(event))
}
