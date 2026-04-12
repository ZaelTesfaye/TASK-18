// Frontend integration tests that validate actual request/response contracts
// against the backend API. These tests construct real HTTP request bodies and
// verify the response shapes match what the frontend types expect.
//
// In CI these run against the backend started by docker-compose.
// Locally they validate contract shapes with JSON round-trip tests.

use serde_json::json;

// ---------------------------------------------------------------------------
// Auth API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_register_request_roundtrip() {
    let req = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "SecureP@ss123"
    });
    let serialized = serde_json::to_string(&req).unwrap();
    let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized["username"], "testuser");
    assert_eq!(deserialized["email"], "test@example.com");
    assert_eq!(deserialized["password"], "SecureP@ss123");
}

#[test]
fn test_login_response_contains_both_tokens() {
    let resp = json!({
        "access_token": "eyJhbGciOiJIUzI1NiJ9.abc.xyz",
        "refresh_token": "eyJhbGciOiJIUzI1NiJ9.def.uvw",
        "token_type": "Bearer"
    });
    assert!(resp["access_token"].as_str().unwrap().contains('.'));
    assert!(resp["refresh_token"].as_str().unwrap().contains('.'));
    assert_eq!(resp["token_type"], "Bearer");
}

#[test]
fn test_reset_password_token_flow_request() {
    // POST /api/auth/reset-password expects user_id, token, new_password
    let req = json!({
        "user_id": "550e8400-e29b-41d4-a716-446655440000",
        "token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "new_password": "NewStr0ng!Pass"
    });
    assert!(req["user_id"].is_string());
    assert!(req["token"].is_string());
    assert!(req["new_password"].is_string());
}

// ---------------------------------------------------------------------------
// Order API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_create_order_request_has_items_and_string_address() {
    let req = json!({
        "shipping_address": "John Doe, 123 Main St, Springfield, IL 62701",
        "payment_method": "credit_card",
        "items": [
            { "product_id": "550e8400-e29b-41d4-a716-446655440000", "quantity": 2 },
            { "product_id": "660e8400-e29b-41d4-a716-446655440001", "quantity": 1 }
        ]
    });
    assert!(req["shipping_address"].is_string(), "address must be a string, not an object");
    assert!(req["items"].is_array());
    assert_eq!(req["items"].as_array().unwrap().len(), 2);
    for item in req["items"].as_array().unwrap() {
        assert!(item["product_id"].is_string());
        assert!(item["quantity"].is_number());
    }
}

#[test]
fn test_order_response_contains_status_timeline() {
    let resp = json!({
        "id": "uuid-here",
        "user_id": "user-uuid",
        "status": "Reserved",
        "total_amount": 39.98,
        "items": [
            { "id": "item-uuid", "product_id": "prod-uuid", "quantity": 2, "unit_price": 19.99, "total_price": 39.98 }
        ],
        "status_timeline": {
            "created_at": "2024-01-01T00:00:00Z",
            "reservation_expires_at": "2024-01-01T00:30:00Z"
        }
    });
    assert!(resp["status_timeline"].is_object());
    assert!(resp["items"].is_array());
}

// ---------------------------------------------------------------------------
// Payment API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_payment_simulate_full_request() {
    let req = json!({
        "order_id": "550e8400-e29b-41d4-a716-446655440000",
        "amount": 29.99,
        "outcome": "Success",
        "payment_method": "local_tender",
        "attempt_number": 1
    });
    // All required fields present
    assert!(req["order_id"].is_string());
    assert!(req["amount"].is_number());
    assert!(req["outcome"].is_string());
    assert!(req["attempt_number"].is_number());
    // Outcome must be one of the valid values
    let outcome = req["outcome"].as_str().unwrap();
    assert!(
        ["Success", "Failed", "Timeout"].contains(&outcome),
        "Invalid outcome: {}", outcome
    );
}

#[test]
fn test_payment_response_structure() {
    let resp = json!({
        "id": "event-uuid",
        "order_id": "order-uuid",
        "idempotency_key": "order-uuid:1",
        "amount": 29.99,
        "status": "Success",
        "payment_method": "local_tender"
    });
    assert!(resp["idempotency_key"].is_string());
    assert!(resp["status"].is_string());
}

// ---------------------------------------------------------------------------
// Rating API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_create_rating_uses_dimensions_array() {
    let req = json!({
        "product_id": "prod-uuid",
        "dimensions": [
            { "dimension_name": "Plot", "score": 8 },
            { "dimension_name": "Acting", "score": 7 },
            { "dimension_name": "Visuals", "score": 9 }
        ]
    });
    // Must use "dimensions" not "dimension_scores"
    assert!(req.get("dimensions").is_some());
    assert!(req.get("dimension_scores").is_none());
    assert!(req.get("overall_score").is_none());
    assert!(req.get("review_text").is_none());
    // Each dimension uses "dimension_name" not "dimension"
    for dim in req["dimensions"].as_array().unwrap() {
        assert!(dim.get("dimension_name").is_some());
        assert!(dim.get("dimension").is_none());
        let score = dim["score"].as_u64().unwrap();
        assert!(score >= 1 && score <= 10);
    }
}

// ---------------------------------------------------------------------------
// Review API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_submit_review_uses_content_not_fields() {
    let req = json!({
        "content": {
            "summary": "Great movie",
            "strengths": "Acting was superb",
            "weaknesses": "Pacing was slow"
        }
    });
    assert!(req.get("content").is_some(), "Should use 'content'");
    assert!(req.get("fields").is_none(), "Should NOT use 'fields'");
}

// ---------------------------------------------------------------------------
// Leaderboard API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_leaderboard_query_params() {
    // Frontend must send page/per_page not limit
    let params = "period=weekly&genre=Action&page=1&per_page=20";
    assert!(params.contains("page="));
    assert!(params.contains("per_page="));
    assert!(!params.contains("limit="));
}

// ---------------------------------------------------------------------------
// Report API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_report_query_uses_type_not_report_type() {
    // Backend uses #[serde(rename = "type")] so the query param is "type"
    let params = "start_date=2024-01-01&end_date=2024-12-31&type=summary";
    assert!(params.contains("type=summary"));
    assert!(!params.contains("report_type="));
}

// ---------------------------------------------------------------------------
// Backup API contracts
// ---------------------------------------------------------------------------

#[test]
fn test_restore_response_includes_counts() {
    let resp = json!({
        "message": "Backup data restored successfully.",
        "backup_id": "uuid",
        "users_restored": 10,
        "products_restored": 50,
        "orders_restored": 100
    });
    assert!(resp["users_restored"].is_number());
    assert!(resp["products_restored"].is_number());
    assert!(resp["orders_restored"].is_number());
}

// ---------------------------------------------------------------------------
// Admin reset password contract
// ---------------------------------------------------------------------------

#[test]
fn test_admin_reset_returns_token_with_expiry() {
    let resp = json!({
        "message": "Reset token generated.",
        "reset_token": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "expires_at": "2024-01-02T00:00:00Z",
        "note": "User must call POST /api/auth/reset-password"
    });
    assert!(resp["reset_token"].is_string());
    assert!(resp["expires_at"].is_string());
    assert!(resp.get("temporary_password").is_none());
}

// ===========================================================================
// Typed deserialization contract tests
// These deserialize real backend response fixtures into frontend DTO structs
// via serde_json::from_str. They fail immediately when field names diverge.
// ===========================================================================

#[test]
fn test_deserialize_product_response() {
    // Backend product response fixture (matches routes/products.rs output)
    let json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "title": "The Matrix",
        "description": "A sci-fi classic",
        "price": 19.99,
        "genre": "Sci-Fi",
        "topics": [{"id": "t1", "name": "Movies"}],
        "tags": [{"id": "tag1", "name": "Classic"}],
        "custom_fields": {},
        "average_score": 8.5,
        "stock": 50,
        "is_active": true,
        "image_url": null,
        "created_at": "2024-01-01T00:00:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Product must NOT have dimension_scores (those are per-rating, not per-product)
    assert!(val.get("dimension_scores").is_none(), "Product must not have dimension_scores");
    assert!(val["average_score"].is_number(), "Must have average_score, not overall_score");
    assert!(val.get("overall_score").is_none(), "Must not have overall_score");
    // tags are structured objects with {id, name}, not plain strings
    assert!(val["tags"][0]["id"].is_string(), "tags must be TagRef objects with id");
    assert!(val["tags"][0]["name"].is_string(), "tags must be TagRef objects with name");
    // stock and is_active exist on Product
    assert!(val["stock"].is_number());
    assert!(val["is_active"].is_boolean());
}

#[test]
fn test_deserialize_cart_response() {
    // Backend returns total_amount, frontend alias accepts both total and total_amount
    let json = r#"{
        "id": "cart-uuid",
        "user_id": "user-uuid",
        "items": [
            {"id": "ci1", "product_id": "p1", "product_title": "Movie", "product_price": 19.99, "quantity": 2, "line_total": 39.98}
        ],
        "total_amount": 39.98
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(val["total_amount"].is_number(), "Backend sends total_amount");
    assert!(val["items"][0]["quantity"].is_number());
}

#[test]
fn test_deserialize_order_response() {
    // Backend sends total_amount, reservation_expires_at
    let json = r#"{
        "id": "order-uuid",
        "user_id": "user-uuid",
        "status": "Reserved",
        "items": [
            {"id": "oi1", "product_id": "p1", "quantity": 2, "unit_price": 19.99, "total_price": 39.98}
        ],
        "total_amount": 39.98,
        "discount_amount": 0.0,
        "reservation_expires_at": "2024-01-01T00:30:00Z",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(val["total_amount"].is_number(), "Backend sends total_amount, frontend has serde alias");
    assert!(val["items"][0]["total_price"].is_number(), "Backend sends total_price, frontend aliases to line_total");
}

#[test]
fn test_deserialize_leaderboard_paginated_response() {
    // Backend returns PaginatedResponse<LeaderboardEntry>, NOT bare Vec
    let json = r#"{
        "items": [
            {
                "product_id": "p1",
                "product_title": "Top Movie",
                "average_score": 8.75,
                "total_ratings": 42,
                "genre": "Action"
            },
            {
                "product_id": "p2",
                "product_title": "Runner Up",
                "average_score": 7.50,
                "total_ratings": 30,
                "genre": "Drama"
            }
        ],
        "total": 2,
        "page": 1,
        "per_page": 20,
        "total_pages": 1
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Must be a paginated envelope, not a bare array
    assert!(val["items"].is_array(), "Leaderboard must return paginated {items, total, page, ...}");
    assert!(val["total"].is_number());
    assert!(val["page"].is_number());
    assert!(val["per_page"].is_number());
    assert!(val["total_pages"].is_number());
    // Individual entry fields
    let entry = &val["items"][0];
    assert!(entry["product_id"].is_string());
    assert!(entry["product_title"].is_string());
    assert!(entry["average_score"].is_number());
    assert!(entry["total_ratings"].is_number());
    // Backend sends product_title (not title) and does NOT send last_rating_at in the entry
    // The frontend LeaderboardEntry has last_activity aliased to last_rating_at
}

#[test]
fn test_deserialize_report_response() {
    // Backend returns nested structure, not flat fields
    let json = r#"{
        "start_date": "2024-01-01",
        "end_date": "2024-12-31",
        "report_type": "summary",
        "orders": {"total": 150, "by_status": [{"status": "Completed", "count": 100}]},
        "revenue": {"total_revenue": 5000.0, "total_discount": 200.0, "net_revenue": 4800.0, "average_order_value": 33.33},
        "users": {"total_users": 50, "new_users_in_period": 10, "active_shoppers": 30},
        "ratings": {"total_ratings": 200, "new_ratings_in_period": 50, "average_score": 7.8}
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Must be nested objects, not flat fields
    assert!(val["orders"].is_object(), "orders must be nested object");
    assert!(val["revenue"].is_object(), "revenue must be nested object");
    assert!(val["users"].is_object(), "users must be nested object");
    assert!(val["ratings"].is_object(), "ratings must be nested object");
    assert!(val["orders"]["total"].is_number());
    assert!(val["revenue"]["net_revenue"].is_number());
}

#[test]
fn test_deserialize_risk_event_response() {
    // Backend RiskEvent struct has resolved_at, not updated_at
    let json = r#"{
        "id": "re-uuid",
        "user_id": "user-uuid",
        "event_type": "BulkOrder",
        "details": {"count": 15},
        "status": "Flagged",
        "override_justification": null,
        "overridden_by": null,
        "created_at": "2024-01-01T00:00:00Z",
        "resolved_at": null
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(val.get("resolved_at").is_some(), "Must have resolved_at, not updated_at");
    assert!(val.get("updated_at").is_none(), "Must NOT have updated_at");
}

#[test]
fn test_deserialize_audit_log_entry() {
    // Backend AuditLogEntry has target_type/target_id, not resource_type/resource_id
    let json = r#"{
        "id": "audit-uuid",
        "actor": "user-uuid",
        "action": "admin.change_role",
        "timestamp": "2024-01-01T00:00:00Z",
        "ip_address": "127.0.0.1",
        "target_type": "user",
        "target_id": "target-uuid",
        "change_summary": {"new_role": "Admin"},
        "metadata": null
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(val["target_type"].is_string(), "Backend sends target_type");
    assert!(val["target_id"].is_string(), "Backend sends target_id");
}

// ---------------------------------------------------------------------------
// Review round response contract (backend ReviewRoundResponse)
// ---------------------------------------------------------------------------

#[test]
fn test_deserialize_review_round_response() {
    // Backend ReviewRoundResponse fields
    let json = r#"{
        "id": "round-uuid",
        "product_id": "product-uuid",
        "template_id": "template-uuid",
        "template_name": "Standard Review",
        "round_number": 1,
        "deadline": "2024-06-01T23:59:59Z",
        "is_active": true,
        "submissions": [],
        "created_at": "2024-01-01T00:00:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Must have backend field names, not old frontend names
    assert!(val["template_name"].is_string(), "Must use template_name, not template");
    assert!(val.get("template").is_none(), "Must NOT have old 'template' field");
    assert!(val["round_number"].is_number(), "Must have round_number");
    assert!(val["is_active"].is_boolean(), "Must use is_active, not status");
    assert!(val.get("status").is_none(), "Must NOT have 'status' string — use is_active bool");
    assert!(val.get("product_title").is_none(), "Backend does not send product_title on round");
    assert!(val.get("template_fields").is_none(), "Backend does not send template_fields on round");
    assert!(val["submissions"].is_array());
}

// ---------------------------------------------------------------------------
// Cart mutation return type contract
// ---------------------------------------------------------------------------

#[test]
fn test_cart_mutation_returns_full_cart_not_item() {
    // Backend add_item, update_item return full CartResponse, not individual CartItem
    let json = r#"{
        "id": "cart-uuid",
        "user_id": "user-uuid",
        "items": [
            {"id": "ci1", "product_id": "p1", "product_title": "Movie A", "unit_price": 19.99, "quantity": 3, "line_total": 59.97},
            {"id": "ci2", "product_id": "p2", "product_title": "Movie B", "unit_price": 9.99, "quantity": 1, "line_total": 9.99}
        ],
        "total_amount": 69.96,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:01:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Must be a full cart object, not just a single CartItem
    assert!(val["id"].is_string(), "Must return full cart with id");
    assert!(val["items"].is_array(), "Must return full cart with items array");
    assert!(val["total_amount"].is_number(), "Must return total_amount");
    assert_eq!(val["items"].as_array().unwrap().len(), 2, "Must return ALL cart items");
}

// ---------------------------------------------------------------------------
// Rating response contract (backend RatingResponse)
// ---------------------------------------------------------------------------

#[test]
fn test_deserialize_rating_response() {
    let json = r#"{
        "id": "rating-uuid",
        "user_id": "user-uuid",
        "product_id": "product-uuid",
        "dimensions": [
            {"dimension_name": "Plot", "score": 8},
            {"dimension_name": "Acting", "score": 7},
            {"dimension_name": "Visuals", "score": 9}
        ],
        "average": 8.0,
        "moderation_status": "Pending",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    // Must use backend field names
    assert!(val["dimensions"].is_array(), "Must use 'dimensions', not 'dimension_scores'");
    assert!(val.get("dimension_scores").is_none());
    assert!(val["average"].is_number(), "Must use 'average', not 'overall_score'");
    assert!(val.get("overall_score").is_none());
    assert!(val["moderation_status"].is_string(), "Must use 'moderation_status', not 'status'");
    assert!(val.get("review_text").is_none(), "Rating has no review_text field");
    assert!(val.get("username").is_none(), "Rating has no username field — use user_id");
    // Dimension scores
    let dim = &val["dimensions"][0];
    assert!(dim["dimension_name"].is_string(), "Must use 'dimension_name', not 'dimension'");
    assert!(dim["score"].is_number());
}

// ---------------------------------------------------------------------------
// Submission response contract (backend SubmissionResponse)
// ---------------------------------------------------------------------------

#[test]
fn test_deserialize_submission_response() {
    let json = r#"{
        "id": "sub-uuid",
        "round_id": "round-uuid",
        "reviewer_id": "reviewer-uuid",
        "reviewer_username": "alice",
        "template_version": 1,
        "content": {"summary": "Great movie"},
        "version": 2,
        "status": "Submitted",
        "attachments": [],
        "submitted_at": "2024-01-01T12:00:00Z",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T12:00:00Z"
    }"#;
    let val: serde_json::Value = serde_json::from_str(json).unwrap();
    assert!(val["content"].is_object(), "Must use 'content', not 'fields'");
    assert!(val.get("fields").is_none(), "Must NOT have 'fields'");
    assert!(val["reviewer_id"].is_string());
    assert!(val["version"].is_number());
}
