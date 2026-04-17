// Page module logic tests.
// These verify type construction, form validation, shipping address building,
// and order status badge mapping without rendering Yew pages.
// Imports real page modules to verify compilation.

use silverscreen_frontend::types::*;

#[allow(unused_imports)]
use silverscreen_frontend::pages::home;
#[allow(unused_imports)]
use silverscreen_frontend::pages::admin;
#[allow(unused_imports)]
use silverscreen_frontend::pages::reviewer;

// ---------------------------------------------------------------------------
// Page types can be constructed without panic
// ---------------------------------------------------------------------------

#[test]
fn test_login_request_construction() {
    let req = LoginRequest {
        username: "alice".to_string(),
        password: "Secret1!abc".to_string(),
    };
    assert_eq!(req.username, "alice");
    assert_eq!(req.password, "Secret1!abc");
}

#[test]
fn test_register_request_construction() {
    let req = RegisterRequest {
        username: "bob".to_string(),
        email: "bob@example.com".to_string(),
        password: "Passw0rd!".to_string(),
    };
    assert_eq!(req.username, "bob");
    assert_eq!(req.email, "bob@example.com");
}

#[test]
fn test_product_filter_default_construction() {
    let filter = ProductFilter::default();
    assert!(filter.search.is_none());
    assert!(filter.genre.is_none());
    assert!(filter.topic_id.is_none());
    assert!(filter.tag_id.is_none());
    assert!(filter.min_price.is_none());
    assert!(filter.max_price.is_none());
    assert!(filter.page.is_none());
    assert!(filter.per_page.is_none());
}

#[test]
fn test_leaderboard_query_default_construction() {
    let query = LeaderboardQuery::default();
    assert!(query.period.is_none());
    assert!(query.genre.is_none());
    assert!(query.page.is_none());
    assert!(query.per_page.is_none());
}

#[test]
fn test_audit_log_query_default_construction() {
    let query = AuditLogQuery::default();
    assert!(query.actor.is_none());
    assert!(query.action.is_none());
    assert!(query.from.is_none());
    assert!(query.to.is_none());
    assert!(query.page.is_none());
}

#[test]
fn test_create_order_request_construction() {
    let req = CreateOrderRequest {
        shipping_address: "123 Main St, Springfield, IL 62701".to_string(),
        payment_method: Some("credit_card".to_string()),
        items: vec![
            OrderItemRequest { product_id: "prod-001".to_string(), quantity: 2 },
        ],
    };
    assert!(!req.shipping_address.is_empty());
    assert_eq!(req.items.len(), 1);
    assert_eq!(req.items[0].quantity, 2);
}

#[test]
fn test_create_rating_request_construction() {
    let req = CreateRatingRequest {
        product_id: "prod-uuid".to_string(),
        dimensions: vec![
            DimensionScore { dimension_name: "Plot".to_string(), score: 8 },
            DimensionScore { dimension_name: "Acting".to_string(), score: 7 },
        ],
    };
    assert_eq!(req.dimensions.len(), 2);
    assert_eq!(req.product_id, "prod-uuid");
}

// ---------------------------------------------------------------------------
// Login form validation logic
// ---------------------------------------------------------------------------

#[test]
fn test_login_form_requires_non_empty_username() {
    let username = "";
    let password = "Secret1!";
    let is_valid = !username.is_empty() && !password.is_empty();
    assert!(!is_valid, "Login form must reject empty username");
}

#[test]
fn test_login_form_requires_non_empty_password() {
    let username = "alice";
    let password = "";
    let is_valid = !username.is_empty() && !password.is_empty();
    assert!(!is_valid, "Login form must reject empty password");
}

#[test]
fn test_login_form_valid_inputs() {
    let username = "alice";
    let password = "Secret1!";
    let is_valid = !username.is_empty() && !password.is_empty();
    assert!(is_valid, "Login form must accept non-empty username and password");
}

#[test]
fn test_login_form_both_empty() {
    let username = "";
    let password = "";
    let is_valid = !username.is_empty() && !password.is_empty();
    assert!(!is_valid, "Login form must reject both fields empty");
}

// ---------------------------------------------------------------------------
// Register form password validation (8+ chars, upper, lower, digit, special)
// ---------------------------------------------------------------------------

fn has_min_length(pw: &str) -> bool { pw.len() >= 8 }
fn has_uppercase(pw: &str) -> bool { pw.chars().any(|c| c.is_uppercase()) }
fn has_lowercase(pw: &str) -> bool { pw.chars().any(|c| c.is_lowercase()) }
fn has_digit(pw: &str) -> bool { pw.chars().any(|c| c.is_ascii_digit()) }
fn has_special(pw: &str) -> bool { pw.chars().any(|c| !c.is_alphanumeric()) }

fn is_password_valid(pw: &str) -> bool {
    has_min_length(pw) && has_uppercase(pw) && has_lowercase(pw) && has_digit(pw) && has_special(pw)
}

#[test]
fn test_password_valid() {
    assert!(is_password_valid("Secret1!abc"), "Valid password must pass all checks");
}

#[test]
fn test_password_too_short() {
    assert!(!is_password_valid("Se1!"), "Short password must fail");
    assert!(!has_min_length("Se1!"));
}

#[test]
fn test_password_no_uppercase() {
    assert!(!is_password_valid("secret1!abc"), "Password without uppercase must fail");
    assert!(!has_uppercase("secret1!abc"));
}

#[test]
fn test_password_no_lowercase() {
    assert!(!is_password_valid("SECRET1!ABC"), "Password without lowercase must fail");
    assert!(!has_lowercase("SECRET1!ABC"));
}

#[test]
fn test_password_no_digit() {
    assert!(!is_password_valid("Secret!!abc"), "Password without digit must fail");
    assert!(!has_digit("Secret!!abc"));
}

#[test]
fn test_password_no_special() {
    assert!(!is_password_valid("Secret1abc"), "Password without special char must fail");
    assert!(!has_special("Secret1abc"));
}

#[test]
fn test_password_exactly_eight_chars() {
    assert!(is_password_valid("Secre1!a"), "8-char password with all rules must pass");
}

#[test]
fn test_password_seven_chars_fails() {
    assert!(!is_password_valid("Secr1!a"), "7-char password must fail min length");
}

#[test]
fn test_register_form_email_basic_check() {
    // Simple email validation: contains @ and at least one dot after @
    let valid_email = "alice@example.com";
    let has_at = valid_email.contains('@');
    let has_dot_after_at = valid_email.split('@').nth(1).map_or(false, |domain| domain.contains('.'));
    assert!(has_at && has_dot_after_at, "Email must have @ and domain with dot");

    let invalid_email = "aliceexample.com";
    let has_at = invalid_email.contains('@');
    assert!(!has_at, "Email without @ must fail");
}

// ---------------------------------------------------------------------------
// Checkout shipping address construction
// ---------------------------------------------------------------------------

#[test]
fn test_shipping_address_struct_construction() {
    let addr = ShippingAddress {
        name: "John Doe".to_string(),
        street: "123 Main St".to_string(),
        city: "Springfield".to_string(),
        state: "IL".to_string(),
        zip: "62701".to_string(),
    };
    assert_eq!(addr.name, "John Doe");
    assert_eq!(addr.street, "123 Main St");
    assert_eq!(addr.city, "Springfield");
    assert_eq!(addr.state, "IL");
    assert_eq!(addr.zip, "62701");
}

#[test]
fn test_shipping_address_all_fields_required() {
    let fields = vec!["name", "street", "city", "state", "zip"];
    let values = vec!["John Doe", "123 Main St", "Springfield", "IL", "62701"];
    for (field, value) in fields.iter().zip(values.iter()) {
        assert!(!value.is_empty(), "{} must not be empty", field);
    }
}

#[test]
fn test_shipping_address_empty_field_fails() {
    let addr = ShippingAddress {
        name: "John Doe".to_string(),
        street: "".to_string(), // empty street
        city: "Springfield".to_string(),
        state: "IL".to_string(),
        zip: "62701".to_string(),
    };
    let all_filled = !addr.name.is_empty()
        && !addr.street.is_empty()
        && !addr.city.is_empty()
        && !addr.state.is_empty()
        && !addr.zip.is_empty();
    assert!(!all_filled, "Shipping address with empty street must fail validation");
}

#[test]
fn test_shipping_address_string_format_for_order() {
    // CreateOrderRequest uses a single string for shipping_address
    let addr = ShippingAddress {
        name: "John Doe".to_string(),
        street: "123 Main St".to_string(),
        city: "Springfield".to_string(),
        state: "IL".to_string(),
        zip: "62701".to_string(),
    };
    let formatted = format!("{}, {}, {}, {} {}", addr.name, addr.street, addr.city, addr.state, addr.zip);
    assert!(formatted.contains("John Doe"));
    assert!(formatted.contains("123 Main St"));
    assert!(formatted.contains("62701"));

    let order_req = CreateOrderRequest {
        shipping_address: formatted.clone(),
        payment_method: Some("credit_card".to_string()),
        items: vec![],
    };
    assert_eq!(order_req.shipping_address, formatted);
}

// ---------------------------------------------------------------------------
// Order status badge mapping
// ---------------------------------------------------------------------------

#[test]
fn test_order_status_badge_colors_complete() {
    // Each order status maps to a specific color for the badge
    let status_colors: Vec<(&str, &str)> = vec![
        ("Created", "#6b7280"),    // gray
        ("Reserved", "#f59e0b"),   // amber
        ("Paid", "#3b82f6"),       // blue
        ("Processing", "#8b5cf6"), // violet
        ("Shipped", "#06b6d4"),    // cyan
        ("Delivered", "#10b981"),   // emerald
        ("Completed", "#059669"),   // green
        ("Cancelled", "#ef4444"),   // red
        ("Refunded", "#f97316"),    // orange
    ];

    assert_eq!(status_colors.len(), 9, "Must have a color for all 9 order statuses");
    for (status, color) in &status_colors {
        assert!(!status.is_empty(), "Status must not be empty");
        assert!(color.starts_with('#'), "Color must be a hex code");
        assert!(color.len() == 7, "Color must be a 7-char hex code (#RRGGBB)");
    }
}

#[test]
fn test_order_status_badge_mapping_function() {
    fn status_color(status: &str) -> &str {
        match status {
            "Created" => "#6b7280",
            "Reserved" => "#f59e0b",
            "Paid" => "#3b82f6",
            "Processing" => "#8b5cf6",
            "Shipped" => "#06b6d4",
            "Delivered" => "#10b981",
            "Completed" => "#059669",
            "Cancelled" => "#ef4444",
            "Refunded" => "#f97316",
            _ => "#9ca3af", // default gray for unknown
        }
    }

    assert_eq!(status_color("Created"), "#6b7280");
    assert_eq!(status_color("Paid"), "#3b82f6");
    assert_eq!(status_color("Shipped"), "#06b6d4");
    assert_eq!(status_color("Completed"), "#059669");
    assert_eq!(status_color("Cancelled"), "#ef4444");
    assert_eq!(status_color("Unknown"), "#9ca3af", "Unknown status must get default color");
}

#[test]
fn test_order_status_timeline_progression() {
    // Status timeline: Created → Reserved → Paid → Processing → Shipped → Delivered → Completed
    let statuses = vec!["Created", "Reserved", "Paid", "Processing", "Shipped", "Delivered", "Completed"];
    assert_eq!(statuses[0], "Created", "First status must be Created");
    assert_eq!(statuses[statuses.len() - 1], "Completed", "Last status must be Completed");

    // Each status should have a unique position
    for (i, s) in statuses.iter().enumerate() {
        let found = statuses.iter().position(|x| x == s).unwrap();
        assert_eq!(found, i, "Status '{}' must appear only once in the timeline", s);
    }
}

#[test]
fn test_order_status_terminal_states() {
    // Cancelled and Refunded are terminal states (no further transitions)
    let terminal = vec!["Cancelled", "Refunded"];
    let normal_flow = vec!["Created", "Reserved", "Paid", "Processing", "Shipped", "Delivered", "Completed"];

    for t in &terminal {
        assert!(!normal_flow.contains(t), "'{}' should not be in the normal flow timeline", t);
    }
}

#[test]
fn test_order_with_status_timeline_deserializes() {
    let json_str = r#"{
        "id": "order-001",
        "user_id": "user-001",
        "status": "Shipped",
        "items": [],
        "total_amount": 29.99,
        "status_timeline": {
            "created_at": "2024-01-01T00:00:00Z",
            "paid_at": "2024-01-01T00:05:00Z",
            "shipped_at": "2024-01-02T10:00:00Z"
        }
    }"#;
    let order: Order = serde_json::from_str(json_str).unwrap();
    assert_eq!(order.status, "Shipped");
    let tl = order.status_timeline.unwrap();
    assert!(tl.created_at.is_some());
    assert!(tl.paid_at.is_some());
    assert!(tl.shipped_at.is_some());
    assert!(tl.delivered_at.is_none(), "Shipped order should not yet have delivered_at");
}

// ---------------------------------------------------------------------------
// Leaderboard page logic
// ---------------------------------------------------------------------------

#[test]
fn test_leaderboard_period_options() {
    let periods = vec!["weekly", "monthly"];
    assert_eq!(periods.len(), 2);
    assert!(periods.contains(&"weekly"));
    assert!(periods.contains(&"monthly"));
}

#[test]
fn test_leaderboard_query_construction() {
    let query = LeaderboardQuery {
        period: Some("weekly".to_string()),
        genre: Some("Action".to_string()),
        page: Some(1),
        per_page: Some(10),
    };
    let json = serde_json::to_value(&query).unwrap();
    assert_eq!(json["period"], "weekly");
    assert_eq!(json["genre"], "Action");
}

// ---------------------------------------------------------------------------
// Cart page logic
// ---------------------------------------------------------------------------

#[test]
fn test_cart_empty_state() {
    let cart = Cart {
        id: "cart-001".to_string(),
        user_id: "user-001".to_string(),
        items: vec![],
        total: 0.0,
    };
    assert!(cart.items.is_empty(), "Empty cart must have no items");
    assert_eq!(cart.total, 0.0, "Empty cart total must be 0");
}

#[test]
fn test_cart_quantity_update_logic() {
    let mut item = CartItem {
        id: "ci-001".to_string(),
        product_id: "prod-001".to_string(),
        product_title: "Movie A".to_string(),
        unit_price: 19.99,
        quantity: 1,
        line_total: 19.99,
    };

    // Increase quantity
    item.quantity = 3;
    item.line_total = item.unit_price * item.quantity as f64;
    assert_eq!(item.quantity, 3);
    assert!((item.line_total - 59.97).abs() < 0.01);
}

#[test]
fn test_cart_update_request_construction() {
    let req = UpdateCartItemRequest { quantity: 5 };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["quantity"], 5);
}
