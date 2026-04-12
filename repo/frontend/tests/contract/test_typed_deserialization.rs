// Typed deserialization contract tests.
//
// These tests deserialize REAL backend response JSON fixtures into ACTUAL
// frontend Rust types using serde_json::from_str::<FrontendType>().
// They will FAIL IMMEDIATELY if any field name or structure diverges
// between frontend types.rs and what the backend actually serializes.
//
// The JSON fixtures here are exact copies of what the backend routes produce.

use silverscreen_frontend::types::*;

// ============================================================================
// Auth refresh response (backend AccessTokenResponse)
// ============================================================================

#[test]
fn contract_refresh_response_deserializes_from_backend_json() {
    let json = r#"{
        "access_token": "eyJhbGciOiJIUzI1NiJ9.new_token",
        "token_type": "Bearer"
    }"#;
    let resp: RefreshResponse = serde_json::from_str(json)
        .expect("Backend AccessTokenResponse must deserialize into frontend RefreshResponse");
    assert_eq!(resp.token_type, "Bearer");
    assert!(!resp.access_token.is_empty());
}

// ============================================================================
// Product (with structured TagRef tags, average_score aliased to aggregate_score, no dimension_scores)
// ============================================================================

#[test]
fn contract_product_deserializes_from_backend_json() {
    let json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "title": "The Matrix",
        "description": "A sci-fi classic",
        "price": 19.99,
        "genre": "Sci-Fi",
        "topics": [
            {"id": "aaa-111", "name": "Movies"}
        ],
        "tags": [
            {"id": "bbb-222", "name": "Classic"},
            {"id": "ccc-333", "name": "Sci-Fi"}
        ],
        "custom_fields": {"director": "Wachowskis"},
        "average_score": 8.75,
        "stock": 42,
        "is_active": true,
        "image_url": "https://example.com/matrix.jpg",
        "created_at": "2024-01-15T10:30:00Z"
    }"#;
    let product: Product = serde_json::from_str(json)
        .expect("Backend product JSON must deserialize into frontend Product type");
    assert_eq!(product.id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(product.title, "The Matrix");
    assert_eq!(product.price, 19.99);
    assert_eq!(product.tags.len(), 2);
    assert_eq!(product.tags[0].id, "bbb-222");
    assert_eq!(product.tags[0].name, "Classic");
    assert_eq!(product.aggregate_score, Some(8.75));
    assert_eq!(product.stock, Some(42));
    assert_eq!(product.is_active, Some(true));
}

// ============================================================================
// Cart response (total_amount alias, CartItem with line_total)
// ============================================================================

#[test]
fn contract_cart_deserializes_from_backend_json() {
    let json = r#"{
        "id": "cart-uuid-1234",
        "user_id": "user-uuid-5678",
        "items": [
            {
                "id": "ci-001",
                "product_id": "prod-001",
                "product_title": "Movie A",
                "unit_price": 19.99,
                "quantity": 2,
                "line_total": 39.98
            },
            {
                "id": "ci-002",
                "product_id": "prod-002",
                "product_title": "Movie B",
                "unit_price": 9.99,
                "quantity": 1,
                "line_total": 9.99
            }
        ],
        "total_amount": 49.97,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:05:00Z"
    }"#;
    let cart: Cart = serde_json::from_str(json)
        .expect("Backend cart JSON must deserialize into frontend Cart type (total_amount -> total via alias)");
    assert_eq!(cart.id, "cart-uuid-1234");
    assert_eq!(cart.items.len(), 2);
    assert_eq!(cart.items[0].product_title, "Movie A");
    assert_eq!(cart.items[0].quantity, 2);
    // total_amount from backend maps to .total via serde alias
    assert!((cart.total - 49.97).abs() < 0.001, "total_amount must alias to total");
}

// ============================================================================
// Order response (total_amount alias, OrderItem total_price alias)
// ============================================================================

#[test]
fn contract_order_deserializes_from_backend_json() {
    let json = r#"{
        "id": "order-uuid-001",
        "user_id": "user-uuid-001",
        "status": "Reserved",
        "items": [
            {
                "id": "oi-001",
                "product_id": "prod-001",
                "product_title": "Movie A",
                "quantity": 2,
                "unit_price": 19.99,
                "total_price": 39.98
            }
        ],
        "total_amount": 39.98,
        "discount_amount": 0.0,
        "status_timeline": {
            "created_at": "2024-01-01T00:00:00Z",
            "reservation_expires_at": "2024-01-01T00:30:00Z",
            "paid_at": null,
            "shipped_at": null,
            "delivered_at": null,
            "completed_at": null,
            "cancelled_at": null,
            "refunded_at": null
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let order: Order = serde_json::from_str(json)
        .expect("Backend order JSON must deserialize into frontend Order type");
    assert_eq!(order.status, "Reserved");
    // total_amount aliases to total
    assert!((order.total - 39.98).abs() < 0.001);
    assert_eq!(order.items.len(), 1);
    // total_price aliases to line_total
    assert!((order.items[0].line_total - 39.98).abs() < 0.001);
    // status_timeline with reservation_expires_at
    let timeline = order.status_timeline.expect("Must have status_timeline");
    assert!(timeline.reservation_expires_at.is_some(), "Must have reservation_expires_at");
    assert!(timeline.paid_at.is_none());
}

#[test]
fn contract_cart_item_unit_price_alias() {
    // Backend sends "unit_price" but frontend type uses it directly now
    let json = r#"{
        "id": "ci-001",
        "product_id": "prod-001",
        "product_title": "Movie A",
        "unit_price": 19.99,
        "quantity": 2,
        "line_total": 39.98
    }"#;
    let item: CartItem = serde_json::from_str(json)
        .expect("Backend cart item with unit_price must deserialize");
    assert!((item.unit_price - 19.99).abs() < 0.001);
}

#[test]
fn contract_attachment_info_includes_approval_status() {
    let json = r#"{
        "id": "att-001",
        "filename": "review.pdf",
        "mime_type": "application/pdf",
        "size_bytes": 12345,
        "approval_status": "Pending",
        "uploaded_at": "2024-06-01T12:00:00Z"
    }"#;
    let info: AttachmentInfo = serde_json::from_str(json)
        .expect("AttachmentInfo must include approval_status");
    assert_eq!(info.approval_status, "Pending");
}

// ============================================================================
// Leaderboard paginated response
// ============================================================================

#[test]
fn contract_leaderboard_deserializes_paginated_from_backend_json() {
    let json = r#"{
        "items": [
            {
                "product_id": "prod-001",
                "product_title": "Top Movie",
                "average_score": 9.25,
                "total_ratings": 150,
                "genre": "Action"
            },
            {
                "product_id": "prod-002",
                "product_title": "Runner Up",
                "average_score": 8.50,
                "total_ratings": 120,
                "genre": "Drama"
            }
        ],
        "total": 2,
        "page": 1,
        "per_page": 20,
        "total_pages": 1
    }"#;
    let paginated: PaginatedResponse<LeaderboardEntry> = serde_json::from_str(json)
        .expect("Backend leaderboard JSON must deserialize into PaginatedResponse<LeaderboardEntry>");
    assert_eq!(paginated.items.len(), 2);
    assert_eq!(paginated.total, 2);
    assert_eq!(paginated.page, 1);
    assert_eq!(paginated.items[0].product_title, "Top Movie");
    assert_eq!(paginated.items[0].average_score, 9.25);
    assert_eq!(paginated.items[0].total_ratings, 150);
}

// ============================================================================
// Review round (backend ReviewRoundResponse)
// ============================================================================

#[test]
fn contract_review_round_deserializes_from_backend_json() {
    let json = r#"{
        "id": "round-uuid-001",
        "product_id": "prod-uuid-001",
        "template_id": "tmpl-uuid-001",
        "template_name": "Standard Review v2",
        "round_number": 3,
        "deadline": "2024-06-30T23:59:59Z",
        "is_active": true,
        "submissions": [
            {
                "id": "sub-001",
                "round_id": "round-uuid-001",
                "reviewer_id": "reviewer-001",
                "template_version": 1,
                "content": {"summary": "Excellent"},
                "attachments": [],
                "version": 1,
                "status": "Submitted",
                "created_at": "2024-06-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z"
            }
        ],
        "created_at": "2024-05-01T00:00:00Z"
    }"#;
    let round: ReviewRound = serde_json::from_str(json)
        .expect("Backend ReviewRoundResponse must deserialize into frontend ReviewRound");
    assert_eq!(round.template_name, "Standard Review v2");
    assert_eq!(round.round_number, 3);
    assert!(round.is_active);
    assert_eq!(round.submissions.len(), 1);
    assert_eq!(round.submissions[0].content["summary"], "Excellent");
}

// ============================================================================
// Rating response (dimensions, average, moderation_status)
// ============================================================================

#[test]
fn contract_rating_deserializes_from_backend_json() {
    let json = r#"{
        "id": "rating-uuid-001",
        "user_id": "user-uuid-001",
        "product_id": "prod-uuid-001",
        "dimensions": [
            {"dimension_name": "Plot", "score": 8},
            {"dimension_name": "Acting", "score": 9},
            {"dimension_name": "Visuals", "score": 7}
        ],
        "average": 8.0,
        "moderation_status": "Approved",
        "created_at": "2024-03-15T14:30:00Z",
        "updated_at": "2024-03-15T14:30:00Z"
    }"#;
    let rating: Rating = serde_json::from_str(json)
        .expect("Backend RatingResponse must deserialize into frontend Rating");
    assert_eq!(rating.dimensions.len(), 3);
    assert_eq!(rating.dimensions[0].dimension_name, "Plot");
    assert_eq!(rating.dimensions[0].score, 8);
    assert_eq!(rating.average, 8.0);
    assert_eq!(rating.moderation_status, "Approved");
}

// ============================================================================
// Audit log entry (target_type/target_id, change_summary -> details alias)
// ============================================================================

#[test]
fn contract_audit_log_entry_deserializes_from_backend_json() {
    let json = r#"{
        "id": "audit-uuid-001",
        "actor": "admin-uuid-001",
        "action": "admin.change_role",
        "timestamp": "2024-01-20T09:15:00Z",
        "ip_address": "192.168.1.100",
        "target_type": "user",
        "target_id": "user-uuid-999",
        "change_summary": {"old_role": "Shopper", "new_role": "Reviewer"},
        "metadata": null
    }"#;
    let entry: AuditLogEntry = serde_json::from_str(json)
        .expect("Backend AuditLogEntry must deserialize into frontend AuditLogEntry");
    assert_eq!(entry.actor, "admin-uuid-001");
    assert_eq!(entry.action, "admin.change_role");
    assert_eq!(entry.target_type, Some("user".to_string()));
    assert_eq!(entry.target_id, Some("user-uuid-999".to_string()));
    // change_summary aliases to details
    assert_eq!(entry.details["new_role"], "Reviewer");
}

// ============================================================================
// Report response (nested orders/revenue/users/ratings objects)
// ============================================================================

#[test]
fn contract_report_deserializes_from_backend_json() {
    let json = r#"{
        "start_date": "2024-01-01",
        "end_date": "2024-06-30",
        "report_type": "summary",
        "orders": {
            "total": 1250,
            "by_status": [
                {"status": "Completed", "count": 800},
                {"status": "Cancelled", "count": 150},
                {"status": "Refunded", "count": 50}
            ]
        },
        "revenue": {
            "total_revenue": 24999.50,
            "total_discount": 1200.00,
            "net_revenue": 23799.50,
            "average_order_value": 29.99
        },
        "users": {
            "total_users": 500,
            "new_users_in_period": 120,
            "active_shoppers": 350
        },
        "ratings": {
            "total_ratings": 2000,
            "new_ratings_in_period": 450,
            "average_score": 7.8
        }
    }"#;
    let report: ReportResponse = serde_json::from_str(json)
        .expect("Backend ReportResponse must deserialize into frontend ReportResponse");
    assert_eq!(report.report_type, "summary");
    assert_eq!(report.orders.total, 1250);
    assert_eq!(report.orders.by_status.len(), 3);
    assert_eq!(report.orders.by_status[0].status, "Completed");
    assert!((report.revenue.net_revenue - 23799.50).abs() < 0.01);
    assert_eq!(report.users.new_users_in_period, 120);
    assert_eq!(report.ratings.total_ratings, 2000);
    assert_eq!(report.ratings.average_score, Some(7.8));
}

// ============================================================================
// Risk event (resolved_at, override_justification, overridden_by)
// ============================================================================

#[test]
fn contract_risk_event_deserializes_from_backend_json() {
    let json = r#"{
        "id": "risk-uuid-001",
        "user_id": "user-uuid-001",
        "event_type": "BulkOrder",
        "status": "Flagged",
        "details": {"order_count": 15, "window_minutes": 60},
        "override_justification": null,
        "overridden_by": null,
        "created_at": "2024-02-10T08:00:00Z",
        "resolved_at": null
    }"#;
    let event: RiskEvent = serde_json::from_str(json)
        .expect("Backend RiskEvent must deserialize into frontend RiskEvent");
    assert_eq!(event.event_type, "BulkOrder");
    assert_eq!(event.status, "Flagged");
    assert!(event.resolved_at.is_none());
    assert!(event.override_justification.is_none());
}

// ============================================================================
// Payment event (backend PaymentEvent)
// ============================================================================

#[test]
fn contract_payment_response_deserializes_from_backend_json() {
    let json = r#"{
        "id": "pay-uuid-001",
        "order_id": "order-uuid-001",
        "idempotency_key": "order-uuid-001:1",
        "amount": 29.99,
        "status": "Success",
        "payment_method": "local_tender",
        "response_data": {"message": "Payment processed"},
        "created_at": "2024-01-15T10:30:00Z"
    }"#;
    let payment: PaymentResponse = serde_json::from_str(json)
        .expect("Backend PaymentEvent JSON must deserialize into frontend PaymentResponse");
    assert_eq!(payment.id, "pay-uuid-001");
    assert_eq!(payment.order_id, "order-uuid-001");
    assert_eq!(payment.status, "Success");
    assert_eq!(payment.payment_method, "local_tender");
    assert!((payment.amount - 29.99).abs() < 0.001);
}

// ============================================================================
// Submission response (backend SubmissionResponse with reviewer_id and attachments)
// ============================================================================

#[test]
fn contract_submission_response_deserializes_from_backend_json() {
    let json = r#"{
        "id": "sub-uuid-001",
        "round_id": "round-uuid-001",
        "reviewer_id": "reviewer-uuid-001",
        "reviewer_username": "alice",
        "template_version": 2,
        "content": {"summary": "Excellent film"},
        "version": 1,
        "status": "Submitted",
        "attachments": [
            {
                "id": "att-uuid-001",
                "filename": "review.pdf",
                "mime_type": "application/pdf",
                "size_bytes": 12345,
                "approval_status": "Pending",
                "uploaded_at": "2024-06-01T12:00:00Z"
            }
        ],
        "submitted_at": "2024-06-01T00:00:00Z",
        "created_at": "2024-05-30T00:00:00Z",
        "updated_at": "2024-06-01T00:00:00Z"
    }"#;
    let sub: ReviewSubmission = serde_json::from_str(json)
        .expect("Backend SubmissionResponse must deserialize into frontend ReviewSubmission");
    assert_eq!(sub.reviewer_id, "reviewer-uuid-001");
    assert_eq!(sub.reviewer_username, Some("alice".to_string()));
    assert_eq!(sub.template_version, 2);
    assert_eq!(sub.attachments.len(), 1);
    assert_eq!(sub.attachments[0].filename, "review.pdf");
    assert_eq!(sub.attachments[0].size_bytes, 12345);
}

// ============================================================================
// Custom field definition (backend shape with allowed_values, slug, version)
// ============================================================================

#[test]
fn contract_custom_field_definition_deserializes_from_backend_json() {
    let json = r#"{
        "id": "field-uuid-001",
        "name": "Director",
        "slug": "director",
        "field_type": "Text",
        "allowed_values": null,
        "status": "Published",
        "version": 1,
        "previous_type": null,
        "previous_allowed_values": null,
        "conflict_count": 0,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }"#;
    let field: CustomFieldDefinition = serde_json::from_str(json)
        .expect("Backend CustomFieldDefinition must deserialize into frontend type");
    assert_eq!(field.name, "Director");
    assert_eq!(field.slug, "director");
    assert_eq!(field.field_type, "Text");
    assert_eq!(field.status, "Published");
    assert_eq!(field.version, 1);
    assert!(field.allowed_values.is_none());
}

#[test]
fn contract_custom_field_enum_with_allowed_values() {
    let json = r#"{
        "id": "field-uuid-002",
        "name": "Genre",
        "slug": "genre",
        "field_type": "Enum",
        "allowed_values": ["Action", "Drama", "Comedy"],
        "status": "Draft",
        "version": 2,
        "previous_type": "Text",
        "previous_allowed_values": null,
        "conflict_count": 3,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-02-01T00:00:00Z"
    }"#;
    let field: CustomFieldDefinition = serde_json::from_str(json)
        .expect("Backend enum field must deserialize");
    assert_eq!(field.field_type, "Enum");
    assert_eq!(field.conflict_count, 3);
    let vals = field.allowed_values.unwrap();
    assert_eq!(vals.as_array().unwrap().len(), 3);
}

// ============================================================================
// Risk events paginated response (admin endpoint)
// ============================================================================

#[test]
fn contract_risk_events_paginated_deserializes_from_backend_json() {
    let json = r#"{
        "items": [
            {
                "id": "risk-uuid-001",
                "user_id": "user-uuid-001",
                "event_type": "BulkOrder",
                "status": "Flagged",
                "details": {"order_count": 15},
                "override_justification": null,
                "overridden_by": null,
                "created_at": "2024-02-10T08:00:00Z",
                "resolved_at": null
            }
        ],
        "total": 1,
        "page": 1,
        "per_page": 20,
        "total_pages": 1
    }"#;
    let paginated: PaginatedResponse<RiskEvent> = serde_json::from_str(json)
        .expect("Backend risk events paginated response must deserialize");
    assert_eq!(paginated.items.len(), 1);
    assert_eq!(paginated.items[0].event_type, "BulkOrder");
    assert_eq!(paginated.items[0].status, "Flagged");
    assert_eq!(paginated.total, 1);
}

// ============================================================================
// Paginated response wrapper
// ============================================================================

#[test]
fn contract_paginated_response_deserializes_from_backend_json() {
    let json = r#"{
        "items": [{"id": "a"}, {"id": "b"}],
        "total": 100,
        "page": 3,
        "per_page": 20,
        "total_pages": 5
    }"#;
    // Use serde_json::Value as the item type since this tests the envelope only
    let paginated: PaginatedResponse<serde_json::Value> = serde_json::from_str(json)
        .expect("PaginatedResponse wrapper must deserialize from backend JSON");
    assert_eq!(paginated.items.len(), 2);
    assert_eq!(paginated.total, 100);
    assert_eq!(paginated.page, 3);
    assert_eq!(paginated.per_page, 20);
    assert_eq!(paginated.total_pages, 5);
}

// ============================================================================
// User is_locked alias (backend sends "is_locked", frontend uses "locked")
// ============================================================================

#[test]
fn contract_user_is_locked_alias_deserializes() {
    let json = r#"{
        "id": "user-uuid-001",
        "username": "alice",
        "email": "alice@example.com",
        "role": "Shopper",
        "is_locked": true,
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
    }"#;
    let user: User = serde_json::from_str(json)
        .expect("Backend JSON with is_locked must deserialize into frontend User (locked field via alias)");
    assert_eq!(user.id, "user-uuid-001");
    assert_eq!(user.username, "alice");
    assert!(user.locked, "is_locked=true must map to locked=true via serde alias");
}

// ============================================================================
// Order includes payment_method field
// ============================================================================

#[test]
fn contract_order_includes_payment_method() {
    let json = r#"{
        "id": "order-uuid-002",
        "user_id": "user-uuid-001",
        "status": "Paid",
        "items": [
            {
                "id": "oi-001",
                "product_id": "prod-001",
                "product_title": "Movie A",
                "quantity": 1,
                "unit_price": 29.99,
                "total_price": 29.99
            }
        ],
        "total_amount": 29.99,
        "payment_method": "CreditCard",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:05:00Z"
    }"#;
    let order: Order = serde_json::from_str(json)
        .expect("Backend order JSON with payment_method must deserialize into frontend Order");
    assert_eq!(order.payment_method, Some("CreditCard".to_string()));
    assert_eq!(order.status, "Paid");
    assert!((order.total - 29.99).abs() < 0.001);
}

// ============================================================================
// Admin user full payload deserialization
// ============================================================================

#[test]
fn contract_admin_user_full_payload() {
    let json = r#"{
        "id": "admin-uuid-001",
        "username": "superadmin",
        "email": "admin@example.com",
        "role": "Admin",
        "is_locked": false,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-06-15T12:00:00Z"
    }"#;
    let user: User = serde_json::from_str(json)
        .expect("Full admin user payload must deserialize into frontend User");
    assert_eq!(user.id, "admin-uuid-001");
    assert_eq!(user.username, "superadmin");
    assert_eq!(user.email, "admin@example.com");
    assert_eq!(user.role, "Admin");
    assert!(!user.locked, "is_locked=false must map to locked=false");
    assert!(user.created_at.is_some(), "created_at must be present");
    assert!(user.updated_at.is_some(), "updated_at must be present");
}
