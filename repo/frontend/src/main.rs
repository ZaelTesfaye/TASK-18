mod api;
mod app;
mod components;
mod config;
mod pages;
mod store;
mod types;

use app::App;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("SilverScreen frontend starting...");
    yew::Renderer::<App>::new().render();
}
