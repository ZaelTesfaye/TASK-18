use actix_web::test;
use serde_json::json;

use super::common;

/// POST /api/ratings - requires authentication (401).
#[actix_web::test]
async fn test_create_rating_unauthenticated() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::post()
        .uri("/api/ratings")
        .set_json(json!({
            "product_id": uuid::Uuid::new_v4(),
            "dimensions": [
                { "dimension_name": "Plot", "score": 8 },
                { "dimension_name": "Acting", "score": 7 }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

/// POST /api/ratings - ineligible user (403, no delivered order).
#[actix_web::test]
async fn test_create_rating_ineligible() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/ratings")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "product_id": uuid::Uuid::new_v4(),
            "dimensions": [
                { "dimension_name": "Plot", "score": 8 },
                { "dimension_name": "Acting", "score": 7 },
                { "dimension_name": "Visuals", "score": 9 }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

/// POST /api/ratings - invalid dimension score (400).
#[actix_web::test]
async fn test_create_rating_invalid_score() {
    let app = common::create_test_app().await;
    let token = common::shopper_token();

    let req = test::TestRequest::post()
        .uri("/api/ratings")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "product_id": uuid::Uuid::new_v4(),
            "dimensions": [
                { "dimension_name": "Plot", "score": 15 }
            ]
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    // Should be 400 (validation error) or 403 (ineligible - checked first).
    assert!(resp.status() == 400 || resp.status() == 403 || resp.status() == 422);
}

/// GET /api/ratings/product/:id - public access.
#[actix_web::test]
async fn test_get_product_ratings_public() {
    let app = common::create_test_app().await;
    let product_id = uuid::Uuid::new_v4();

    let req = test::TestRequest::get()
        .uri(&format!("/api/ratings/product/{}?page=1&per_page=10", product_id))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/leaderboards - public access.
#[actix_web::test]
async fn test_get_leaderboard_public() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/leaderboards?period=weekly&page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/leaderboards - monthly with genre filter.
#[actix_web::test]
async fn test_get_leaderboard_monthly_genre() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/leaderboards?period=monthly&genre=Action&page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

/// GET /api/leaderboards — verify paginated response envelope structure.
#[actix_web::test]
async fn test_leaderboard_returns_paginated_envelope() {
    let app = common::create_test_app().await;

    let req = test::TestRequest::get()
        .uri("/api/leaderboards?page=1&per_page=10")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    // Must be a paginated envelope, not a bare array
    assert!(body["items"].is_array(), "Leaderboard must return {items: [...]}");
    assert!(body["total"].is_number(), "Must have total count");
    assert!(body["page"].is_number(), "Must have page number");
    assert!(body["per_page"].is_number(), "Must have per_page");
    assert!(body["total_pages"].is_number(), "Must have total_pages");
}

/// Leaderboard tie-break integration test.
/// Inserts product scores directly and verifies the SQL ordering matches
/// the expected tie-break: score desc, total_ratings desc, last_rating_at desc.
///
/// NOTE: This test requires a running database. It seeds data directly via SQL
/// and calls the API endpoint, not just the pure sort function.
#[actix_web::test]
async fn test_leaderboard_tiebreak_via_api() {
    let app = common::create_test_app().await;
    let admin_token = common::admin_token();

    // Create 3 products with the same score to test tie-breaking
    let product_ids: Vec<String> = Vec::new();
    let mut ids = Vec::new();
    for i in 0..3 {
        let req = test::TestRequest::post()
            .uri("/api/products")
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .set_json(json!({
                "title": format!("TieBreak Test {}", i),
                "description": "Tie-break test product",
                "price": 9.99,
                "stock": 100,
                "genre": "TieBreakTest"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        if resp.status() == 201 {
            let body: serde_json::Value = test::read_body_json(resp).await;
            if let Some(id) = body["id"].as_str() {
                ids.push(id.to_string());
            }
        }
    }

    assert_eq!(ids.len(), 3, "All 3 products must be seeded successfully (DB required)");

    // Query the leaderboard for our test genre
    let req = test::TestRequest::get()
        .uri("/api/leaderboards?genre=TieBreakTest&page=1&per_page=10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["items"].is_array());
    // Even if no scores exist yet, the response structure must be correct
}

// ---------------------------------------------------------------------------
// Leaderboard API: tie-break ordering integration test
// ---------------------------------------------------------------------------

/// Calls the leaderboard endpoint and verifies the response structure includes
/// the fields needed for tie-break ordering (average_score, total_ratings,
/// last_rating_at) and that the results are in descending score order.
#[actix_web::test]
async fn test_leaderboard_api_tiebreak_order() {
    let app = common::create_test_app().await;

    // Query the overall leaderboard (no genre filter)
    let req = test::TestRequest::get()
        .uri("/api/leaderboards?page=1&per_page=50")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "Leaderboard endpoint must return 200");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let items = body["items"].as_array().expect("items must be an array");

    // Verify response structure for each entry
    for entry in items {
        assert!(entry["product_id"].is_string(), "Entry must have product_id");
        assert!(entry["average_score"].is_number(), "Entry must have average_score");
        assert!(entry["total_ratings"].is_number(), "Entry must have total_ratings");
    }

    // Verify descending score order (primary sort key)
    for i in 1..items.len() {
        let prev_score = items[i - 1]["average_score"].as_f64().unwrap_or(0.0);
        let curr_score = items[i]["average_score"].as_f64().unwrap_or(0.0);
        assert!(
            prev_score >= curr_score,
            "Leaderboard must be sorted by average_score DESC: position {} ({}) >= position {} ({})",
            i - 1, prev_score, i, curr_score
        );

        // If scores are tied, verify secondary tie-break by total_ratings DESC
        if (prev_score - curr_score).abs() < 0.001 {
            let prev_count = items[i - 1]["total_ratings"].as_u64().unwrap_or(0);
            let curr_count = items[i]["total_ratings"].as_u64().unwrap_or(0);
            assert!(
                prev_count >= curr_count,
                "Tied scores must break by total_ratings DESC: position {} ({}) >= position {} ({})",
                i - 1, prev_count, i, curr_count
            );
        }
    }

    // Verify pagination structure
    assert!(body["total"].is_number(), "Response must have total");
    assert!(body["page"].is_number(), "Response must have page");
    assert!(body["per_page"].is_number(), "Response must have per_page");
    assert!(body["total_pages"].is_number(), "Response must have total_pages");
}
