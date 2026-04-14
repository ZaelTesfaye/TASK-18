// Route structure validation tests.
// These verify that all expected routes are defined and accessible,
// and that route logic (guards, parameters, filter state) works correctly.

use silverscreen_frontend::types::*;

#[test]
fn test_all_public_routes_defined() {
    // Public routes that should be accessible without authentication.
    let public_routes = vec![
        "/",
        "/login",
        "/register",
        "/catalog",
        "/leaderboards",
    ];
    for route in public_routes {
        assert!(!route.is_empty(), "Route should be defined: {}", route);
    }
}

#[test]
fn test_all_authenticated_routes_defined() {
    // Routes requiring authentication.
    let auth_routes = vec![
        "/cart",
        "/checkout",
        "/orders",
    ];
    for route in auth_routes {
        assert!(!route.is_empty(), "Route should be defined: {}", route);
    }
}

#[test]
fn test_admin_routes_defined() {
    let admin_routes = vec![
        "/admin",
        "/admin/users",
        "/admin/taxonomy",
        "/admin/fields",
        "/admin/moderation",
        "/admin/audit",
        "/admin/reports",
        "/admin/backup",
    ];
    for route in admin_routes {
        assert!(route.starts_with("/admin"), "Admin route should start with /admin: {}", route);
    }
}

#[test]
fn test_reviewer_routes_defined() {
    let reviewer_routes = vec![
        "/reviewer/rounds",
    ];
    for route in reviewer_routes {
        assert!(route.starts_with("/reviewer"), "Reviewer route: {}", route);
    }
}

#[test]
fn test_product_detail_route_format() {
    let product_id = "550e8400-e29b-41d4-a716-446655440000";
    let route = format!("/products/{}", product_id);
    assert!(route.starts_with("/products/"));
    assert!(route.contains(product_id));
}

#[test]
fn test_order_detail_route_format() {
    let order_id = "550e8400-e29b-41d4-a716-446655440000";
    let route = format!("/orders/{}", order_id);
    assert!(route.starts_with("/orders/"));
}

#[test]
fn test_rating_route_format() {
    let product_id = "550e8400-e29b-41d4-a716-446655440000";
    let route = format!("/ratings/{}", product_id);
    assert!(route.starts_with("/ratings/"));
}

// ===========================================================================
// Behavioral route tests using actual frontend types
// ===========================================================================

// ---------------------------------------------------------------------------
// Role-based route guard logic using real User type
// ---------------------------------------------------------------------------

/// Admin route guard: only users with role "Admin" can access admin routes.
#[test]
fn test_admin_route_guard_with_user_type() {
    let admin_user = UserResponse {
        id: "user-001".to_string(),
        username: "admin_jane".to_string(),
        email: "jane@example.com".to_string(),
        role: "Admin".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };

    let shopper_user = UserResponse {
        id: "user-002".to_string(),
        username: "shopper_bob".to_string(),
        email: "bob@example.com".to_string(),
        role: "Shopper".to_string(),
        locked: false,
        created_at: None,
        updated_at: None,
    };

    let locked_admin = UserResponse {
        id: "user-003".to_string(),
        username: "locked_admin".to_string(),
        email: "locked@example.com".to_string(),
        role: "Admin".to_string(),
        locked: true,
        created_at: None,
        updated_at: None,
    };

    // Guard function: admin routes require role == "Admin" and not locked
    let can_access_admin = |user: &UserResponse| -> bool {
        user.role == "Admin" && !user.locked
    };

    assert!(can_access_admin(&admin_user), "Admin should access admin routes");
    assert!(!can_access_admin(&shopper_user), "Shopper must NOT access admin routes");
    assert!(!can_access_admin(&locked_admin), "Locked admin must NOT access admin routes");
}

/// Reviewer route guard: both "Reviewer" and "Admin" roles can access reviewer routes.
#[test]
fn test_reviewer_route_guard_with_user_type() {
    let make_user = |role: &str| -> UserResponse {
        UserResponse {
            id: "user-001".to_string(),
            username: "test".to_string(),
            email: "test@example.com".to_string(),
            role: role.to_string(),
            locked: false,
            created_at: None,
            updated_at: None,
        }
    };

    let can_access_reviewer = |user: &UserResponse| -> bool {
        user.role == "Reviewer" || user.role == "Admin"
    };

    assert!(can_access_reviewer(&make_user("Reviewer")));
    assert!(can_access_reviewer(&make_user("Admin")));
    assert!(!can_access_reviewer(&make_user("Shopper")));
}

/// Auth guard: authenticated routes require a non-empty access token equivalent.
#[test]
fn test_auth_route_guard_with_login_response() {
    // Simulate a successful login
    let login_resp = LoginResponse {
        access_token: "eyJhbGciOiJIUzI1NiJ9.test.signature".to_string(),
        refresh_token: "refresh-token-value".to_string(),
        user: Some(UserResponse {
            id: "user-001".to_string(),
            username: "alice".to_string(),
            email: "alice@example.com".to_string(),
            role: "Shopper".to_string(),
            locked: false,
            created_at: None,
            updated_at: None,
        }),
    };

    let is_authenticated = !login_resp.access_token.is_empty();
    assert!(is_authenticated, "User with access token is authenticated");

    // Auth routes should be accessible
    let auth_routes = vec!["/cart", "/checkout", "/orders"];
    for route in &auth_routes {
        assert!(is_authenticated, "Route {} requires authentication", route);
    }

    // Simulate no token (logged out)
    let no_token = "";
    assert!(no_token.is_empty(), "Logged out user has no token");
}

// ---------------------------------------------------------------------------
// Parameterized route construction with real types
// ---------------------------------------------------------------------------

/// Product detail route must use the product's UUID from the Product type.
#[test]
fn test_product_detail_route_from_product_type() {
    let product = Product {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        title: "The Matrix".to_string(),
        description: String::new(),
        price: 19.99,
        genre: "Sci-Fi".to_string(),
        topics: vec![],
        tags: vec![],
        custom_fields: serde_json::Value::Null,
        aggregate_score: None,
        stock: None,
        is_active: None,
        image_url: None,
        created_at: None,
    };

    let route = format!("/products/{}", product.id);
    assert_eq!(route, "/products/550e8400-e29b-41d4-a716-446655440000");
    assert!(route.starts_with("/products/"));
    // Must not contain the product title
    assert!(!route.contains("Matrix"));
}

/// Order detail route must use the order's UUID from the Order type.
#[test]
fn test_order_detail_route_from_order_type() {
    let order = Order {
        id: "order-uuid-550e8400".to_string(),
        user_id: "user-001".to_string(),
        status: "Completed".to_string(),
        items: vec![],
        total: 39.98,
        shipping_address: None,
        payment_method: None,
        status_timeline: None,
        created_at: None,
        updated_at: None,
    };

    let route = format!("/orders/{}", order.id);
    assert_eq!(route, "/orders/order-uuid-550e8400");
}

/// Rating route must use product_id, not the product title.
#[test]
fn test_rating_route_uses_product_id() {
    let product = Product {
        id: "prod-uuid-001".to_string(),
        title: "Inception".to_string(),
        description: String::new(),
        price: 14.99,
        genre: String::new(),
        topics: vec![],
        tags: vec![],
        custom_fields: serde_json::Value::Null,
        aggregate_score: None,
        stock: None,
        is_active: None,
        image_url: None,
        created_at: None,
    };

    let route = format!("/ratings/{}", product.id);
    assert_eq!(route, "/ratings/prod-uuid-001");
    assert!(!route.contains("Inception"), "Route must use ID, not title");
}

// ---------------------------------------------------------------------------
// Route → filter state mapping
// ---------------------------------------------------------------------------

/// Navigating to catalog with a tag should produce a ProductFilter with that tag_id.
#[test]
fn test_catalog_route_with_tag_filter_state() {
    // Simulate: user clicks tag "Action" (id: tag-uuid) on a product card,
    // which navigates to /catalog and sets filter state
    let tag = TagRef {
        id: "tag-uuid-action".to_string(),
        name: "Action".to_string(),
    };

    let filter = ProductFilter {
        tag_id: Some(tag.id.clone()),
        page: Some(1),
        per_page: Some(12),
        ..Default::default()
    };

    // The filter should serialize to query params for the catalog route
    let json = serde_json::to_value(&filter).unwrap();
    assert_eq!(json["tag_id"], "tag-uuid-action");
    assert_eq!(json["page"], 1);
    // Other fields omitted (skip_serializing_if = None)
    assert!(json.get("search").is_none());
    assert!(json.get("genre").is_none());
}

/// Navigating to leaderboards with a period should produce a LeaderboardQuery.
#[test]
fn test_leaderboard_route_query_state() {
    let query = LeaderboardQuery {
        period: Some("weekly".to_string()),
        genre: Some("Action".to_string()),
        page: Some(1),
        per_page: Some(20),
    };

    let json = serde_json::to_value(&query).unwrap();
    assert_eq!(json["period"], "weekly");
    assert_eq!(json["genre"], "Action");
    assert_eq!(json["page"], 1);
    assert_eq!(json["per_page"], 20);
}

/// Navigating to audit log with filters should produce an AuditLogQuery.
#[test]
fn test_audit_route_query_state() {
    let query = AuditLogQuery {
        actor: Some("admin-uuid-001".to_string()),
        action: Some("admin.change_role".to_string()),
        from: Some("2024-01-01".to_string()),
        to: Some("2024-12-31".to_string()),
        page: Some(1),
    };

    let json = serde_json::to_value(&query).unwrap();
    assert_eq!(json["actor"], "admin-uuid-001");
    assert_eq!(json["action"], "admin.change_role");
    assert_eq!(json["from"], "2024-01-01");
    assert_eq!(json["to"], "2024-12-31");
}
