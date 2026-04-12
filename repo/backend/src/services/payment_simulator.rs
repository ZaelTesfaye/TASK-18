// Mocking Payment Gateway response for audit stability -- no external processor contacted

use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::payment::{PaymentEvent, SimulatePaymentRequest};

/// Tolerance for floating-point amount comparison (1 cent).
const AMOUNT_TOLERANCE: f64 = 0.01;

// ---------------------------------------------------------------------------
// Simulate payment
// ---------------------------------------------------------------------------

/// Simulates a payment gateway interaction locally.
///
/// Generates an idempotency key of the form `"{order_id}:{attempt_number}"`.
/// If a payment event with that key already exists, the existing result is
/// returned (replay semantics). Otherwise a new `PaymentEvent` is created
/// with the requested outcome.
///
/// On `Success`: transitions the order from Reserved -> Paid.
/// On `Failed` / `Timeout`: records the event without changing order state.
pub async fn simulate_payment(
    pool: &PgPool,
    request: SimulatePaymentRequest,
) -> Result<PaymentEvent, AppError> {
    let idempotency_key = format!("{}:{}", request.order_id, request.attempt_number);

    // Check idempotency -- return existing event if present
    let existing = sqlx::query_as::<_, PaymentEvent>(
        "SELECT * FROM payment_events WHERE idempotency_key = $1",
    )
    .bind(&idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to check idempotency: {}", e))
    })?;

    if let Some(event) = existing {
        log::info!(
            "Payment replay: idempotency_key={}, status={}",
            idempotency_key,
            event.status
        );
        return Ok(event);
    }

    // Validate amount matches order total
    let order_total = sqlx::query_scalar::<_, f64>(
        "SELECT total_amount::float8 FROM orders WHERE id = $1",
    )
    .bind(request.order_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch order total: {}", e)))?
    .ok_or_else(|| AppError::NotFound(format!("Order {} not found", request.order_id)))?;

    if (request.amount - order_total).abs() > AMOUNT_TOLERANCE {
        // Log a risk event for the amount discrepancy
        sqlx::query(
            "INSERT INTO risk_events (id, user_id, event_type, details, status, created_at) \
             VALUES ($1, \
                     (SELECT user_id FROM orders WHERE id = $2), \
                     'BulkOrder', $3, 'Flagged', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(request.order_id)
        .bind(serde_json::json!({
            "reason": "payment_amount_mismatch",
            "submitted_amount": request.amount,
            "order_total": order_total,
            "order_id": request.order_id
        }))
        .execute(pool)
        .await
        .ok(); // best-effort risk logging

        return Err(AppError::ValidationError(format!(
            "Payment amount {:.2} does not match order total {:.2}",
            request.amount, order_total
        )));
    }

    // Validate outcome
    let status = match request.outcome.as_str() {
        "Success" => "Success",
        "Failed" => "Failed",
        "Timeout" => "Timeout",
        _ => {
            return Err(AppError::ValidationError(format!(
                "Invalid outcome: {}. Must be Success, Failed, or Timeout",
                request.outcome
            )));
        }
    };

    let payment_method = request
        .payment_method
        .as_deref()
        .unwrap_or("local_tender");

    let response_data = serde_json::json!({
        "simulator": true,
        "outcome": status,
        "message": format!("Simulated {} payment", status),
    });

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // For success payments, verify the order is in Reserved status BEFORE
    // inserting the payment event. This prevents recording a success event
    // when the order cannot actually transition to Paid.
    if status == "Success" {
        let current_status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM orders WHERE id = $1 FOR UPDATE",
        )
        .bind(request.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to lock order: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("Order {} not found", request.order_id)))?;

        if current_status != "Reserved" {
            return Err(AppError::BadRequest(format!(
                "Cannot process payment: order status is '{}', expected 'Reserved'",
                current_status
            )));
        }
    }

    let event = sqlx::query_as::<_, PaymentEvent>(
        "INSERT INTO payment_events (id, order_id, idempotency_key, amount, status, \
         payment_method, response_data, created_at) \
         VALUES ($1, $2, $3, $4, $5::payment_status, $6, $7, NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(request.order_id)
    .bind(&idempotency_key)
    .bind(request.amount)
    .bind(status)
    .bind(payment_method)
    .bind(&response_data)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to create payment event: {}", e))
    })?;

    // On success, transition order Reserved -> Paid and verify the update.
    if status == "Success" {
        let result = sqlx::query(
            "UPDATE orders SET status = 'Paid', paid_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND status = 'Reserved'",
        )
        .bind(request.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to update order status: {}", e))
        })?;

        // If no row was updated, the order was concurrently modified — roll back
        // the entire transaction so the payment event is NOT persisted.
        if result.rows_affected() != 1 {
            return Err(AppError::Conflict(
                "Order status changed concurrently — payment rolled back. Please retry.".to_string(),
            ));
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit payment: {}", e)))?;

    log::info!(
        "Payment simulated: order_id={}, status={}, amount={}, idempotency_key={}",
        request.order_id,
        status,
        request.amount,
        idempotency_key
    );

    Ok(event)
}
