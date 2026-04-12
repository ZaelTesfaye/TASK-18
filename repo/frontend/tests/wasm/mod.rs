// Browser-level integration tests using wasm-bindgen-test.
//
// These tests run in a real headless browser (Chrome/Firefox) via:
//   wasm-pack test --headless --chrome --test wasm
//
// They exercise actual DOM rendering and user interaction flows through the
// Yew application, going beyond the static type-level checks in the e2e/ tests.

mod test_browser;
