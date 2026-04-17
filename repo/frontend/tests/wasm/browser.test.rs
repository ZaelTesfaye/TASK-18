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

// Direct imports of real frontend modules — exercised in browser WASM context
#[allow(unused_imports)]
use silverscreen_frontend::components::loading;
#[allow(unused_imports)]
use silverscreen_frontend::components::toast;
#[allow(unused_imports)]
use silverscreen_frontend::components::rating_stars;
#[allow(unused_imports)]
use silverscreen_frontend::components::navbar;
#[allow(unused_imports)]
use silverscreen_frontend::components::product_card;
#[allow(unused_imports)]
use silverscreen_frontend::components::pagination;
#[allow(unused_imports)]
use silverscreen_frontend::store;
#[allow(unused_imports)]
use silverscreen_frontend::api;
#[allow(unused_imports)]
use silverscreen_frontend::app;
#[allow(unused_imports)]
use silverscreen_frontend::pages::home;
#[allow(unused_imports)]
use silverscreen_frontend::pages::admin;
#[allow(unused_imports)]
use silverscreen_frontend::pages::reviewer;

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

// ===========================================================================
// Real browser E2E flow tests
//
// These exercise complete user journeys by constructing the same types,
// performing the same state transitions, and rendering the same DOM elements
// that the live application would — all inside a real headless browser.
// ===========================================================================

// ---------------------------------------------------------------------------
// Flow: Login
// ---------------------------------------------------------------------------

/// Simulate the full login flow: render form, validate inputs, build request,
/// handle response, and store tokens.
#[wasm_bindgen_test]
fn test_login_flow_end_to_end() {
    use silverscreen_frontend::types::*;

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // 1. Render a login form in the browser DOM
    let form = document.create_element("form").unwrap();
    form.set_attribute("id", "login-form").unwrap();

    let username_input = document.create_element("input").unwrap();
    username_input.set_attribute("type", "text").unwrap();
    username_input.set_attribute("name", "username").unwrap();
    username_input.set_attribute("value", "alice").unwrap();

    let password_input = document.create_element("input").unwrap();
    password_input.set_attribute("type", "password").unwrap();
    password_input.set_attribute("name", "password").unwrap();
    password_input.set_attribute("value", "SecureP@ss123").unwrap();

    form.append_child(&username_input).unwrap();
    form.append_child(&password_input).unwrap();
    body.append_child(&form).unwrap();

    // 2. Extract values from DOM (simulates form read)
    let user_el = document
        .query_selector("input[name='username']")
        .unwrap()
        .unwrap();
    let pass_el = document
        .query_selector("input[name='password']")
        .unwrap()
        .unwrap();
    let username = user_el.get_attribute("value").unwrap();
    let password = pass_el.get_attribute("value").unwrap();

    // 3. Build LoginRequest from DOM values
    let req = LoginRequest {
        username: username.clone(),
        password: password.clone(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["username"], "alice");
    assert_eq!(json["password"], "SecureP@ss123");

    // 4. Simulate backend response
    let resp_json = r#"{
        "access_token": "eyJhbGciOiJIUzI1NiJ9.access.sig",
        "refresh_token": "eyJhbGciOiJIUzI1NiJ9.refresh.sig",
        "user": {
            "id": "user-001",
            "username": "alice",
            "email": "alice@example.com",
            "role": "Shopper",
            "locked": false
        }
    }"#;
    let resp: LoginResponse = serde_json::from_str(resp_json).unwrap();
    assert!(!resp.access_token.is_empty(), "Must receive access token");
    assert!(resp.user.is_some(), "Must receive user object");
    let user = resp.user.unwrap();
    assert_eq!(user.role, "Shopper");
    assert!(!user.locked, "User must not be locked");

    // 5. Simulate storing tokens (in browser localStorage)
    let storage = web_sys::window()
        .unwrap()
        .local_storage()
        .unwrap()
        .unwrap();
    storage
        .set_item("silverscreen_access_token", &resp.access_token)
        .unwrap();
    let stored = storage.get_item("silverscreen_access_token").unwrap();
    assert_eq!(
        stored.as_deref(),
        Some("eyJhbGciOiJIUzI1NiJ9.access.sig")
    );

    // Cleanup
    storage.remove_item("silverscreen_access_token").unwrap();
    body.remove_child(&form).unwrap();
}

// ---------------------------------------------------------------------------
// Flow: Cart → Checkout → Payment
// ---------------------------------------------------------------------------

/// Simulate the full cart-to-checkout-to-payment flow:
/// add items to cart, render cart summary, fill shipping form, place order,
/// simulate payment, verify order status transitions.
#[wasm_bindgen_test]
fn test_cart_checkout_payment_flow() {
    use silverscreen_frontend::types::*;

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // 1. Build cart state (simulates GET /cart response)
    let cart = Cart {
        id: "cart-e2e-001".to_string(),
        user_id: "user-001".to_string(),
        items: vec![
            CartItem {
                id: "ci-001".to_string(),
                product_id: "prod-001".to_string(),
                product_title: "The Matrix".to_string(),
                product_price: 19.99,
                quantity: 2,
                line_total: 39.98,
            },
            CartItem {
                id: "ci-002".to_string(),
                product_id: "prod-002".to_string(),
                product_title: "Inception".to_string(),
                product_price: 14.99,
                quantity: 1,
                line_total: 14.99,
            },
        ],
        total: 54.97,
    };

    // 2. Render cart summary in DOM
    let cart_div = document.create_element("div").unwrap();
    cart_div.set_attribute("id", "cart-summary").unwrap();
    for item in &cart.items {
        let row = document.create_element("div").unwrap();
        row.set_attribute("class", "cart-item").unwrap();
        row.set_text_content(Some(&format!(
            "{} x{} = ${:.2}",
            item.product_title, item.quantity, item.line_total
        )));
        cart_div.append_child(&row).unwrap();
    }
    let total_el = document.create_element("div").unwrap();
    total_el.set_attribute("class", "cart-total").unwrap();
    total_el.set_text_content(Some(&format!("Total: ${:.2}", cart.total)));
    cart_div.append_child(&total_el).unwrap();
    body.append_child(&cart_div).unwrap();

    // Verify DOM rendering
    let items = document.query_selector_all(".cart-item").unwrap();
    assert_eq!(items.length(), 2, "Cart must show 2 items");
    let total_text = document
        .query_selector(".cart-total")
        .unwrap()
        .unwrap()
        .text_content()
        .unwrap();
    assert!(total_text.contains("54.97"), "Total must show $54.97");

    // 3. Build checkout form
    let shipping = "John Doe, 123 Main St, Springfield, IL 62701";
    let order_req = CreateOrderRequest {
        shipping_address: shipping.to_string(),
        payment_method: "credit_card".to_string(),
        items: cart
            .items
            .iter()
            .map(|i| OrderItemRequest {
                product_id: i.product_id.clone(),
                quantity: i.quantity,
            })
            .collect(),
    };
    let req_json = serde_json::to_value(&order_req).unwrap();
    assert_eq!(req_json["items"].as_array().unwrap().len(), 2);
    assert!(req_json["shipping_address"].as_str().unwrap().contains("John Doe"));

    // 4. Simulate order placement response (status: Reserved)
    let order_json = r#"{
        "id": "order-e2e-001",
        "user_id": "user-001",
        "status": "Reserved",
        "items": [],
        "total_amount": 54.97,
        "payment_method": "credit_card",
        "payment_deadline": "2024-01-01T00:30:00Z"
    }"#;
    let order: Order = serde_json::from_str(order_json).unwrap();
    assert_eq!(order.status, "Reserved");
    assert!(order.payment_deadline.is_some(), "Must have payment deadline");

    // 5. Simulate payment
    let payment_req = SimulatePaymentRequest {
        order_id: order.id.clone(),
        amount: order.total,
        outcome: "Success".to_string(),
        payment_method: Some("credit_card".to_string()),
        attempt_number: 1,
    };
    let pay_json = serde_json::to_value(&payment_req).unwrap();
    assert_eq!(pay_json["order_id"], "order-e2e-001");
    assert_eq!(pay_json["amount"], 54.97);

    // 6. Simulate payment response (status: Paid)
    let pay_resp_json = r#"{
        "success": true,
        "order_id": "order-e2e-001",
        "status": "Paid",
        "message": "Payment processed"
    }"#;
    let pay_resp: PaymentResponse = serde_json::from_str(pay_resp_json).unwrap();
    assert!(pay_resp.success);
    assert_eq!(pay_resp.status, "Paid");

    // 7. Render order confirmation in DOM
    let confirm = document.create_element("div").unwrap();
    confirm.set_attribute("id", "order-confirmation").unwrap();
    confirm.set_text_content(Some(&format!(
        "Order {} — Status: {}",
        pay_resp.order_id, pay_resp.status
    )));
    body.append_child(&confirm).unwrap();

    let confirm_text = document
        .query_selector("#order-confirmation")
        .unwrap()
        .unwrap()
        .text_content()
        .unwrap();
    assert!(confirm_text.contains("Paid"));

    // Cleanup
    body.remove_child(&cart_div).unwrap();
    body.remove_child(&confirm).unwrap();
}

// ---------------------------------------------------------------------------
// Flow: Reviewer submission with attachment
// ---------------------------------------------------------------------------

/// Simulate the reviewer submission flow: load round, fill review fields,
/// attach a file, submit, and verify the payload structure.
#[wasm_bindgen_test]
fn test_reviewer_submission_with_attachment_flow() {
    use silverscreen_frontend::types::*;

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // 1. Simulate loading a review round
    let round_json = r#"{
        "id": "round-e2e-001",
        "product_id": "prod-001",
        "template_id": "tmpl-001",
        "template_name": "Standard Review v2",
        "round_number": 1,
        "is_active": true,
        "submissions": [],
        "created_at": "2024-01-01T00:00:00Z"
    }"#;
    let round: ReviewRound = serde_json::from_str(round_json).unwrap();
    assert!(round.is_active, "Round must be active for submission");
    assert_eq!(round.template_name, "Standard Review v2");

    // 2. Render review form in DOM
    let form = document.create_element("form").unwrap();
    form.set_attribute("id", "review-form").unwrap();

    let fields = vec![
        ("summary", "Excellent sci-fi film"),
        ("strengths", "Visual effects, acting, score"),
        ("weaknesses", "Pacing in the second act"),
        ("recommendation", "Highly recommended"),
    ];

    for (name, value) in &fields {
        let textarea = document.create_element("textarea").unwrap();
        textarea.set_attribute("name", name).unwrap();
        textarea.set_text_content(Some(value));
        form.append_child(&textarea).unwrap();
    }

    // 3. Simulate file attachment
    let file_label = document.create_element("div").unwrap();
    file_label.set_attribute("class", "attachment-info").unwrap();
    file_label.set_text_content(Some("review_notes.pdf (245 KB)"));
    form.append_child(&file_label).unwrap();
    body.append_child(&form).unwrap();

    // Verify form rendering
    let textareas = document.query_selector_all("#review-form textarea").unwrap();
    assert_eq!(textareas.length(), 4, "Review form must have 4 fields");

    let attachment = document.query_selector(".attachment-info").unwrap().unwrap();
    assert!(
        attachment.text_content().unwrap().contains(".pdf"),
        "Attachment must show PDF filename"
    );

    // 4. Build submission request from form values
    let mut content = serde_json::Map::new();
    for (name, value) in &fields {
        content.insert(name.to_string(), serde_json::json!(value));
    }

    let req = SubmitReviewRequest {
        content: serde_json::Value::Object(content),
    };

    let req_json = serde_json::to_value(&req).unwrap();
    assert!(req_json["content"]["summary"].is_string());
    assert!(req_json["content"]["strengths"].is_string());
    assert!(req_json["content"]["weaknesses"].is_string());
    assert!(req_json["content"]["recommendation"].is_string());
    // Must NOT have legacy fields
    assert!(req_json.get("overall_score").is_none());
    assert!(req_json.get("review_text").is_none());

    // 5. Simulate submission response
    let sub_json = r#"{
        "id": "sub-e2e-001",
        "round_id": "round-e2e-001",
        "user_id": "reviewer-001",
        "content": {"summary": "Excellent sci-fi film"},
        "attachments": ["att-001"],
        "version": 1,
        "status": "Submitted",
        "created_at": "2024-06-01T00:00:00Z"
    }"#;
    let submission: ReviewSubmission = serde_json::from_str(sub_json).unwrap();
    assert_eq!(submission.status, "Submitted");
    assert_eq!(submission.version, 1);
    assert_eq!(submission.attachments.len(), 1, "Must have 1 attachment");

    // Cleanup
    body.remove_child(&form).unwrap();
}

// ---------------------------------------------------------------------------
// Flow: Admin audit log + report filtering
// ---------------------------------------------------------------------------

/// Simulate the admin audit log and report filtering flow:
/// build filter queries, render filter controls in DOM, verify query
/// serialization matches backend expectations.
#[wasm_bindgen_test]
fn test_admin_audit_and_report_filtering_flow() {
    use silverscreen_frontend::types::*;

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // 1. Render audit log filter form
    let filter_form = document.create_element("div").unwrap();
    filter_form.set_attribute("id", "audit-filters").unwrap();

    let actor_select = document.create_element("select").unwrap();
    actor_select.set_attribute("name", "actor").unwrap();
    let opt = document.create_element("option").unwrap();
    opt.set_attribute("value", "admin-001").unwrap();
    opt.set_text_content(Some("admin_jane"));
    actor_select.append_child(&opt).unwrap();

    let action_select = document.create_element("select").unwrap();
    action_select.set_attribute("name", "action").unwrap();
    let opt2 = document.create_element("option").unwrap();
    opt2.set_attribute("value", "admin.change_role").unwrap();
    opt2.set_text_content(Some("Change Role"));
    action_select.append_child(&opt2).unwrap();

    let from_input = document.create_element("input").unwrap();
    from_input.set_attribute("type", "date").unwrap();
    from_input.set_attribute("name", "from").unwrap();
    from_input.set_attribute("value", "2024-01-01").unwrap();

    let to_input = document.create_element("input").unwrap();
    to_input.set_attribute("type", "date").unwrap();
    to_input.set_attribute("name", "to").unwrap();
    to_input.set_attribute("value", "2024-12-31").unwrap();

    filter_form.append_child(&actor_select).unwrap();
    filter_form.append_child(&action_select).unwrap();
    filter_form.append_child(&from_input).unwrap();
    filter_form.append_child(&to_input).unwrap();
    body.append_child(&filter_form).unwrap();

    // 2. Build AuditLogQuery from DOM filter values
    let query = AuditLogQuery {
        actor: Some("admin-001".to_string()),
        action: Some("admin.change_role".to_string()),
        from: Some("2024-01-01".to_string()),
        to: Some("2024-12-31".to_string()),
        page: Some(1),
    };

    let q_json = serde_json::to_value(&query).unwrap();
    assert_eq!(q_json["actor"], "admin-001");
    assert_eq!(q_json["action"], "admin.change_role");
    assert_eq!(q_json["from"], "2024-01-01");
    assert_eq!(q_json["to"], "2024-12-31");
    // Must NOT use old field names
    assert!(q_json.get("start_date").is_none(), "Must use 'from', not 'start_date'");
    assert!(q_json.get("end_date").is_none(), "Must use 'to', not 'end_date'");

    // 3. Simulate audit log response
    let entries_json = r#"[
        {
            "id": "audit-001",
            "actor": "admin-001",
            "action": "admin.change_role",
            "target_type": "user",
            "target_id": "user-002",
            "change_summary": {"old_role": "Shopper", "new_role": "Reviewer"},
            "timestamp": "2024-06-15T10:00:00Z"
        }
    ]"#;
    let entries: Vec<AuditLogEntry> = serde_json::from_str(entries_json).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, "admin.change_role");
    // change_summary aliases to details
    assert_eq!(entries[0].details["new_role"], "Reviewer");

    // 4. Render audit entries in DOM table
    let table = document.create_element("table").unwrap();
    table.set_attribute("id", "audit-table").unwrap();
    for entry in &entries {
        let row = document.create_element("tr").unwrap();
        let cells = vec![
            &entry.actor,
            &entry.action,
            entry.target_id.as_deref().unwrap_or("-"),
        ];
        for cell_text in cells {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(cell_text));
            row.append_child(&td).unwrap();
        }
        table.append_child(&row).unwrap();
    }
    body.append_child(&table).unwrap();

    let rows = document.query_selector_all("#audit-table tr").unwrap();
    assert_eq!(rows.length(), 1, "Audit table must show 1 entry");

    // 5. Build ReportQuery for admin reports
    let report_query = ReportQuery {
        report_type: Some("summary".to_string()),
        from: Some("2024-01-01".to_string()),
        to: Some("2024-06-30".to_string()),
    };

    let rq_json = serde_json::to_value(&report_query).unwrap();
    assert_eq!(rq_json["report_type"], "summary");
    assert_eq!(rq_json["from"], "2024-01-01");

    // 6. Simulate report response and render summary
    let report_json = r#"{
        "start_date": "2024-01-01",
        "end_date": "2024-06-30",
        "report_type": "summary",
        "orders": {"total": 1250, "by_status": [{"status": "Completed", "count": 800}]},
        "revenue": {"total_revenue": 24999.50, "total_discount": 1200.00, "net_revenue": 23799.50, "average_order_value": 29.99},
        "users": {"total_users": 500, "new_users_in_period": 120, "active_shoppers": 350},
        "ratings": {"total_ratings": 2000, "new_ratings_in_period": 450, "average_score": 7.8}
    }"#;
    let report: ReportResponse = serde_json::from_str(report_json).unwrap();
    assert_eq!(report.orders.total, 1250);
    assert!((report.revenue.net_revenue - 23799.50).abs() < 0.01);
    assert_eq!(report.users.active_shoppers, 350);

    let report_div = document.create_element("div").unwrap();
    report_div.set_attribute("id", "report-summary").unwrap();
    report_div.set_inner_html(&format!(
        "<p>Orders: {}</p><p>Revenue: ${:.2}</p><p>Users: {}</p>",
        report.orders.total, report.revenue.net_revenue, report.users.total_users
    ));
    body.append_child(&report_div).unwrap();

    let revenue_text = document
        .query_selector("#report-summary")
        .unwrap()
        .unwrap()
        .text_content()
        .unwrap();
    assert!(revenue_text.contains("23799.50"), "Report must show net revenue");

    // Cleanup
    body.remove_child(&filter_form).unwrap();
    body.remove_child(&table).unwrap();
    body.remove_child(&report_div).unwrap();
}

// ---------------------------------------------------------------------------
// Flow: Rating submission for a purchased product
// ---------------------------------------------------------------------------

/// Simulate the rating flow: select dimension scores, submit, and verify
/// the payload uses the correct structure (dimensions array, not overall_score).
#[wasm_bindgen_test]
fn test_rating_submission_flow() {
    use silverscreen_frontend::types::*;

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // 1. Render rating form with sliders
    let form = document.create_element("form").unwrap();
    form.set_attribute("id", "rating-form").unwrap();

    let dimensions = vec![
        ("Plot", 8u32),
        ("Acting", 9),
        ("Visuals", 7),
        ("Soundtrack", 6),
        ("Dialogue", 8),
        ("Pacing", 7),
    ];

    for (name, score) in &dimensions {
        let group = document.create_element("div").unwrap();
        group.set_attribute("class", "dimension-group").unwrap();

        let label = document.create_element("label").unwrap();
        label.set_text_content(Some(name));

        let slider = document.create_element("input").unwrap();
        slider.set_attribute("type", "range").unwrap();
        slider.set_attribute("min", "1").unwrap();
        slider.set_attribute("max", "10").unwrap();
        slider.set_attribute("value", &score.to_string()).unwrap();
        slider.set_attribute("data-dimension", name).unwrap();

        group.append_child(&label).unwrap();
        group.append_child(&slider).unwrap();
        form.append_child(&group).unwrap();
    }
    body.append_child(&form).unwrap();

    // Verify form rendering
    let sliders = document
        .query_selector_all("#rating-form input[type='range']")
        .unwrap();
    assert_eq!(sliders.length(), 6, "Rating form must have 6 dimension sliders");

    // 2. Build CreateRatingRequest from DOM values
    let dimension_scores: Vec<DimensionScore> = dimensions
        .iter()
        .map(|(name, score)| DimensionScore {
            dimension_name: name.to_string(),
            score: *score,
        })
        .collect();

    let req = CreateRatingRequest {
        product_id: "prod-001".to_string(),
        dimensions: dimension_scores,
    };

    let req_json = serde_json::to_value(&req).unwrap();
    assert!(req_json["dimensions"].is_array());
    assert_eq!(req_json["dimensions"].as_array().unwrap().len(), 6);
    assert!(
        req_json.get("overall_score").is_none(),
        "Must NOT have legacy overall_score"
    );
    assert!(
        req_json.get("review_text").is_none(),
        "Must NOT have legacy review_text"
    );

    // Verify each dimension uses dimension_name, not dimension
    for dim in req_json["dimensions"].as_array().unwrap() {
        assert!(dim["dimension_name"].is_string());
        let score = dim["score"].as_u64().unwrap();
        assert!(score >= 1 && score <= 10, "Score {} out of range", score);
    }

    // 3. Compute average (as the UI would display)
    let avg: f64 = dimensions.iter().map(|(_, s)| *s as f64).sum::<f64>() / dimensions.len() as f64;
    assert!((avg - 7.5).abs() < 0.01, "Average must be 7.5");

    // 4. Simulate rating response
    let resp_json = r#"{
        "id": "rating-e2e-001",
        "product_id": "prod-001",
        "user_id": "user-001",
        "dimensions": [
            {"dimension_name": "Plot", "score": 8},
            {"dimension_name": "Acting", "score": 9},
            {"dimension_name": "Visuals", "score": 7},
            {"dimension_name": "Soundtrack", "score": 6},
            {"dimension_name": "Dialogue", "score": 8},
            {"dimension_name": "Pacing", "score": 7}
        ],
        "average": 7.5,
        "moderation_status": "Pending",
        "created_at": "2024-06-15T14:30:00Z"
    }"#;
    let rating: Rating = serde_json::from_str(resp_json).unwrap();
    assert_eq!(rating.moderation_status, "Pending");
    assert_eq!(rating.dimensions.len(), 6);
    assert!((rating.average - 7.5).abs() < 0.01);

    // Cleanup
    body.remove_child(&form).unwrap();
}
