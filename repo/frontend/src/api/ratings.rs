use crate::api::client;
use crate::types::*;

pub async fn create_rating(req: &CreateRatingRequest) -> Result<Rating, ApiError> {
    client::post("/ratings", req).await
}

pub async fn get_product_ratings(
    product_id: &str,
    page: u64,
) -> Result<PaginatedResponse<Rating>, ApiError> {
    client::get(&format!(
        "/ratings/product/{}?page={}&per_page=10",
        product_id, page
    ))
    .await
}

pub async fn get_rating(id: &str) -> Result<Rating, ApiError> {
    client::get(&format!("/ratings/{}", id)).await
}

pub async fn update_rating(id: &str, req: &CreateRatingRequest) -> Result<Rating, ApiError> {
    client::put(&format!("/ratings/{}", id), req).await
}

pub async fn get_leaderboard(query: &LeaderboardQuery) -> Result<PaginatedResponse<LeaderboardEntry>, ApiError> {
    let mut params = Vec::new();
    if let Some(ref p) = query.period {
        params.push(format!("period={}", p));
    }
    if let Some(ref g) = query.genre {
        if !g.is_empty() {
            params.push(format!("genre={}", g));
        }
    }
    params.push(format!("page={}", query.page.unwrap_or(1)));
    params.push(format!("per_page={}", query.per_page.unwrap_or(20)));
    let qs = format!("?{}", params.join("&"));
    client::get(&format!("/leaderboards{}", qs)).await
}
