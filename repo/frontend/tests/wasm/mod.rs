// Browser-level integration tests using wasm-bindgen-test.
// Files use the .test.rs naming convention for audit detection.
//
// Run with: wasm-pack test --headless --chrome --test wasm

#[path = "browser.test.rs"]
mod browser_test;

#[path = "e2e_flow.test.rs"]
mod e2e_flow_test;
