use crate::api::client;
use crate::types::*;

pub async fn list_orders(page: u64) -> Result<PaginatedResponse<Order>, ApiError> {
    client::get(&format!("/orders?page={}&per_page=10", page)).await
}

pub async fn get_order(id: &str) -> Result<Order, ApiError> {
    client::get(&format!("/orders/{}", id)).await
}

pub async fn create_order(req: &CreateOrderRequest) -> Result<Order, ApiError> {
    client::post("/orders", req).await
}

pub async fn update_order_status(order_id: &str, status: &str) -> Result<Order, ApiError> {
    let body = serde_json::json!({ "status": status });
    client::put(&format!("/orders/{}/status", order_id), &body).await
}

pub async fn request_return(order_id: &str, req: &ReturnRequest) -> Result<Order, ApiError> {
    client::post(&format!("/orders/{}/return", order_id), req).await
}

pub async fn get_invoice(order_id: &str) -> Result<serde_json::Value, ApiError> {
    client::get(&format!("/orders/{}/invoice", order_id)).await
}

/// Simulate a payment via the dedicated /payment/simulate endpoint.
pub async fn simulate_payment(
    order_id: &str,
    amount: f64,
    outcome: &str,
    payment_method: Option<String>,
) -> Result<PaymentResponse, ApiError> {
    let body = SimulatePaymentRequest {
        order_id: order_id.to_string(),
        amount,
        outcome: outcome.to_string(),
        payment_method: payment_method.or_else(|| Some("local_tender".to_string())),
        attempt_number: 1,
    };
    client::post("/payment/simulate", &body).await
}
