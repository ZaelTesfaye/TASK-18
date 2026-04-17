// Frontend unit tests
// These run in native Rust (not WASM) for logic-only modules.
// Files use the .test.rs naming convention for audit detection.

#[path = "types.test.rs"]
mod types_test;

#[path = "store.test.rs"]
mod store_test;

#[path = "navbar.test.rs"]
mod navbar_test;

#[path = "pagination.test.rs"]
mod pagination_test;

#[path = "product_card.test.rs"]
mod product_card_test;

#[path = "pages.test.rs"]
mod pages_test;

#[path = "api_client.test.rs"]
mod api_client_test;
