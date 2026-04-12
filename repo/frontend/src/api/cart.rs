use crate::api::client;
use crate::types::*;

pub async fn get_cart() -> Result<Cart, ApiError> {
    client::get("/cart").await
}

/// Backend returns the full cart response after adding an item.
pub async fn add_to_cart(req: &AddToCartRequest) -> Result<Cart, ApiError> {
    client::post("/cart/items", req).await
}

/// Backend returns the full cart response after updating an item.
pub async fn update_cart_item(item_id: &str, quantity: u32) -> Result<Cart, ApiError> {
    let body = UpdateCartItemRequest { quantity };
    client::put(&format!("/cart/items/{}", item_id), &body).await
}

pub async fn remove_cart_item(item_id: &str) -> Result<(), ApiError> {
    client::delete(&format!("/cart/items/{}", item_id)).await
}

pub async fn clear_cart() -> Result<(), ApiError> {
    client::delete("/cart").await
}
