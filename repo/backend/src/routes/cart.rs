use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::cart::{AddToCartRequest, CartItemResponse, CartResponse, UpdateCartItemRequest};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/cart")
            .route("", web::get().to(get_cart))
            .route("/items", web::post().to(add_item))
            .route("/items/{id}", web::put().to(update_item))
            .route("/items/{id}", web::delete().to(remove_item))
            .route("", web::delete().to(clear_cart)),
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct CartRow {
    id: Uuid,
    user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct CartItemRow {
    id: Uuid,
    product_id: Uuid,
    product_title: String,
    unit_price: f64,
    quantity: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct ExistingCartItem {
    id: Uuid,
    quantity: i32,
}

async fn get_or_create_cart(pool: &PgPool, user_id: Uuid) -> Result<CartRow, AppError> {
    let existing = sqlx::query_as::<_, CartRow>(
        "SELECT * FROM carts WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch cart: {}", e)))?;

    if let Some(cart) = existing {
        return Ok(cart);
    }

    let cart = sqlx::query_as::<_, CartRow>(
        "INSERT INTO carts (id, user_id, created_at, updated_at) \
         VALUES ($1, $2, NOW(), NOW()) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to create cart: {}", e)))?;

    Ok(cart)
}

async fn build_cart_response(pool: &PgPool, cart: &CartRow) -> Result<CartResponse, AppError> {
    let items = sqlx::query_as::<_, CartItemRow>(
        "SELECT ci.id, ci.product_id, p.title AS product_title, \
         p.price::float8 AS unit_price, ci.quantity \
         FROM cart_items ci \
         JOIN products p ON p.id = ci.product_id \
         WHERE ci.cart_id = $1 \
         ORDER BY ci.created_at ASC",
    )
    .bind(cart.id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch cart items: {}", e)))?;

    let mut total_amount = 0.0;
    let item_responses: Vec<CartItemResponse> = items
        .into_iter()
        .map(|i| {
            let line_total = i.unit_price * i.quantity as f64;
            total_amount += line_total;
            CartItemResponse {
                id: i.id,
                product_id: i.product_id,
                product_title: i.product_title,
                unit_price: i.unit_price,
                quantity: i.quantity,
                line_total,
            }
        })
        .collect();

    Ok(CartResponse {
        id: cart.id,
        user_id: cart.user_id,
        items: item_responses,
        total_amount,
        created_at: cart.created_at,
        updated_at: cart.updated_at,
    })
}

// ---------------------------------------------------------------------------
// GET /api/cart
// ---------------------------------------------------------------------------

async fn get_cart(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;
    let response = build_cart_response(pool.get_ref(), &cart).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/cart/items
// ---------------------------------------------------------------------------

async fn add_item(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<AddToCartRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();

    if req.quantity <= 0 {
        return Err(AppError::ValidationError(
            "Quantity must be greater than zero".to_string(),
        ));
    }

    // Validate product exists and has stock
    let stock = sqlx::query_scalar::<_, i32>(
        "SELECT stock FROM products WHERE id = $1 AND is_active = TRUE",
    )
    .bind(req.product_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch product: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Product not found or inactive".to_string()))?;

    if stock < req.quantity {
        return Err(AppError::Conflict(format!(
            "Insufficient stock: requested {}, available {}",
            req.quantity, stock
        )));
    }

    let cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;

    // Check if item already in cart - update quantity if so
    let existing = sqlx::query_as::<_, ExistingCartItem>(
        "SELECT id, quantity FROM cart_items WHERE cart_id = $1 AND product_id = $2",
    )
    .bind(cart.id)
    .bind(req.product_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check existing item: {}", e)))?;

    if let Some(item) = existing {
        let new_total = item.quantity + req.quantity;
        if new_total > stock {
            return Err(AppError::Conflict(format!(
                "Insufficient stock: {} already in cart + {} requested = {}, but only {} available",
                item.quantity, req.quantity, new_total, stock
            )));
        }
        sqlx::query(
            "UPDATE cart_items SET quantity = quantity + $1 WHERE id = $2",
        )
        .bind(req.quantity)
        .bind(item.id)
        .execute(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to update cart item: {}", e)))?;
    } else {
        sqlx::query(
            "INSERT INTO cart_items (id, cart_id, product_id, quantity, created_at) \
             VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(cart.id)
        .bind(req.product_id)
        .bind(req.quantity)
        .execute(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to add cart item: {}", e)))?;
    }

    // Update cart timestamp
    sqlx::query("UPDATE carts SET updated_at = NOW() WHERE id = $1")
        .bind(cart.id)
        .execute(pool.get_ref())
        .await?;

    let updated_cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;
    let response = build_cart_response(pool.get_ref(), &updated_cart).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// PUT /api/cart/items/{id}
// ---------------------------------------------------------------------------

async fn update_item(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateCartItemRequest>,
) -> Result<HttpResponse, AppError> {
    let item_id = path.into_inner();
    let req = body.into_inner();

    if req.quantity <= 0 {
        return Err(AppError::ValidationError(
            "Quantity must be greater than zero".to_string(),
        ));
    }

    // Verify item belongs to user's cart
    let cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;

    let rows = sqlx::query(
        "UPDATE cart_items SET quantity = $1 WHERE id = $2 AND cart_id = $3",
    )
    .bind(req.quantity)
    .bind(item_id)
    .bind(cart.id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to update cart item: {}", e)))?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Cart item not found".to_string()));
    }

    sqlx::query("UPDATE carts SET updated_at = NOW() WHERE id = $1")
        .bind(cart.id)
        .execute(pool.get_ref())
        .await?;

    let updated_cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;
    let response = build_cart_response(pool.get_ref(), &updated_cart).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// DELETE /api/cart/items/{id}
// ---------------------------------------------------------------------------

async fn remove_item(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let item_id = path.into_inner();
    let cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;

    let rows = sqlx::query(
        "DELETE FROM cart_items WHERE id = $1 AND cart_id = $2",
    )
    .bind(item_id)
    .bind(cart.id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to remove cart item: {}", e)))?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Cart item not found".to_string()));
    }

    sqlx::query("UPDATE carts SET updated_at = NOW() WHERE id = $1")
        .bind(cart.id)
        .execute(pool.get_ref())
        .await?;

    let response = build_cart_response(pool.get_ref(), &cart).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// DELETE /api/cart
// ---------------------------------------------------------------------------

async fn clear_cart(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let cart = get_or_create_cart(pool.get_ref(), user.user_id).await?;

    sqlx::query("DELETE FROM cart_items WHERE cart_id = $1")
        .bind(cart.id)
        .execute(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to clear cart: {}", e)))?;

    sqlx::query("UPDATE carts SET updated_at = NOW() WHERE id = $1")
        .bind(cart.id)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Cart cleared"})))
}
