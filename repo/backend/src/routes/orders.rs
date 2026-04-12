use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::{require_owner_or_admin, require_role};
use crate::middleware::risk::{check_bulk_order_risk, check_discount_abuse_risk};
use crate::models::common::PaginatedResponse;
use crate::models::order::{
    CreateOrderRequest, MergeOrderRequest, Order, OrderItem, OrderItemResponse, OrderResponse,
    ReturnRequest, SplitOrderRequest, StatusTimeline,
};
use crate::services::audit_service;
use crate::services::encryption_service;
use crate::services::order_service;
use crate::services::order_state_machine::{OrderStateMachine, OrderStatus, TransitionContext};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .route("", web::get().to(list_orders))
            .route("/{id}", web::get().to(get_order))
            .route("", web::post().to(create_order))
            .route("/{id}/status", web::put().to(update_status))
            .route("/{id}/return", web::post().to(request_return))
            .route("/{id}/exchange", web::post().to(request_exchange))
            .route("/{id}/refund", web::post().to(request_refund))
            .route("/{id}/split", web::post().to(split_order))
            .route("/{id}/merge", web::post().to(merge_orders))
            .route("/{id}/invoice", web::get().to(get_invoice)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OrderListQuery {
    status: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

impl OrderListQuery {
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
struct UpdateStatusRequest {
    status: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn build_order_response(pool: &PgPool, order: &Order) -> Result<OrderResponse, AppError> {
    let items = sqlx::query_as::<_, OrderItemRow>(
        "SELECT oi.id, oi.product_id, p.title AS product_title, \
         oi.quantity, oi.unit_price::float8 AS unit_price, \
         oi.total_price::float8 AS total_price \
         FROM order_items oi \
         JOIN products p ON p.id = oi.product_id \
         WHERE oi.order_id = $1 \
         ORDER BY oi.created_at ASC",
    )
    .bind(order.id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch order items: {}", e)))?;

    let item_responses: Vec<OrderItemResponse> = items
        .into_iter()
        .map(|i| OrderItemResponse {
            id: i.id,
            product_id: i.product_id,
            product_title: i.product_title,
            quantity: i.quantity,
            unit_price: i.unit_price,
            total_price: i.total_price,
        })
        .collect();

    let timeline = StatusTimeline {
        created_at: order.created_at,
        reservation_expires_at: order.reservation_expires_at,
        paid_at: order.paid_at,
        shipped_at: order.shipped_at,
        delivered_at: order.delivered_at,
        completed_at: order.completed_at,
        cancelled_at: order.cancelled_at,
        refunded_at: order.refunded_at,
    };

    Ok(OrderResponse {
        id: order.id,
        user_id: order.user_id,
        status: order.status.clone(),
        parent_order_id: order.parent_order_id,
        total_amount: order.total_amount,
        discount_amount: order.discount_amount,
        reason_code: order.reason_code.clone(),
        payment_method: order.payment_method.clone(),
        items: item_responses,
        status_timeline: timeline,
        legal_hold: order.legal_hold,
        created_at: order.created_at,
        updated_at: order.updated_at,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct OrderItemRow {
    id: Uuid,
    product_id: Uuid,
    product_title: String,
    quantity: i32,
    unit_price: f64,
    total_price: f64,
}

// ---------------------------------------------------------------------------
// GET /api/orders
// ---------------------------------------------------------------------------

async fn list_orders(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    query: web::Query<OrderListQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let page = q.page();
    let per_page = q.per_page();
    let offset = q.offset();

    let (count_sql, data_sql) = if q.status.is_some() {
        (
            "SELECT COUNT(*) FROM orders WHERE user_id = $1 AND status = $2".to_string(),
            "SELECT id, user_id, status, parent_order_id, \
             shipping_address_encrypted, total_amount::float8 AS total_amount, \
             discount_amount::float8 AS discount_amount, reason_code, payment_method, \
             reservation_expires_at, paid_at, shipped_at, delivered_at, \
             completed_at, cancelled_at, refunded_at, legal_hold, \
             created_at, updated_at \
             FROM orders WHERE user_id = $1 AND status = $2 \
             ORDER BY created_at DESC LIMIT $3 OFFSET $4"
                .to_string(),
        )
    } else {
        (
            "SELECT COUNT(*) FROM orders WHERE user_id = $1".to_string(),
            "SELECT id, user_id, status, parent_order_id, \
             shipping_address_encrypted, total_amount::float8 AS total_amount, \
             discount_amount::float8 AS discount_amount, reason_code, payment_method, \
             reservation_expires_at, paid_at, shipped_at, delivered_at, \
             completed_at, cancelled_at, refunded_at, legal_hold, \
             created_at, updated_at \
             FROM orders WHERE user_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                .to_string(),
        )
    };

    let total = if let Some(ref status) = q.status {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(user.user_id)
            .bind(status)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count orders: {}", e)))?
    } else {
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(user.user_id)
            .fetch_one(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to count orders: {}", e)))?
    };

    let orders: Vec<Order> = if let Some(ref status) = q.status {
        sqlx::query_as::<_, Order>(&data_sql)
            .bind(user.user_id)
            .bind(status)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch orders: {}", e)))?
    } else {
        sqlx::query_as::<_, Order>(&data_sql)
            .bind(user.user_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch orders: {}", e)))?
    };

    let mut responses = Vec::new();
    for order in &orders {
        let resp = build_order_response(pool.get_ref(), order).await?;
        responses.push(resp);
    }

    let paginated = PaginatedResponse::new(responses, total, page, per_page);
    Ok(HttpResponse::Ok().json(paginated))
}

// ---------------------------------------------------------------------------
// GET /api/orders/{id}
// ---------------------------------------------------------------------------

async fn get_order(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();

    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    require_owner_or_admin(&user, order.user_id)?;

    let response = build_order_response(pool.get_ref(), &order).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/orders
// ---------------------------------------------------------------------------

async fn create_order(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateOrderRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();
    let config = Config::get();

    if req.items.is_empty() {
        return Err(AppError::ValidationError(
            "Order must contain at least one item".to_string(),
        ));
    }
    if req.shipping_address.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Shipping address is required".to_string(),
        ));
    }

    // Check bulk order risk
    let total_quantity: i32 = req.items.iter().map(|i| i.quantity).sum();
    check_bulk_order_risk(pool.get_ref(), user.user_id, total_quantity as u32, config).await?;

    // Check discount abuse risk (flags users who have used discounts too frequently)
    check_discount_abuse_risk(pool.get_ref(), user.user_id, config).await?;

    let key_bytes = derive_key_bytes(&config.encryption_key);

    // Place order using the order service
    let order = order_service::place_order(
        pool.get_ref(),
        user.user_id,
        req,
        &key_bytes,
        config.encryption_key_version,
    )
    .await?;

    // Clear user's cart after successful order
    let cart_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM carts WHERE user_id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch cart: {}", e)))?;

    if let Some(cid) = cart_id {
        sqlx::query("DELETE FROM cart_items WHERE cart_id = $1")
            .bind(cid)
            .execute(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to clear cart: {}", e)))?;
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.create",
        Some(&ip),
        "order",
        &order.id.to_string(),
        Some(serde_json::json!({
            "total_amount": order.total_amount,
            "status": order.status
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &order).await?;
    Ok(HttpResponse::Created().json(response))
}

fn derive_key_bytes(key_str: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key_str.as_bytes());
    hasher.finalize().to_vec()
}

// ---------------------------------------------------------------------------
// PUT /api/orders/{id}/status
// ---------------------------------------------------------------------------

async fn update_status(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateStatusRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let current = OrderStatus::from_str(&order.status)?;
    let target = OrderStatus::from_str(&req.status)?;

    // Check if admin-only transition
    if OrderStateMachine::is_admin_only_transition(&current, &target) {
        require_role(&user, "Admin")?;
    } else {
        require_owner_or_admin(&user, order.user_id)?;
    }

    // Validate transition — the state machine will reject return/exchange/refund
    // transitions that lack proper context (reason code + 30-day window), so the
    // generic endpoint cannot bypass those checks.
    let new_status = OrderStateMachine::transition(current, target)?;
    let new_status_str = new_status.as_str();

    // Update order status and relevant timestamps
    let timestamp_update = match new_status {
        OrderStatus::Paid => ", paid_at = NOW()",
        OrderStatus::Shipped => ", shipped_at = NOW()",
        OrderStatus::Delivered => ", delivered_at = NOW()",
        OrderStatus::Completed => ", completed_at = NOW()",
        OrderStatus::Cancelled => ", cancelled_at = NOW()",
        OrderStatus::Refunded => ", refunded_at = NOW()",
        _ => "",
    };

    let update_sql = format!(
        "UPDATE orders SET status = $1, updated_at = NOW() {} WHERE id = $2 \
         RETURNING id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at",
        timestamp_update
    );

    let updated = sqlx::query_as::<_, Order>(&update_sql)
        .bind(new_status_str)
        .bind(order_id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to update order status: {}", e)))?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.status_change",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({
            "from": order.status,
            "to": new_status_str
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &updated).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/orders/{id}/return
// ---------------------------------------------------------------------------

async fn request_return(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ReturnRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.reason_code.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Reason code is required for returns".to_string(),
        ));
    }

    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    require_owner_or_admin(&user, order.user_id)?;

    let target_status = "ReturnRequested";

    // Centralized validation: reason code + 30-day window enforced by state machine
    let current = OrderStatus::from_str(&order.status)?;
    let target = OrderStatus::from_str(target_status)?;
    let ctx = TransitionContext {
        reason_code: Some(&req.reason_code),
        delivered_at: order.delivered_at,
    };
    OrderStateMachine::transition_with_context(current, target, Some(&ctx))?;

    let updated = sqlx::query_as::<_, Order>(
        "UPDATE orders SET status = $1, reason_code = $2::return_reason, updated_at = NOW() \
         WHERE id = $3 \
         RETURNING id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at",
    )
    .bind(target_status)
    .bind(&req.reason_code)
    .bind(order_id)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.return_request",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({
            "reason_code": req.reason_code,
            "target_status": target_status
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &updated).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/orders/{id}/exchange
// ---------------------------------------------------------------------------

async fn request_exchange(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ReturnRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.reason_code.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Reason code is required for exchange requests".to_string(),
        ));
    }

    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    require_owner_or_admin(&user, order.user_id)?;

    let target_status = "ExchangeRequested";

    let current = OrderStatus::from_str(&order.status)?;
    let target = OrderStatus::from_str(target_status)?;
    let ctx = TransitionContext {
        reason_code: Some(&req.reason_code),
        delivered_at: order.delivered_at,
    };
    OrderStateMachine::transition_with_context(current, target, Some(&ctx))?;

    let updated = sqlx::query_as::<_, Order>(
        "UPDATE orders SET status = $1, reason_code = $2::return_reason, updated_at = NOW() \
         WHERE id = $3 \
         RETURNING id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at",
    )
    .bind(target_status)
    .bind(&req.reason_code)
    .bind(order_id)
    .fetch_one(pool.get_ref())
    .await?;

    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.exchange_request",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({
            "reason_code": req.reason_code,
            "target_status": target_status
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &updated).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/orders/{id}/refund
// ---------------------------------------------------------------------------

async fn request_refund(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ReturnRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.reason_code.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Reason code is required for refund requests".to_string(),
        ));
    }

    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    // Refunds require admin privileges
    require_role(&user, "Admin")?;

    let target_status = "Refunded";

    let current = OrderStatus::from_str(&order.status)?;
    let target = OrderStatus::from_str(target_status)?;
    let ctx = TransitionContext {
        reason_code: Some(&req.reason_code),
        delivered_at: order.delivered_at,
    };
    OrderStateMachine::transition_with_context(current, target, Some(&ctx))?;

    let updated = sqlx::query_as::<_, Order>(
        "UPDATE orders SET status = $1, reason_code = $2::return_reason, \
         refunded_at = NOW(), updated_at = NOW() \
         WHERE id = $3 \
         RETURNING id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at",
    )
    .bind(target_status)
    .bind(&req.reason_code)
    .bind(order_id)
    .fetch_one(pool.get_ref())
    .await?;

    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.refund",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({
            "reason_code": req.reason_code,
            "target_status": target_status
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &updated).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/orders/{id}/split
// ---------------------------------------------------------------------------

async fn split_order(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<SplitOrderRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let order_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let children = order_service::split_order(pool.get_ref(), order_id, req.item_ids).await?;

    // Audit log
    let child_ids: Vec<String> = children.iter().map(|c| c.id.to_string()).collect();
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.split",
        Some(&ip),
        "order",
        &order_id.to_string(),
        Some(serde_json::json!({
            "child_order_ids": child_ids
        })),
    )
    .await?;

    let mut responses = Vec::new();
    for child in &children {
        let resp = build_order_response(pool.get_ref(), child).await?;
        responses.push(resp);
    }

    Ok(HttpResponse::Ok().json(responses))
}

// ---------------------------------------------------------------------------
// POST /api/orders/{id}/merge
// ---------------------------------------------------------------------------

async fn merge_orders(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<MergeOrderRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let _context_order_id = path.into_inner();
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let merged = order_service::merge_orders(pool.get_ref(), req.order_ids.clone()).await?;

    // Audit log
    let source_ids: Vec<String> = req.order_ids.iter().map(|id| id.to_string()).collect();
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "order.merge",
        Some(&ip),
        "order",
        &merged.id.to_string(),
        Some(serde_json::json!({
            "source_order_ids": source_ids
        })),
    )
    .await?;

    let response = build_order_response(pool.get_ref(), &merged).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/orders/{id}/invoice
// ---------------------------------------------------------------------------

async fn get_invoice(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let order_id = path.into_inner();

    // Verify order ownership
    let order = sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, parent_order_id, \
         shipping_address_encrypted, total_amount::float8 AS total_amount, \
         discount_amount::float8 AS discount_amount, reason_code, payment_method, \
         reservation_expires_at, paid_at, shipped_at, delivered_at, \
         completed_at, cancelled_at, refunded_at, legal_hold, \
         created_at, updated_at \
         FROM orders WHERE id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    require_owner_or_admin(&user, order.user_id)?;

    // Check if invoice already exists
    let existing = sqlx::query_as::<_, crate::models::payment::Invoice>(
        "SELECT * FROM invoices WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check invoice: {}", e)))?;

    let invoice = match existing {
        Some(inv) => inv,
        None => order_service::generate_invoice(pool.get_ref(), order_id).await?,
    };

    Ok(HttpResponse::Ok().json(invoice))
}
