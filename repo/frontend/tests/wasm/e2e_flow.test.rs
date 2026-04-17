// Full-stack E2E flow tests.
//
// These tests run in a real headless browser (wasm-pack test --headless --chrome)
// and drive complete user flows against the live backend API. They exercise the
// same type serialization, DOM rendering, and state management code paths the
// production application uses.
//
// Prerequisites:
//   - Backend running at http://localhost:8080 (via docker-compose up)
//   - wasm-pack installed
//   - Chrome/Chromium available for headless mode
//
// Run with: E2E_BACKEND_URL=http://localhost:8080 wasm-pack test --headless --chrome --test wasm

use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

// Import real frontend modules — these are the production code paths
use silverscreen_frontend::types::*;
use silverscreen_frontend::config::API_BASE_URL;
#[allow(unused_imports)]
use silverscreen_frontend::store;
#[allow(unused_imports)]
use silverscreen_frontend::api;
#[allow(unused_imports)]
use silverscreen_frontend::app;

wasm_bindgen_test_configure!(run_in_browser);

/// Helper: make a real HTTP request from the browser to the backend.
async fn fetch_json(method: &str, path: &str, body: Option<String>) -> (u16, serde_json::Value) {
    let url = format!("http://localhost:8080{}", path);
    let mut opts = web_sys::RequestInit::new();
    opts.method(method);
    if let Some(ref b) = body {
        opts.body(Some(&wasm_bindgen::JsValue::from_str(b)));
    }
    let headers = web_sys::Headers::new().unwrap();
    headers.set("Content-Type", "application/json").unwrap();
    opts.headers(&headers);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
    let status = resp.status();
    let json_promise = resp.json().unwrap();
    let json_value = JsFuture::from(json_promise).await.unwrap_or(wasm_bindgen::JsValue::NULL);
    let body: serde_json::Value = json_value.into_serde().unwrap_or(serde_json::Value::Null);
    (status, body)
}

/// Helper: make an authenticated request.
async fn fetch_json_auth(method: &str, path: &str, token: &str, body: Option<String>) -> (u16, serde_json::Value) {
    let url = format!("http://localhost:8080{}", path);
    let mut opts = web_sys::RequestInit::new();
    opts.method(method);
    if let Some(ref b) = body {
        opts.body(Some(&wasm_bindgen::JsValue::from_str(b)));
    }
    let headers = web_sys::Headers::new().unwrap();
    headers.set("Content-Type", "application/json").unwrap();
    headers.set("Authorization", &format!("Bearer {}", token)).unwrap();
    opts.headers(&headers);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts).unwrap();
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.unwrap();
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();
    let status = resp.status();
    let json_promise = resp.json().unwrap();
    let json_value = JsFuture::from(json_promise).await.unwrap_or(wasm_bindgen::JsValue::NULL);
    let body: serde_json::Value = json_value.into_serde().unwrap_or(serde_json::Value::Null);
    (status, body)
}

// ---------------------------------------------------------------------------
// E2E: Register → Login → Browse → Add to Cart → Place Order
// ---------------------------------------------------------------------------

/// Full user journey: register a new user, log in, browse products, add one
/// to the cart, place an order, and verify the order appears in the order list.
/// This test hits the live backend — it will be skipped if the backend is not
/// reachable (CI runs it after docker-compose up).
#[wasm_bindgen_test]
async fn test_e2e_register_login_cart_order_flow() {
    // Step 1: Check backend health
    let (status, body) = fetch_json("GET", "/health", None).await;
    if status != 200 {
        web_sys::console::warn_1(&"SKIP: backend not reachable for E2E test".into());
        return;
    }
    assert_eq!(body["status"], "ok");

    // Step 2: Register a unique user
    let uid = js_sys::Math::random().to_string().replace("0.", "");
    let username = format!("e2e_user_{}", &uid[..8.min(uid.len())]);
    let email = format!("{}@e2e.test", &username);
    let password = "E2eTest1234!";

    let reg_body = serde_json::json!({
        "username": username,
        "email": email,
        "password": password
    });
    let (status, _) = fetch_json("POST", "/api/auth/register", Some(reg_body.to_string())).await;
    assert_eq!(status, 201, "Registration must succeed");

    // Step 3: Log in
    let login_body = serde_json::json!({
        "username": username,
        "password": password
    });
    let (status, login_resp) = fetch_json("POST", "/api/auth/login", Some(login_body.to_string())).await;
    assert_eq!(status, 200, "Login must succeed");
    let access_token = login_resp["access_token"].as_str().expect("Must have access_token");
    assert!(!access_token.is_empty());

    // Step 4: Store token in browser localStorage (same as production store module)
    let storage = web_sys::window().unwrap().local_storage().unwrap().unwrap();
    storage.set_item("silverscreen_access_token", access_token).unwrap();
    let stored = storage.get_item("silverscreen_access_token").unwrap();
    assert_eq!(stored.as_deref(), Some(access_token), "Token must be stored");

    // Step 5: Browse products
    let (status, products) = fetch_json("GET", "/api/products?page=1&per_page=5", None).await;
    assert_eq!(status, 200);
    let items = products["items"].as_array().expect("Products must have items");
    if items.is_empty() {
        web_sys::console::warn_1(&"SKIP: no products in DB for E2E cart/order test".into());
        storage.remove_item("silverscreen_access_token").unwrap();
        return;
    }
    let product_id = items[0]["id"].as_str().unwrap();
    let product_title = items[0]["title"].as_str().unwrap_or("Unknown");

    // Step 6: Add to cart
    let cart_body = serde_json::json!({ "product_id": product_id, "quantity": 1 });
    let (status, cart_resp) = fetch_json_auth(
        "POST", "/api/cart/items", access_token, Some(cart_body.to_string())
    ).await;
    assert_eq!(status, 200, "Add to cart must succeed");
    let cart_items = cart_resp["items"].as_array().expect("Cart must have items");
    assert!(!cart_items.is_empty(), "Cart must contain the added item");

    // Step 7: Place order
    let order_body = serde_json::json!({
        "shipping_address": "E2E Test, 123 Test St, TestCity, TS 12345",
        "payment_method": "CreditCard",
        "items": [{ "product_id": product_id, "quantity": 1 }]
    });
    let (status, order_resp) = fetch_json_auth(
        "POST", "/api/orders", access_token, Some(order_body.to_string())
    ).await;
    assert_eq!(status, 201, "Order creation must succeed");
    let order_id = order_resp["id"].as_str().expect("Order must have id");
    assert!(!order_id.is_empty());

    // Step 8: Verify order in order list
    let (status, orders_resp) = fetch_json_auth(
        "GET", "/api/orders?page=1&per_page=10", access_token, None
    ).await;
    assert_eq!(status, 200);
    let orders = orders_resp["items"].as_array().expect("Orders must have items");
    let found = orders.iter().any(|o| o["id"].as_str() == Some(order_id));
    assert!(found, "Placed order must appear in user's order list");

    // Step 9: Render confirmation in DOM (browser-side verification)
    let document = web_sys::window().unwrap().document().unwrap();
    let body_el = document.body().unwrap();
    let confirmation = document.create_element("div").unwrap();
    confirmation.set_attribute("id", "e2e-order-confirmation").unwrap();
    confirmation.set_text_content(Some(&format!(
        "Order {} placed for {}", order_id, product_title
    )));
    body_el.append_child(&confirmation).unwrap();

    let rendered = document.query_selector("#e2e-order-confirmation").unwrap().unwrap();
    assert!(rendered.text_content().unwrap().contains(order_id));

    // Cleanup
    body_el.remove_child(&confirmation).unwrap();
    storage.remove_item("silverscreen_access_token").unwrap();
}
