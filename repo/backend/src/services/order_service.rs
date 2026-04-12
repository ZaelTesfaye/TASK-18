use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::order::{CreateOrderRequest, Order, OrderItem};
use crate::models::payment::Invoice;
use crate::services::encryption_service;

// ---------------------------------------------------------------------------
// Place order
// ---------------------------------------------------------------------------

/// Creates a new order from the request:
/// 1. Validates stock availability with `SELECT ... FOR UPDATE` to prevent overselling
/// 2. Reserves inventory (decrements stock)
/// 3. Creates the order with a 30-minute reservation expiry
/// 4. Creates order items
pub async fn place_order(
    pool: &PgPool,
    user_id: Uuid,
    request: CreateOrderRequest,
    encryption_key: &[u8],
    key_version: u32,
) -> Result<Order, AppError> {
    if request.items.is_empty() {
        return Err(AppError::ValidationError(
            "Order must contain at least one item".to_string(),
        ));
    }

    let encrypted_address =
        encryption_service::encrypt(&request.shipping_address, encryption_key, key_version)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    let order_id = Uuid::new_v4();
    let mut total_amount: f64 = 0.0;

    // Validate stock and reserve inventory for each item
    for item in &request.items {
        let product = sqlx::query_as::<_, ProductRow>(
            "SELECT id, price, stock FROM products WHERE id = $1 FOR UPDATE",
        )
        .bind(item.product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch product: {}", e)))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product {} not found", item.product_id))
        })?;

        if product.stock < item.quantity {
            return Err(AppError::Conflict(format!(
                "Insufficient stock for product {}: requested {}, available {}",
                item.product_id, item.quantity, product.stock
            )));
        }

        // Reserve inventory
        sqlx::query("UPDATE products SET stock = stock - $1, updated_at = NOW() WHERE id = $2")
            .bind(item.quantity)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to reserve inventory: {}", e))
            })?;

        let line_total = product.price * item.quantity as f64;
        total_amount += line_total;

        // Create order item
        sqlx::query(
            "INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, total_price, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(order_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(product.price)
        .bind(line_total)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to create order item: {}", e))
        })?;
    }

    let reservation_expires = Utc::now() + Duration::minutes(30);

    // Create the order in Reserved state
    let order = sqlx::query_as::<_, Order>(
        "INSERT INTO orders (id, user_id, status, shipping_address_encrypted, total_amount, \
         discount_amount, payment_method, reservation_expires_at, created_at, updated_at) \
         VALUES ($1, $2, 'Reserved', $3, $4, 0, $5, $6, NOW(), NOW()) \
         RETURNING *",
    )
    .bind(order_id)
    .bind(user_id)
    .bind(&encrypted_address)
    .bind(total_amount)
    .bind(&request.payment_method)
    .bind(reservation_expires)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create order: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit transaction: {}", e)))?;

    log::info!(
        "Order placed: order_id={}, user_id={}, total={}",
        order.id,
        user_id,
        total_amount
    );

    Ok(order)
}

/// Minimal product row for stock validation during order placement.
#[derive(Debug, sqlx::FromRow)]
struct ProductRow {
    #[allow(dead_code)]
    id: Uuid,
    #[sqlx(try_from = "bigdecimal::BigDecimal")]
    price: f64,
    stock: i32,
}

// ---------------------------------------------------------------------------
// Return eligibility
// ---------------------------------------------------------------------------

/// Checks whether an order is eligible for return (within 30 days of delivery).
pub fn check_return_eligibility(order: &Order) -> Result<(), AppError> {
    let delivered_at = order.delivered_at.ok_or_else(|| {
        AppError::BadRequest("Order has not been delivered yet".to_string())
    })?;

    let deadline = delivered_at + Duration::days(30);
    if Utc::now() > deadline {
        return Err(AppError::BadRequest(
            "Return window has expired (30 days from delivery)".to_string(),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Split order
// ---------------------------------------------------------------------------

/// Splits an order into two child orders:
/// - Child A contains the specified `item_ids`
/// - Child B contains the remaining items
///
/// The original order is cancelled and lineage records are created.
/// The accounting invariant (parent total == sum of children) is preserved.
pub async fn split_order(
    pool: &PgPool,
    order_id: Uuid,
    item_ids: Vec<Uuid>,
) -> Result<Vec<Order>, AppError> {
    if item_ids.is_empty() {
        return Err(AppError::ValidationError(
            "Must specify at least one item to split".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    let parent = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 FOR UPDATE")
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch order: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let all_items = sqlx::query_as::<_, OrderItem>(
        "SELECT * FROM order_items WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch order items: {}", e)))?;

    let split_items: Vec<&OrderItem> = all_items
        .iter()
        .filter(|i| item_ids.contains(&i.id))
        .collect();
    let remaining_items: Vec<&OrderItem> = all_items
        .iter()
        .filter(|i| !item_ids.contains(&i.id))
        .collect();

    if split_items.is_empty() {
        return Err(AppError::BadRequest(
            "None of the specified item IDs belong to this order".to_string(),
        ));
    }
    if remaining_items.is_empty() {
        return Err(AppError::BadRequest(
            "Cannot split all items -- at least one must remain".to_string(),
        ));
    }

    let mut children = Vec::new();

    // Create child A (split items)
    let child_a_id = Uuid::new_v4();
    let child_a_total: f64 = split_items.iter().map(|i| i.total_price).sum();
    let child_a = create_child_order(
        &mut tx,
        child_a_id,
        &parent,
        child_a_total,
        &split_items,
    )
    .await?;
    children.push(child_a);

    // Create child B (remaining items)
    let child_b_id = Uuid::new_v4();
    let child_b_total: f64 = remaining_items.iter().map(|i| i.total_price).sum();
    let child_b = create_child_order(
        &mut tx,
        child_b_id,
        &parent,
        child_b_total,
        &remaining_items,
    )
    .await?;
    children.push(child_b);

    // Record lineage
    for child in &children {
        sqlx::query(
            "INSERT INTO order_lineage (id, parent_order_id, child_order_id, operation, created_at) \
             VALUES ($1, $2, $3, 'split', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(order_id)
        .bind(child.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to record lineage: {}", e))
        })?;
    }

    // Cancel the parent order
    sqlx::query(
        "UPDATE orders SET status = 'Cancelled', cancelled_at = NOW(), updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to cancel parent order: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit split: {}", e)))?;

    log::info!(
        "Order split: parent={}, children=[{}, {}]",
        order_id,
        children[0].id,
        children[1].id
    );

    Ok(children)
}

/// Internal helper to create a child order during a split.
async fn create_child_order(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    child_id: Uuid,
    parent: &Order,
    total: f64,
    items: &[&OrderItem],
) -> Result<Order, AppError> {
    let child = sqlx::query_as::<_, Order>(
        "INSERT INTO orders (id, user_id, status, parent_order_id, shipping_address_encrypted, \
         total_amount, discount_amount, reservation_expires_at, paid_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, NOW(), NOW()) RETURNING *",
    )
    .bind(child_id)
    .bind(parent.user_id)
    .bind(&parent.status)
    .bind(parent.id)
    .bind(&parent.shipping_address_encrypted)
    .bind(total)
    .bind(parent.reservation_expires_at)
    .bind(parent.paid_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create child order: {}", e)))?;

    for item in items {
        sqlx::query(
            "INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, total_price, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(child_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(item.unit_price)
        .bind(item.total_price)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to copy order item: {}", e))
        })?;
    }

    Ok(child)
}

// ---------------------------------------------------------------------------
// Merge orders
// ---------------------------------------------------------------------------

/// Merges multiple child orders back into a single order.
///
/// All orders must share the same `parent_order_id`. A new merged order is
/// created, children are cancelled, and lineage is recorded.
pub async fn merge_orders(
    pool: &PgPool,
    order_ids: Vec<Uuid>,
) -> Result<Order, AppError> {
    if order_ids.len() < 2 {
        return Err(AppError::ValidationError(
            "Must specify at least two orders to merge".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    let mut orders = Vec::new();
    for oid in &order_ids {
        let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 FOR UPDATE")
            .bind(oid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to fetch order: {}", e)))?
            .ok_or_else(|| AppError::NotFound(format!("Order {} not found", oid)))?;
        orders.push(order);
    }

    // Validate all share the same parent
    let parent_id = orders[0].parent_order_id.ok_or_else(|| {
        AppError::BadRequest("Orders without a parent cannot be merged".to_string())
    })?;
    for o in &orders {
        if o.parent_order_id != Some(parent_id) {
            return Err(AppError::BadRequest(
                "All orders must share the same parent to merge".to_string(),
            ));
        }
    }

    // Collect all items from all child orders
    let mut all_items = Vec::new();
    let mut total_amount: f64 = 0.0;
    for o in &orders {
        let items = sqlx::query_as::<_, OrderItem>(
            "SELECT * FROM order_items WHERE order_id = $1",
        )
        .bind(o.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to fetch order items: {}", e))
        })?;
        total_amount += o.total_amount;
        all_items.extend(items);
    }

    // Create merged order
    let merged_id = Uuid::new_v4();
    let merged = sqlx::query_as::<_, Order>(
        "INSERT INTO orders (id, user_id, status, parent_order_id, shipping_address_encrypted, \
         total_amount, discount_amount, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 0, NOW(), NOW()) RETURNING *",
    )
    .bind(merged_id)
    .bind(orders[0].user_id)
    .bind(&orders[0].status)
    .bind(parent_id)
    .bind(&orders[0].shipping_address_encrypted)
    .bind(total_amount)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create merged order: {}", e)))?;

    // Copy items to merged order
    for item in &all_items {
        sqlx::query(
            "INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, total_price, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(merged_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(item.unit_price)
        .bind(item.total_price)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to copy order item: {}", e))
        })?;
    }

    // Cancel children and record lineage
    for o in &orders {
        sqlx::query(
            "UPDATE orders SET status = 'Cancelled', cancelled_at = NOW(), updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(o.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to cancel child order: {}", e))
        })?;

        sqlx::query(
            "INSERT INTO order_lineage (id, parent_order_id, child_order_id, operation, created_at) \
             VALUES ($1, $2, $3, 'merge', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(merged_id)
        .bind(o.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to record lineage: {}", e))
        })?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit merge: {}", e)))?;

    log::info!(
        "Orders merged: merged_id={}, children={:?}",
        merged_id,
        order_ids
    );

    Ok(merged)
}

// ---------------------------------------------------------------------------
// Reconcile expired reservations
// ---------------------------------------------------------------------------

/// Finds all orders in `Reserved` status whose reservation has expired,
/// cancels them, releases inventory back to products, and logs each
/// cancellation to the audit log.
///
/// Returns the number of orders reconciled.
pub async fn reconcile_expired_orders(pool: &PgPool) -> Result<u32, AppError> {
    let now = Utc::now();
    let expired = sqlx::query_as::<_, Order>(
        "SELECT * FROM orders WHERE status = 'Reserved' AND reservation_expires_at < $1",
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch expired orders: {}", e))
    })?;

    let mut count: u32 = 0;

    for order in &expired {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to begin transaction: {}", e))
            })?;

        // Release inventory
        let items = sqlx::query_as::<_, OrderItem>(
            "SELECT * FROM order_items WHERE order_id = $1",
        )
        .bind(order.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to fetch order items: {}", e))
        })?;

        for item in &items {
            sqlx::query(
                "UPDATE products SET stock = stock + $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(item.quantity)
            .bind(item.product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to release inventory: {}", e))
            })?;
        }

        // Cancel the order
        sqlx::query(
            "UPDATE orders SET status = 'Cancelled', cancelled_at = NOW(), updated_at = NOW() \
             WHERE id = $1",
        )
        .bind(order.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to cancel order: {}", e))
        })?;

        // Audit log
        sqlx::query(
            "INSERT INTO audit_log (id, actor, action, timestamp, target_type, target_id, change_summary) \
             VALUES ($1, 'SYSTEM', 'order.reservation_expired', NOW(), 'order', $2, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(order.id.to_string())
        .bind(serde_json::json!({"reason": "Reservation expired after 30 minutes"}))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to write audit log: {}", e))
        })?;

        tx.commit()
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to commit reconciliation: {}", e))
            })?;

        count += 1;
        log::info!("Reconciled expired order: order_id={}", order.id);
    }

    log::info!("Reconciliation complete: {} orders cancelled", count);
    Ok(count)
}

// ---------------------------------------------------------------------------
// Generate invoice
// ---------------------------------------------------------------------------

/// Generates an invoice for a given order. The invoice total must equal the
/// sum of all line item totals.
///
/// Disallows invoicing cancelled orders that have active child orders
/// (from split/merge operations) to prevent duplicate financial artifacts.
pub async fn generate_invoice(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Invoice, AppError> {
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch order: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    // Block invoicing for cancelled orders that have been superseded by children
    if order.status == "Cancelled" {
        let has_children = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM order_lineage WHERE parent_order_id = $1)",
        )
        .bind(order_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if has_children {
            return Err(AppError::BadRequest(
                "Cannot generate invoice for a cancelled order that has been split or merged. \
                 Invoice the child orders instead.".to_string(),
            ));
        }
    }

    let items = sqlx::query_as::<_, OrderItem>(
        "SELECT * FROM order_items WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch order items: {}", e))
    })?;

    let line_items: Vec<serde_json::Value> = items
        .iter()
        .map(|i| {
            serde_json::json!({
                "product_id": i.product_id,
                "quantity": i.quantity,
                "unit_price": i.unit_price,
                "total_price": i.total_price,
            })
        })
        .collect();

    let total: f64 = items.iter().map(|i| i.total_price).sum();

    // Verify accounting invariant
    let diff = (total - order.total_amount).abs();
    if diff > 0.01 {
        log::warn!(
            "Invoice total mismatch: order_total={}, items_total={}, order_id={}",
            order.total_amount,
            total,
            order_id
        );
    }

    let invoice_number = format!(
        "INV-{}",
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0000")
    );

    // Use ON CONFLICT to handle the race where two concurrent requests both
    // try to create an invoice. The UNIQUE constraint on order_id ensures only
    // one succeeds; the loser returns the existing invoice.
    let invoice = sqlx::query_as::<_, Invoice>(
        "INSERT INTO invoices (id, order_id, invoice_number, total_amount, line_items, created_at) \
         VALUES ($1, $2, $3, $4, $5, NOW()) \
         ON CONFLICT (order_id) DO UPDATE SET id = invoices.id \
         RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(&invoice_number)
    .bind(total)
    .bind(serde_json::json!(line_items))
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create invoice: {}", e)))?;

    log::info!(
        "Invoice generated: invoice_id={}, order_id={}, total={}",
        invoice.id,
        order_id,
        total
    );

    Ok(invoice)
}
