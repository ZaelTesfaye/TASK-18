// Library target for the SilverScreen frontend.
// Exposes types and config for native integration tests, and all modules
// (components, pages, store, api, app) for wasm-bindgen-test browser tests.

pub mod types;
pub mod config;
pub mod store;
pub mod api;
pub mod components;
pub mod pages;
pub mod app;
