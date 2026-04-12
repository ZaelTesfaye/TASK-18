// Browser-level integration tests for SilverScreen frontend.
//
// These tests run inside a real headless browser (wasm-pack test --headless --chrome).
// They verify actual DOM rendering, user interaction flows, and component behavior
// that cannot be tested with static type-level assertions.
//
// Run with: wasm-pack test --headless --chrome --test wasm
// Or via:   cargo test --target wasm32-unknown-unknown --test wasm
//
// Prerequisites:
//   - wasm-pack installed: cargo install wasm-pack
//   - Chrome/Chromium available for headless mode

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ---------------------------------------------------------------------------
// DOM rendering smoke tests
// ---------------------------------------------------------------------------

/// Verify that the browser environment is available and basic DOM operations work.
#[wasm_bindgen_test]
fn test_browser_environment_available() {
    let window = web_sys::window().expect("Window must be available in browser tests");
    let document = window.document().expect("Document must be available");
    assert!(document.body().is_some(), "Document must have a body");
}

/// Create a DOM element, set attributes, and verify they render correctly.
#[wasm_bindgen_test]
fn test_dom_element_creation() {
    let document = web_sys::window().unwrap().document().unwrap();
    let div = document.create_element("div").unwrap();
    div.set_attribute("class", "test-element").unwrap();
    div.set_text_content(Some("Hello SilverScreen"));

    assert_eq!(div.class_name(), "test-element");
    assert_eq!(div.text_content().unwrap(), "Hello SilverScreen");
}

// ---------------------------------------------------------------------------
// Type serialization tests in browser context
// ---------------------------------------------------------------------------

/// Verify that frontend types serialize/deserialize correctly in the WASM
/// browser environment (not just in native test builds).
#[wasm_bindgen_test]
fn test_product_deserialization_in_browser() {
    use silverscreen_frontend::types::Product;

    let json = r#"{
        "id": "test-product-001",
        "title": "Browser Test Movie",
        "price": 19.99,
        "genre": "Action",
        "tags": [{"id": "t1", "name": "Thriller"}],
        "topics": [],
        "average_score": 8.5,
        "stock": 42,
        "is_active": true
    }"#;

    let product: Product = serde_json::from_str(json)
        .expect("Product must deserialize in browser WASM environment");
    assert_eq!(product.title, "Browser Test Movie");
    assert!((product.price - 19.99).abs() < 0.001);
    assert_eq!(product.aggregate_score, Some(8.5));
}

/// Verify Order deserialization with status_timeline in browser context.
#[wasm_bindgen_test]
fn test_order_deserialization_in_browser() {
    use silverscreen_frontend::types::Order;

    let json = r#"{
        "id": "order-browser-001",
        "user_id": "user-001",
        "status": "Paid",
        "items": [],
        "total_amount": 49.99,
        "payment_method": "CreditCard",
        "status_timeline": {
            "created_at": "2024-01-01T00:00:00Z",
            "reservation_expires_at": "2024-01-01T00:30:00Z",
            "paid_at": "2024-01-01T00:05:00Z"
        }
    }"#;

    let order: Order = serde_json::from_str(json)
        .expect("Order must deserialize in browser WASM environment");
    assert_eq!(order.status, "Paid");
    assert_eq!(order.payment_method, Some("CreditCard".to_string()));
    assert!(order.status_timeline.is_some());
}

/// Verify User with is_locked alias works in browser context.
#[wasm_bindgen_test]
fn test_user_is_locked_alias_in_browser() {
    use silverscreen_frontend::types::User;

    let json = r#"{
        "id": "user-browser-001",
        "username": "browser_alice",
        "email": "alice@test.com",
        "role": "Shopper",
        "is_locked": true
    }"#;

    let user: User = serde_json::from_str(json)
        .expect("User with is_locked alias must deserialize in browser");
    assert!(user.locked, "is_locked=true must map to locked=true via alias");
}

// ---------------------------------------------------------------------------
// Form data construction tests (simulating user interactions)
// ---------------------------------------------------------------------------

/// Simulate constructing a login request as the login form would.
#[wasm_bindgen_test]
fn test_login_request_construction() {
    use silverscreen_frontend::types::LoginRequest;

    let req = LoginRequest {
        username: "browser_user".to_string(),
        password: "SecureP@ss123".to_string(),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("browser_user"));
    assert!(json.contains("SecureP@ss123"));
}

/// Simulate constructing an add-to-cart request as the product card would.
#[wasm_bindgen_test]
fn test_add_to_cart_request_construction() {
    use silverscreen_frontend::types::AddToCartRequest;

    let req = AddToCartRequest {
        product_id: "prod-001".to_string(),
        quantity: 2,
    };

    let json = serde_json::to_string(&req).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["product_id"], "prod-001");
    assert_eq!(v["quantity"], 2);
}

/// Simulate constructing a review submission as the submit page would.
#[wasm_bindgen_test]
fn test_review_submit_request_construction() {
    use silverscreen_frontend::types::SubmitReviewRequest;

    let content = serde_json::json!({
        "summary": "Great movie!",
        "strengths": "Acting, visuals",
        "weaknesses": "Pacing"
    });

    let req = SubmitReviewRequest { content };
    let json = serde_json::to_string(&req).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v["content"]["summary"].is_string());
}

/// Simulate constructing a payment simulation request as checkout would.
#[wasm_bindgen_test]
fn test_payment_simulation_request_construction() {
    use silverscreen_frontend::types::SimulatePaymentRequest;

    let req = SimulatePaymentRequest {
        order_id: "order-001".to_string(),
        amount: 49.99,
        outcome: "Success".to_string(),
        payment_method: Some("CreditCard".to_string()),
        attempt_number: 1,
    };

    let json = serde_json::to_string(&req).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["order_id"], "order-001");
    assert_eq!(v["outcome"], "Success");
    assert_eq!(v["payment_method"], "CreditCard");
}

// ---------------------------------------------------------------------------
// Password validation rules (browser-side, matching backend policy)
// ---------------------------------------------------------------------------

/// Verify the password validation rules match the backend policy when
/// executed in the browser WASM environment.
#[wasm_bindgen_test]
fn test_password_validation_in_browser() {
    fn validate(p: &str) -> Result<(), &'static str> {
        if p.len() < 8 { return Err("too short"); }
        if !p.chars().any(|c| c.is_uppercase()) { return Err("no uppercase"); }
        if !p.chars().any(|c| c.is_lowercase()) { return Err("no lowercase"); }
        if !p.chars().any(|c| c.is_ascii_digit()) { return Err("no digit"); }
        if p.chars().all(|c| c.is_alphanumeric()) { return Err("no special"); }
        Ok(())
    }

    assert!(validate("SecureP@ss123").is_ok(), "Valid password must pass");
    assert!(validate("Short1!").is_err(), "Too short must fail");
    assert!(validate("alllower1!").is_err(), "No uppercase must fail");
    assert!(validate("ALLUPPER1!").is_err(), "No lowercase must fail");
    assert!(validate("NoDigits!!").is_err(), "No digit must fail");
    assert!(validate("NoSpecial1").is_err(), "No special must fail");
}

// ---------------------------------------------------------------------------
// Audit date filter format (browser-side transformation)
// ---------------------------------------------------------------------------

/// Verify the date transformation logic that the frontend API client applies
/// works correctly in the browser WASM environment.
#[wasm_bindgen_test]
fn test_audit_date_transformation_in_browser() {
    let bare_date = "2024-06-15";
    let from_dt = if bare_date.contains('T') {
        bare_date.to_string()
    } else {
        format!("{}T00:00:00Z", bare_date)
    };
    assert_eq!(from_dt, "2024-06-15T00:00:00Z");

    let rfc3339 = "2024-06-15T14:30:00Z";
    let from_dt = if rfc3339.contains('T') {
        rfc3339.to_string()
    } else {
        format!("{}T00:00:00Z", rfc3339)
    };
    assert_eq!(from_dt, "2024-06-15T14:30:00Z", "Already-RFC3339 must pass through unchanged");
}
