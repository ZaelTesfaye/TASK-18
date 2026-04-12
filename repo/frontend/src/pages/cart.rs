use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::Cart;

#[function_component(CartPage)]
pub fn cart_page() -> Html {
    let cart = use_state(|| Option::<Cart>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let updating = use_state(|| Option::<String>::None); // item id being updated

    let fetch_cart = {
        let cart = cart.clone();
        let loading = loading.clone();
        let error = error.clone();
        move || {
            let cart = cart.clone();
            let loading = loading.clone();
            let error = error.clone();
            spawn_local(async move {
                loading.set(true);
                match api::cart::get_cart().await {
                    Ok(c) => cart.set(Some(c)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    };

    {
        let fetch = fetch_cart.clone();
        use_effect_with((), move |_| {
            fetch();
            || ()
        });
    }

    let on_increase = {
        let cart = cart.clone();
        let error = error.clone();
        let updating = updating.clone();
        Callback::from(move |(item_id, qty): (String, u32)| {
            let cart = cart.clone();
            let error = error.clone();
            let updating = updating.clone();
            updating.set(Some(item_id.clone()));
            spawn_local(async move {
                match api::cart::update_cart_item(&item_id, qty + 1).await {
                    Ok(_) => {
                        // Refresh cart
                        if let Ok(c) = api::cart::get_cart().await {
                            cart.set(Some(c));
                        }
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                updating.set(None);
            });
        })
    };

    let on_decrease = {
        let cart = cart.clone();
        let error = error.clone();
        let updating = updating.clone();
        Callback::from(move |(item_id, qty): (String, u32)| {
            let cart = cart.clone();
            let error = error.clone();
            let updating = updating.clone();
            updating.set(Some(item_id.clone()));
            spawn_local(async move {
                if qty <= 1 {
                    match api::cart::remove_cart_item(&item_id).await {
                        Ok(_) => {
                            if let Ok(c) = api::cart::get_cart().await {
                                cart.set(Some(c));
                            }
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                } else {
                    match api::cart::update_cart_item(&item_id, qty - 1).await {
                        Ok(_) => {
                            if let Ok(c) = api::cart::get_cart().await {
                                cart.set(Some(c));
                            }
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                }
                updating.set(None);
            });
        })
    };

    let on_remove = {
        let cart = cart.clone();
        let error = error.clone();
        let updating = updating.clone();
        Callback::from(move |item_id: String| {
            let cart = cart.clone();
            let error = error.clone();
            let updating = updating.clone();
            updating.set(Some(item_id.clone()));
            spawn_local(async move {
                match api::cart::remove_cart_item(&item_id).await {
                    Ok(_) => {
                        if let Ok(c) = api::cart::get_cart().await {
                            cart.set(Some(c));
                        }
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                updating.set(None);
            });
        })
    };

    let on_clear = {
        let cart = cart.clone();
        let error = error.clone();
        let loading = loading.clone();
        Callback::from(move |_: MouseEvent| {
            let cart = cart.clone();
            let error = error.clone();
            let loading = loading.clone();
            loading.set(true);
            spawn_local(async move {
                match api::cart::clear_cart().await {
                    Ok(_) => {
                        if let Ok(c) = api::cart::get_cart().await {
                            cart.set(Some(c));
                        }
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        })
    };

    if *loading && (*cart).is_none() {
        return html! { <Loading /> };
    }

    html! {
        <div class="page-container">
            <h1 class="page-title">{ "Shopping Cart" }</h1>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { match &*cart {
                Some(c) if c.items.is_empty() => html! {
                    <div class="empty-state">
                        <h2>{ "Your cart is empty" }</h2>
                        <p>{ "Browse our catalog and add some items!" }</p>
                        <Link<Route> to={Route::Catalog} classes="btn btn-primary">
                            { "Browse Catalog" }
                        </Link<Route>>
                    </div>
                },
                Some(c) => html! {
                    <>
                        <div class="cart-items">
                            { for c.items.iter().map(|item| {
                                let item_id = item.id.clone();
                                let qty = item.quantity;
                                let is_updating = (*updating).as_ref() == Some(&item_id);

                                let inc_id = item_id.clone();
                                let dec_id = item_id.clone();
                                let rem_id = item_id.clone();
                                let on_inc = on_increase.clone();
                                let on_dec = on_decrease.clone();
                                let on_rem = on_remove.clone();

                                html! {
                                    <div class="cart-item">
                                        <div class="cart-item-info">
                                            <Link<Route> to={Route::ProductDetail { id: item.product_id.clone() }}>
                                                <h3 class="cart-item-title">{ &item.product_title }</h3>
                                            </Link<Route>>
                                            <span class="cart-item-price">
                                                { format!("${:.2} each", item.unit_price) }
                                            </span>
                                        </div>
                                        <div class="cart-item-controls">
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                disabled={is_updating}
                                                onclick={Callback::from(move |_| on_dec.emit((dec_id.clone(), qty)))}
                                            >
                                                { "-" }
                                            </button>
                                            <span class="cart-item-qty">{ qty }</span>
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                disabled={is_updating}
                                                onclick={Callback::from(move |_| on_inc.emit((inc_id.clone(), qty)))}
                                            >
                                                { "+" }
                                            </button>
                                        </div>
                                        <div class="cart-item-total">
                                            { format!("${:.2}", item.line_total) }
                                        </div>
                                        <button
                                            class="btn btn-sm btn-danger"
                                            disabled={is_updating}
                                            onclick={Callback::from(move |_| on_rem.emit(rem_id.clone()))}
                                        >
                                            { "Remove" }
                                        </button>
                                    </div>
                                }
                            })}
                        </div>

                        <div class="cart-summary">
                            <div class="cart-total">
                                <span>{ "Total:" }</span>
                                <span class="cart-total-amount">{ format!("${:.2}", c.total) }</span>
                            </div>
                            <div class="cart-actions">
                                <button class="btn btn-secondary" onclick={on_clear}>
                                    { "Clear Cart" }
                                </button>
                                <Link<Route> to={Route::Checkout} classes="btn btn-primary">
                                    { "Proceed to Checkout" }
                                </Link<Route>>
                            </div>
                        </div>
                    </>
                },
                None => html! {
                    <div class="empty-state">
                        <p>{ "Unable to load cart." }</p>
                    </div>
                },
            }}
        </div>
    }
}
