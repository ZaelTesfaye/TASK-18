use crate::api::client;
use crate::store;
use crate::types::*;

pub async fn login(req: &LoginRequest) -> Result<LoginResponse, ApiError> {
    client::post("/auth/login", req).await
}

pub async fn register(req: &RegisterRequest) -> Result<UserResponse, ApiError> {
    client::post("/auth/register", req).await
}

pub async fn refresh_token(token: &str) -> Result<RefreshResponse, ApiError> {
    let body = serde_json::json!({ "refresh_token": token });
    client::post("/auth/refresh", &body).await
}

pub async fn logout() -> Result<(), ApiError> {
    let refresh_token = store::get_refresh_token().unwrap_or_default();
    let body = serde_json::json!({ "refresh_token": refresh_token });
    client::post_empty("/auth/logout", &body).await
}

/// Get current user profile — backend endpoint is /users/me.
pub async fn get_current_user() -> Result<UserResponse, ApiError> {
    client::get("/users/me").await
}
