use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::app::Route;
use crate::components::rating_stars::RatingStars;
use crate::store;
use crate::types::{AddToCartRequest, Product};

#[derive(Properties, PartialEq)]
pub struct ProductCardProps {
    pub product: Product,
    #[prop_or_default]
    pub on_added_to_cart: Callback<String>,
    #[prop_or_default]
    pub on_error: Callback<String>,
}

#[function_component(ProductCard)]
pub fn product_card(props: &ProductCardProps) -> Html {
    let loading = use_state(|| false);
    let product = &props.product;

    let on_add = {
        let loading = loading.clone();
        let product_id = product.id.clone();
        let on_added = props.on_added_to_cart.clone();
        let on_error = props.on_error.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            if !store::is_authenticated() {
                on_error.emit("Please log in to add items to cart.".to_string());
                return;
            }
            let loading = loading.clone();
            let product_id = product_id.clone();
            let on_added = on_added.clone();
            let on_error = on_error.clone();
            loading.set(true);
            spawn_local(async move {
                let req = AddToCartRequest {
                    product_id: product_id.clone(),
                    quantity: 1,
                };
                match api::cart::add_to_cart(&req).await {
                    Ok(_) => on_added.emit(product_id),
                    Err(e) => on_error.emit(e.to_string()),
                }
                loading.set(false);
            });
        })
    };

    let score = product.aggregate_score.unwrap_or(0.0);

    html! {
        <div class="product-card">
            <Link<Route> to={Route::ProductDetail { id: product.id.clone() }}>
                <div class="product-card-image">
                    { if let Some(ref url) = product.image_url {
                        html! { <img src={url.clone()} alt={product.title.clone()} /> }
                    } else {
                        html! {
                            <div class="product-card-placeholder">
                                <span>{ "\u{1F3AC}" }</span>
                            </div>
                        }
                    }}
                </div>
                <div class="product-card-body">
                    <h3 class="product-card-title">{ &product.title }</h3>
                    { if !product.genre.is_empty() {
                        html! { <span class="badge badge-secondary">{ &product.genre }</span> }
                    } else {
                        html! {}
                    }}
                    <div class="product-card-rating">
                        <RatingStars score={score} show_value={true} />
                    </div>
                    <div class="product-card-price">{ format!("${:.2}", product.price) }</div>
                </div>
            </Link<Route>>
            <div class="product-card-actions">
                <button
                    class="btn btn-primary btn-sm"
                    onclick={on_add}
                    disabled={*loading}
                >
                    { if *loading { "Adding..." } else { "Add to Cart" } }
                </button>
            </div>
        </div>
    }
}
