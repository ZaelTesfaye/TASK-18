use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::{Cart, CreateOrderRequest, Order, OrderItemRequest};

#[function_component(CheckoutPage)]
pub fn checkout_page() -> Html {
    let navigator = use_navigator().unwrap();
    let cart = use_state(|| Option::<Cart>::None);
    let loading = use_state(|| true);
    let submitting = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let order = use_state(|| Option::<Order>::None);
    let paying = use_state(|| false);

    // Shipping form
    let name = use_state(String::new);
    let street = use_state(String::new);
    let city = use_state(String::new);
    let state = use_state(String::new);
    let zip = use_state(String::new);
    let payment_method = use_state(|| "credit_card".to_string());

    // Fetch cart
    {
        let cart = cart.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::cart::get_cart().await {
                    Ok(c) => cart.set(Some(c)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    let make_input = |state: UseStateHandle<String>, placeholder: &str, label: &str, id: &str| {
        let s = state.clone();
        let oninput = Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            s.set(el.value());
        });
        let val = (*state).clone();
        let disabled = *submitting || (*order).is_some();
        let lbl = label.to_string();
        let id_str = id.to_string();
        let ph = placeholder.to_string();
        html! {
            <div class="form-group">
                <label for={id_str.clone()} class="form-label">{ lbl }</label>
                <input
                    id={id_str}
                    type="text"
                    class="form-input"
                    placeholder={ph}
                    value={val}
                    oninput={oninput}
                    disabled={disabled}
                />
            </div>
        }
    };

    let on_payment_method = {
        let pm = payment_method.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            pm.set(el.value());
        })
    };

    let on_submit = {
        let name = name.clone();
        let street = street.clone();
        let city = city.clone();
        let state_val = state.clone();
        let zip = zip.clone();
        let payment_method = payment_method.clone();
        let submitting = submitting.clone();
        let error = error.clone();
        let order = order.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let n = (*name).clone();
            let s = (*street).clone();
            let c = (*city).clone();
            let st = (*state_val).clone();
            let z = (*zip).clone();
            let pm = (*payment_method).clone();

            if n.is_empty() || s.is_empty() || c.is_empty() || st.is_empty() || z.is_empty() {
                error.set(Some("Please fill in all shipping fields.".into()));
                return;
            }

            let submitting = submitting.clone();
            let error = error.clone();
            let order = order.clone();
            submitting.set(true);
            error.set(None);

            let cart_ref = (*cart).clone();
            spawn_local(async move {
                // Build items from cart
                let items: Vec<OrderItemRequest> = cart_ref
                    .map(|c| c.items.iter().map(|i| OrderItemRequest {
                        product_id: i.product_id.clone(),
                        quantity: i.quantity,
                    }).collect())
                    .unwrap_or_default();

                let req = CreateOrderRequest {
                    shipping_address: format!("{}, {}, {}, {} {}", n, s, c, st, z),
                    payment_method: Some(pm),
                    items,
                };
                match api::orders::create_order(&req).await {
                    Ok(o) => {
                        order.set(Some(o));
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                submitting.set(false);
            });
        })
    };

    let on_simulate_pay = {
        let order = order.clone();
        let error = error.clone();
        let paying = paying.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: MouseEvent| {
            let order_data = (*order).clone();
            if let Some(ref o) = order_data {
                let oid = o.id.clone();
                let amount = o.total;
                let pm = o.payment_method.clone();
                let error = error.clone();
                let paying = paying.clone();
                let navigator = navigator.clone();
                paying.set(true);
                spawn_local(async move {
                    match api::orders::simulate_payment(&oid, amount, "Success", pm).await {
                        Ok(_) => {
                            navigator.push(&Route::OrderDetail { id: oid });
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                    paying.set(false);
                });
            }
        })
    };

    if *loading {
        return html! { <Loading /> };
    }

    let cart_data = match &*cart {
        Some(c) if c.items.is_empty() => return html! {
            <div class="page-container">
                <div class="empty-state">
                    <h2>{ "Your cart is empty" }</h2>
                    <Link<Route> to={Route::Catalog} classes="btn btn-primary">
                        { "Browse Catalog" }
                    </Link<Route>>
                </div>
            </div>
        },
        Some(c) => c.clone(),
        None => return html! {
            <div class="page-container">
                <div class="alert alert-error">{ "Unable to load cart" }</div>
            </div>
        },
    };

    html! {
        <div class="page-container">
            <h1 class="page-title">{ "Checkout" }</h1>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if let Some(ref o) = *order {
                html! {
                    <div class="order-placed">
                        <div class="alert alert-success">
                            <h2>{ "Order Placed!" }</h2>
                            <p>{ format!("Order ID: {}", o.id) }</p>
                            <p class="text-warning">{ "Order reserved for 30 minutes." }</p>
                            { if let Some(deadline) = o.status_timeline.as_ref().and_then(|t| t.reservation_expires_at) {
                                html! {
                                    <p class="text-warning">
                                        { format!("Unpaid order will auto-cancel at {}", deadline.format("%Y-%m-%d %H:%M UTC")) }
                                    </p>
                                }
                            } else { html! {} }}
                        </div>
                        <div class="checkout-actions">
                            <button
                                class="btn btn-primary"
                                onclick={on_simulate_pay}
                                disabled={*paying}
                            >
                                { if *paying { "Processing Payment..." } else { "Simulate Payment" } }
                            </button>
                            <Link<Route> to={Route::OrderDetail { id: o.id.clone() }} classes="btn btn-secondary">
                                { "View Order" }
                            </Link<Route>>
                        </div>
                    </div>
                }
            } else {
                html! {
                    <div class="checkout-layout">
                        <div class="checkout-form">
                            <h2>{ "Shipping Address" }</h2>
                            <form onsubmit={on_submit}>
                                { make_input(name.clone(), "Full name", "Name", "ship-name") }
                                { make_input(street.clone(), "Street address", "Street", "ship-street") }
                                <div class="form-row">
                                    { make_input(city.clone(), "City", "City", "ship-city") }
                                    { make_input(state.clone(), "State", "State", "ship-state") }
                                    { make_input(zip.clone(), "ZIP code", "ZIP", "ship-zip") }
                                </div>

                                <h2>{ "Payment Method" }</h2>
                                <div class="form-group">
                                    <select
                                        class="form-select"
                                        onchange={on_payment_method}
                                        disabled={*submitting}
                                    >
                                        <option value="credit_card" selected={*payment_method == "credit_card"}>
                                            { "Credit Card" }
                                        </option>
                                        <option value="debit_card" selected={*payment_method == "debit_card"}>
                                            { "Debit Card" }
                                        </option>
                                        <option value="bank_transfer" selected={*payment_method == "bank_transfer"}>
                                            { "Bank Transfer" }
                                        </option>
                                        <option value="digital_wallet" selected={*payment_method == "digital_wallet"}>
                                            { "Digital Wallet" }
                                        </option>
                                    </select>
                                </div>

                                <button
                                    type="submit"
                                    class="btn btn-primary btn-full"
                                    disabled={*submitting}
                                >
                                    { if *submitting { "Placing Order..." } else { "Place Order" } }
                                </button>
                            </form>
                        </div>

                        <div class="checkout-summary">
                            <h2>{ "Order Summary" }</h2>
                            <div class="summary-items">
                                { for cart_data.items.iter().map(|item| html! {
                                    <div class="summary-item">
                                        <span>{ format!("{} x{}", item.product_title, item.quantity) }</span>
                                        <span>{ format!("${:.2}", item.line_total) }</span>
                                    </div>
                                })}
                            </div>
                            <div class="summary-total">
                                <span>{ "Total" }</span>
                                <span class="total-amount">{ format!("${:.2}", cart_data.total) }</span>
                            </div>
                        </div>
                    </div>
                }
            }}
        </div>
    }
}
