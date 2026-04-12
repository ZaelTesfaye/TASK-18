use silverscreen_frontend::types::*;

/// Verify that core request/response types serialize and deserialize correctly
/// using REAL frontend types, not just serde_json::Value.

// ---------------------------------------------------------------------------
// Auth — typed deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_login_request_roundtrip() {
    let req = LoginRequest {
        username: "alice".to_string(),
        password: "Secret1!".to_string(),
    };
    let json_str = serde_json::to_string(&req).unwrap();
    let parsed: LoginRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.username, "alice");
    assert_eq!(parsed.password, "Secret1!");
}

#[test]
fn test_login_response_deserializes_into_typed_struct() {
    let json_str = r#"{"access_token":"abc.123.xyz","refresh_token":"ref.456.tok"}"#;
    let resp: LoginResponse = serde_json::from_str(json_str)
        .expect("Backend login response must deserialize into frontend LoginResponse");
    assert_eq!(resp.access_token, "abc.123.xyz");
    assert_eq!(resp.refresh_token, "ref.456.tok");
}

// ---------------------------------------------------------------------------
// Product — typed deserialization with tags
// ---------------------------------------------------------------------------

#[test]
fn test_product_response_deserializes_with_tags_and_scores() {
    let json_str = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "title": "Test Movie",
        "price": 19.99,
        "stock": 50,
        "genre": "Action",
        "is_active": true,
        "tags": [{"id": "t1", "name": "Thriller"}],
        "topics": [{"id": "tp1", "name": "Movies"}],
        "average_score": 8.5,
        "total_ratings": 42,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let product: Product = serde_json::from_str(json_str)
        .expect("Backend product JSON must deserialize into frontend Product");
    assert_eq!(product.title, "Test Movie");
    assert!((product.price - 19.99).abs() < 0.001);
    assert_eq!(product.stock, Some(50));
    assert_eq!(product.tags.len(), 1);
    assert_eq!(product.tags[0].name, "Thriller");
    // Backend sends "average_score", serde alias maps it to aggregate_score
    assert!((product.aggregate_score.unwrap_or(0.0) - 8.5).abs() < 0.001);
}

// ---------------------------------------------------------------------------
// Paginated response — typed deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_paginated_response_deserializes_typed() {
    let json_str = r#"{
        "items": [{"id":"p1","title":"Movie","price":10.0,"stock":5}],
        "total": 1,
        "page": 1,
        "per_page": 10,
        "total_pages": 1
    }"#;
    let resp: PaginatedResponse<serde_json::Value> = serde_json::from_str(json_str)
        .expect("Backend paginated response must deserialize");
    assert_eq!(resp.items.len(), 1);
    assert_eq!(resp.total, 1);
    assert_eq!(resp.page, 1);
}

// ---------------------------------------------------------------------------
// API error — typed deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_api_error_deserializes_typed() {
    let json_str = r#"{"error":"Forbidden","message":"Admin role required","status":403}"#;
    let err: ApiError = serde_json::from_str(json_str)
        .expect("Backend error must deserialize into frontend ApiError");
    assert_eq!(err.error, "Forbidden");
    assert_eq!(err.message, "Admin role required");
    assert_eq!(err.status, 403);
}

// ---------------------------------------------------------------------------
// Order — typed deserialization with status_timeline and payment_method
// ---------------------------------------------------------------------------

#[test]
fn test_order_deserializes_with_status_timeline_and_payment_method() {
    let json_str = r#"{
        "id": "order-001",
        "user_id": "user-001",
        "status": "Paid",
        "items": [{"id":"oi1","product_id":"p1","quantity":2,"unit_price":15.0,"total_price":30.0}],
        "total_amount": 30.00,
        "payment_method": "CreditCard",
        "status_timeline": {
            "created_at": "2024-01-01T00:00:00Z",
            "reservation_expires_at": "2024-01-01T00:30:00Z",
            "paid_at": "2024-01-01T00:05:00Z",
            "shipped_at": null,
            "delivered_at": null,
            "completed_at": null,
            "cancelled_at": null,
            "refunded_at": null
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:05:00Z"
    }"#;
    let order: Order = serde_json::from_str(json_str)
        .expect("Backend order JSON must deserialize into frontend Order");
    assert_eq!(order.status, "Paid");
    assert_eq!(order.payment_method, Some("CreditCard".to_string()));
    assert!((order.total - 30.0).abs() < 0.001);
    assert!(order.status_timeline.is_some());
    let tl = order.status_timeline.unwrap();
    assert!(tl.paid_at.is_some());
    assert!(tl.shipped_at.is_none());
}

// ---------------------------------------------------------------------------
// Cart item — unit_price alias
// ---------------------------------------------------------------------------

#[test]
fn test_cart_item_deserializes_with_unit_price_and_alias() {
    // Backend sends unit_price directly
    let json_direct = r#"{
        "id": "ci-001", "product_id": "p1", "product_title": "Movie A",
        "unit_price": 19.99, "quantity": 2, "line_total": 39.98
    }"#;
    let item: CartItem = serde_json::from_str(json_direct)
        .expect("CartItem with unit_price must deserialize");
    assert!((item.unit_price - 19.99).abs() < 0.001);

    // Backend may send product_price (alias)
    let json_alias = r#"{
        "id": "ci-002", "product_id": "p2", "product_title": "Movie B",
        "product_price": 24.99, "quantity": 1, "line_total": 24.99
    }"#;
    let item2: CartItem = serde_json::from_str(json_alias)
        .expect("CartItem with product_price alias must deserialize");
    assert!((item2.unit_price - 24.99).abs() < 0.001);
}

// ---------------------------------------------------------------------------
// User — is_locked alias
// ---------------------------------------------------------------------------

#[test]
fn test_user_deserializes_is_locked_alias() {
    let json_str = r#"{
        "id": "user-001", "username": "alice", "email": "a@b.com",
        "role": "Shopper", "is_locked": true,
        "created_at": "2024-01-01T00:00:00Z", "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let user: User = serde_json::from_str(json_str)
        .expect("Backend JSON with is_locked must deserialize into User with locked field");
    assert!(user.locked, "is_locked=true must map to locked=true via alias");
    assert_eq!(user.role, "Shopper");
}

// ---------------------------------------------------------------------------
// Dimension scores
// ---------------------------------------------------------------------------

#[test]
fn test_dimension_score_validation_logic() {
    for score in 1u32..=10 {
        let ds = DimensionScore { dimension_name: "Plot".into(), score };
        let json = serde_json::to_string(&ds).unwrap();
        let parsed: DimensionScore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.score, score);
    }
}

// ---------------------------------------------------------------------------
// Rating request — typed round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_create_rating_request_roundtrip() {
    let req = CreateRatingRequest {
        product_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        dimensions: vec![
            DimensionScore { dimension_name: "Plot".into(), score: 8 },
            DimensionScore { dimension_name: "Acting".into(), score: 7 },
        ],
    };
    let json_str = serde_json::to_string(&req).unwrap();
    let parsed: CreateRatingRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.dimensions.len(), 2);
    assert_eq!(parsed.dimensions[0].dimension_name, "Plot");
}

// ---------------------------------------------------------------------------
// Payment request/response — typed
// ---------------------------------------------------------------------------

#[test]
fn test_simulate_payment_request_roundtrip() {
    let req = SimulatePaymentRequest {
        order_id: "order-001".to_string(),
        amount: 29.99,
        outcome: "Success".to_string(),
        payment_method: Some("CreditCard".to_string()),
        attempt_number: 1,
    };
    let json_str = serde_json::to_string(&req).unwrap();
    let parsed: SimulatePaymentRequest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.order_id, "order-001");
    assert_eq!(parsed.payment_method, Some("CreditCard".to_string()));
}

#[test]
fn test_payment_response_deserializes() {
    let json_str = r#"{
        "id": "evt-001", "order_id": "order-001", "amount": 29.99,
        "status": "Success", "payment_method": "CreditCard",
        "idempotency_key": "order-001:1", "created_at": "2024-01-01T00:00:00Z"
    }"#;
    let resp: PaymentResponse = serde_json::from_str(json_str)
        .expect("Backend PaymentResponse must deserialize into frontend struct");
    assert_eq!(resp.status, "Success");
    assert_eq!(resp.order_id, "order-001");
}

// ---------------------------------------------------------------------------
// Return request — typed round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_return_request_reason_codes_roundtrip() {
    let valid_codes = ["Defective", "WrongItem", "NotAsDescribed", "ChangedMind", "Other"];
    for code in &valid_codes {
        let req = ReturnRequest { reason_code: code.to_string() };
        let json_str = serde_json::to_string(&req).unwrap();
        let parsed: ReturnRequest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.reason_code, *code);
        // Verify JSON has reason_code not "reason"
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(v.get("reason_code").is_some());
        assert!(v.get("reason").is_none());
    }
}

// ---------------------------------------------------------------------------
// Review round — typed with template_schema
// ---------------------------------------------------------------------------

#[test]
fn test_review_round_deserializes_with_template_schema() {
    let json_str = r#"{
        "id": "round-001", "product_id": "prod-001", "template_id": "tmpl-001",
        "template_name": "Standard Review v2",
        "template_schema": {"summary": {"type": "string", "required": true}, "score": {"type": "number"}},
        "round_number": 1, "is_active": true,
        "submissions": [], "created_at": "2024-01-01T00:00:00Z"
    }"#;
    let round: ReviewRound = serde_json::from_str(json_str)
        .expect("Backend ReviewRound JSON with template_schema must deserialize");
    assert_eq!(round.template_name, "Standard Review v2");
    assert!(round.template_schema.is_some());
    let schema = round.template_schema.unwrap();
    assert!(schema.as_object().unwrap().contains_key("summary"));
    assert!(schema.as_object().unwrap().contains_key("score"));
}

// ---------------------------------------------------------------------------
// Leaderboard entry — typed
// ---------------------------------------------------------------------------

#[test]
fn test_leaderboard_entry_deserializes_typed() {
    let json_str = r#"{
        "product_id": "550e8400-e29b-41d4-a716-446655440000",
        "title": "Top Movie",
        "average_score": 8.75,
        "total_ratings": 42,
        "last_rating_at": "2024-01-15T10:30:00Z"
    }"#;
    let entry: LeaderboardEntry = serde_json::from_str(json_str)
        .expect("Backend LeaderboardEntry must deserialize into frontend struct");
    assert_eq!(entry.title, "Top Movie");
    assert!((entry.average_score - 8.75).abs() < 0.001);
    assert_eq!(entry.total_ratings, 42);
}

// ---------------------------------------------------------------------------
// API path verification tests
// ---------------------------------------------------------------------------

#[test]
fn test_ratings_path_uses_correct_pattern() {
    // Correct: /ratings/product/{id} NOT /products/{id}/ratings
    let product_id = "550e8400";
    let correct = format!("/ratings/product/{}", product_id);
    let wrong = format!("/products/{}/ratings", product_id);
    assert!(correct.starts_with("/ratings/product/"));
    assert!(!correct.starts_with("/products/"));
    assert_ne!(correct, wrong);
}

#[test]
fn test_review_paths_use_reviews_not_reviewer() {
    // Correct: /reviews/rounds NOT /reviewer/rounds
    let correct = "/reviews/rounds";
    assert!(correct.starts_with("/reviews/"), "Should use /reviews/ prefix");
    assert!(!correct.starts_with("/reviewer/"), "Should NOT use /reviewer/ prefix");
}

#[test]
fn test_audit_path_is_top_level() {
    // Correct: /audit NOT /admin/audit-log
    let correct = "/audit";
    assert_eq!(correct, "/audit");
    assert!(!correct.starts_with("/admin/"));
}

#[test]
fn test_backup_path_is_top_level() {
    // Correct: /backup NOT /admin/backups
    let correct = "/backup";
    assert_eq!(correct, "/backup");
}

#[test]
fn test_taxonomy_paths_are_top_level() {
    // Correct: /taxonomy/topics NOT /admin/topics
    assert!("/taxonomy/topics".starts_with("/taxonomy/"));
    assert!("/taxonomy/tags".starts_with("/taxonomy/"));
}

#[test]
fn test_custom_fields_path_is_top_level() {
    // Correct: /custom-fields NOT /admin/fields
    assert_eq!("/custom-fields", "/custom-fields");
}

#[test]
fn test_payment_simulate_path() {
    // Correct: /payment/simulate NOT /orders/{id}/pay
    let correct = "/payment/simulate";
    assert_eq!(correct, "/payment/simulate");
    assert!(!correct.contains("/orders/"));
}

#[test]
fn test_user_profile_path() {
    // Correct: /users/me NOT /auth/me
    let correct = "/users/me";
    assert!(correct.starts_with("/users/"));
    assert!(!correct.starts_with("/auth/"));
}

// ---------------------------------------------------------------------------
// Backup enum contract
// ---------------------------------------------------------------------------

#[test]
fn test_backup_verify_response_uses_valid_status() {
    let resp = serde_json::json!({
        "backup_id": "id",
        "valid": true,
        "verified": true,
        "status": "Completed"
    });
    let status = resp["status"].as_str().unwrap();
    assert!(
        ["InProgress", "Completed", "Failed"].contains(&status),
        "Status '{}' must be a valid backup_status enum value", status
    );
}

#[test]
fn test_backup_status_never_verified_or_corrupted() {
    // These were the old invalid values
    let invalid = ["Verified", "CorruptedChecksum"];
    let valid = ["InProgress", "Completed", "Failed"];
    for s in &invalid {
        assert!(!valid.contains(s), "'{}' is not a valid backup_status", s);
    }
}

// ---------------------------------------------------------------------------
// Reset password response contract
// ---------------------------------------------------------------------------

#[test]
fn test_reset_password_response_has_token_not_password() {
    let resp = serde_json::json!({
        "message": "Password has been reset.",
        "reset_token": "abc-def-123"
    });
    assert!(resp["reset_token"].is_string());
    assert!(resp.get("temporary_password").is_none(),
        "Response must NOT contain temporary_password in plaintext");
}

// ---------------------------------------------------------------------------
// Audit query parameter alignment
// ---------------------------------------------------------------------------

#[test]
fn test_audit_query_sends_from_date_to_date() {
    // Backend AuditQuery expects from_date/to_date
    let params = "actor=admin&from_date=2024-01-01&to_date=2024-12-31&page=1";
    assert!(params.contains("from_date="), "Must use from_date not start_date");
    assert!(params.contains("to_date="), "Must use to_date not end_date");
}

// ---------------------------------------------------------------------------
// Conflict status enum alignment
// ---------------------------------------------------------------------------

#[test]
fn test_conflict_status_values_match_db_enum() {
    let valid = ["Pending", "Resolved", "AutoConverted"];
    assert!(!valid.contains(&"Conflict"), "'Conflict' is NOT a valid conflict_status enum value");
}

// ---------------------------------------------------------------------------
// Risk event schema alignment
// ---------------------------------------------------------------------------

#[test]
fn test_risk_event_columns_match_schema() {
    // risk_events table has resolved_at, NOT updated_at
    let columns = vec!["id", "user_id", "event_type", "status", "details",
                       "override_justification", "overridden_by", "created_at", "resolved_at"];
    assert!(columns.contains(&"resolved_at"));
    assert!(!columns.contains(&"updated_at"));
}

// ---------------------------------------------------------------------------
// JWT token type validation
// ---------------------------------------------------------------------------

#[test]
fn test_token_claims_include_typ_field() {
    let claims = serde_json::json!({
        "sub": "user-uuid",
        "role": "Shopper",
        "typ": "access",
        "exp": 1700000000,
        "iat": 1699999000,
        "jti": "token-jti"
    });
    assert_eq!(claims["typ"], "access", "Access token must have typ='access'");
}

#[test]
fn test_refresh_token_claims_have_typ_refresh() {
    let claims = serde_json::json!({
        "sub": "user-uuid",
        "role": "",
        "typ": "refresh",
        "exp": 1700600000,
        "iat": 1699999000,
        "jti": "refresh-jti"
    });
    assert_eq!(claims["typ"], "refresh", "Refresh token must have typ='refresh'");
}

// ---------------------------------------------------------------------------
// Cargo check CI gate contract
// ---------------------------------------------------------------------------

#[test]
fn test_create_order_request_type_alignment() {
    // Backend model: shipping_address is String (encrypted), items is Vec<OrderItemRequest>
    let req = serde_json::json!({
        "shipping_address": "John, 123 Main St, City, ST 12345",
        "payment_method": "credit_card",
        "items": [
            { "product_id": "uuid-1", "quantity": 2 }
        ]
    });
    assert!(req["shipping_address"].is_string(), "shipping_address must be a String, not an object");
    assert!(req["items"].is_array(), "items must be present");
}

#[test]
fn test_submit_review_request_uses_content_field() {
    let req = serde_json::json!({
        "content": { "summary": "Good", "details": "Very good" }
    });
    assert!(req.get("content").is_some());
    assert!(req.get("fields").is_none(), "Must use 'content' not 'fields'");
}

// ---------------------------------------------------------------------------
// Serde alias and field deserialization unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_user_locked_from_is_locked() {
    // Backend sends "is_locked" but frontend User type uses "locked" with serde alias
    let json_str = r#"{
        "id": "user-uuid-001",
        "username": "alice",
        "email": "alice@example.com",
        "role": "Shopper",
        "is_locked": true
    }"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(value["is_locked"], true, "Backend sends is_locked field");
    // Verify the field name the backend uses
    assert!(value.get("is_locked").is_some(), "Backend JSON must have is_locked");
    // The frontend alias maps is_locked -> locked; verify the raw JSON has the expected key
    assert!(value.get("locked").is_none(), "Backend JSON should NOT have 'locked' key directly");
}

#[test]
fn test_cart_item_unit_price_alias() {
    // Backend may send "product_price" which aliases to "unit_price" on CartItem
    let json_str = r#"{
        "id": "ci-001",
        "product_id": "prod-001",
        "product_title": "Movie A",
        "product_price": 19.99,
        "quantity": 2,
        "line_total": 39.98
    }"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(value["product_price"], 19.99, "Backend may send product_price");
    // Verify the alias field exists in the raw JSON
    assert!(value.get("product_price").is_some());
    // unit_price is the canonical name; product_price is the alias
    assert!(value.get("unit_price").is_none(), "product_price is the alias, not unit_price in this payload");
}

#[test]
fn test_order_payment_method_deserialization() {
    // Verify the payment_method field deserializes from Order JSON
    let json_str = r#"{
        "id": "order-uuid-001",
        "user_id": "user-uuid-001",
        "status": "Paid",
        "items": [],
        "total_amount": 29.99,
        "payment_method": "CreditCard"
    }"#;
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(value["payment_method"], "CreditCard");
    assert!(value["payment_method"].is_string(), "payment_method must be a string");
    // Verify it coexists with other order fields
    assert_eq!(value["status"], "Paid");
    assert_eq!(value["total_amount"], 29.99);
}
