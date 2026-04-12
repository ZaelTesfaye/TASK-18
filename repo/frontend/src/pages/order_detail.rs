use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::{Order, ReturnRequest};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[function_component(OrderDetailPage)]
pub fn order_detail_page(props: &Props) -> Html {
    let order = use_state(|| Option::<Order>::None);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let action_loading = use_state(|| false);
    let show_return_form = use_state(|| false);
    let return_reason = use_state(|| "Defective".to_string());
    let return_text = use_state(String::new);

    let id = props.id.clone();

    {
        let order = order.clone();
        let loading = loading.clone();
        let error = error.clone();
        let id = id.clone();
        use_effect_with(id.clone(), move |id| {
            let id = id.clone();
            let order = order.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::orders::get_order(&id).await {
                    Ok(o) => order.set(Some(o)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    let on_simulate_pay = {
        let order = order.clone();
        let error = error.clone();
        let action_loading = action_loading.clone();
        let id = id.clone();
        Callback::from(move |_: MouseEvent| {
            let order = order.clone();
            let error = error.clone();
            let action_loading = action_loading.clone();
            let id = id.clone();
            let amount = order.as_ref().map(|o| o.total).unwrap_or(0.0);
            let pm = order.as_ref().and_then(|o| o.payment_method.clone());
            action_loading.set(true);
            spawn_local(async move {
                match api::orders::simulate_payment(&id, amount, "Success", pm).await {
                    Ok(_) => {
                        if let Ok(o) = api::orders::get_order(&id).await {
                            order.set(Some(o));
                        }
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    let on_cancel = {
        let order = order.clone();
        let error = error.clone();
        let action_loading = action_loading.clone();
        let id = id.clone();
        Callback::from(move |_: MouseEvent| {
            let order = order.clone();
            let error = error.clone();
            let action_loading = action_loading.clone();
            let id = id.clone();
            action_loading.set(true);
            spawn_local(async move {
                match api::orders::update_order_status(&id, "Cancelled").await {
                    Ok(o) => {
                        order.set(Some(o));
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    let on_toggle_return = {
        let show_return_form = show_return_form.clone();
        Callback::from(move |_: MouseEvent| {
            show_return_form.set(!*show_return_form);
        })
    };

    let on_return_reason = {
        let return_reason = return_reason.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            return_reason.set(el.value());
        })
    };

    let on_return_text = {
        let return_text = return_text.clone();
        Callback::from(move |e: InputEvent| {
            let el: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            return_text.set(el.value());
        })
    };

    let on_submit_return = {
        let order = order.clone();
        let error = error.clone();
        let action_loading = action_loading.clone();
        let return_reason = return_reason.clone();
        let return_text = return_text.clone();
        let show_return_form = show_return_form.clone();
        let id = id.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let order = order.clone();
            let error = error.clone();
            let action_loading = action_loading.clone();
            let show_return_form = show_return_form.clone();
            let reason_code = (*return_reason).clone();
            let _reason_text = (*return_text).clone();
            let id = id.clone();
            action_loading.set(true);
            spawn_local(async move {
                let req = ReturnRequest {
                    reason_code,
                };
                match api::orders::request_return(&id, &req).await {
                    Ok(o) => {
                        order.set(Some(o));
                        show_return_form.set(false);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    if *loading {
        return html! { <Loading /> };
    }

    let o = match &*order {
        Some(o) => o,
        None => return html! {
            <div class="page-container">
                { if let Some(ref err) = *error {
                    html! { <div class="alert alert-error">{ err }</div> }
                } else {
                    html! { <div class="alert alert-error">{ "Order not found" }</div> }
                }}
                <Link<Route> to={Route::Orders} classes="btn btn-secondary">{ "Back to Orders" }</Link<Route>>
            </div>
        },
    };

    let status_class = match o.status.to_lowercase().as_str() {
        "pending" | "reserved" => "badge badge-warning badge-lg",
        "paid" | "completed" | "delivered" => "badge badge-success badge-lg",
        "cancelled" | "failed" => "badge badge-danger badge-lg",
        "returned" | "return_requested" => "badge badge-info badge-lg",
        _ => "badge badge-secondary badge-lg",
    };

    let can_pay = matches!(o.status.to_lowercase().as_str(), "reserved" | "pending");
    let can_cancel = matches!(o.status.to_lowercase().as_str(), "reserved" | "pending");
    let can_return = matches!(o.status.to_lowercase().as_str(), "paid" | "completed" | "delivered");

    // Status timeline steps
    let statuses = vec!["Reserved", "Paid", "Completed", "Delivered"];

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Orders} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Back to Orders" }
                </Link<Route>>
                <h1 class="page-title">{ format!("Order #{}", &o.id[..8.min(o.id.len())]) }</h1>
            </div>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="order-detail-grid">
                <div class="order-info-card">
                    <h2>{ "Order Info" }</h2>
                    <div class="info-row">
                        <span class="info-label">{ "Status:" }</span>
                        <span class={status_class}>{ &o.status }</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">{ "Date:" }</span>
                        <span>{ o.created_at.map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_default() }</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">{ "Total:" }</span>
                        <span class="order-total-lg">{ format!("${:.2}", o.total) }</span>
                    </div>
                    { if let Some(ref pm) = o.payment_method {
                        html! {
                            <div class="info-row">
                                <span class="info-label">{ "Payment:" }</span>
                                <span>{ pm }</span>
                            </div>
                        }
                    } else { html! {} }}
                    { if let Some(ref addr) = o.shipping_address {
                        html! {
                            <div class="info-row">
                                <span class="info-label">{ "Ship to:" }</span>
                                <span>{ format!("{}, {}, {} {} {}", addr.name, addr.street, addr.city, addr.state, addr.zip) }</span>
                            </div>
                        }
                    } else { html! {} }}
                    { if let Some(deadline) = o.status_timeline.as_ref().and_then(|t| t.reservation_expires_at) {
                        if can_pay {
                            html! {
                                <div class="alert alert-warning">
                                    { format!("Payment deadline: {}", deadline.format("%Y-%m-%d %H:%M UTC")) }
                                </div>
                            }
                        } else { html! {} }
                    } else { html! {} }}
                </div>

                // Status timeline
                <div class="status-timeline">
                    <h2>{ "Status Timeline" }</h2>
                    <div class="timeline">
                        { for statuses.iter().enumerate().map(|(i, s)| {
                            let current_idx = statuses.iter().position(|x| {
                                x.to_lowercase() == o.status.to_lowercase()
                            }).unwrap_or(0);
                            let cls = if i < current_idx {
                                "timeline-step timeline-done"
                            } else if i == current_idx {
                                "timeline-step timeline-current"
                            } else {
                                "timeline-step timeline-pending"
                            };
                            html! {
                                <div class={cls}>
                                    <div class="timeline-dot"></div>
                                    <span class="timeline-label">{ s }</span>
                                </div>
                            }
                        })}
                    </div>
                </div>
            </div>

            // Items
            <div class="order-items-section">
                <h2>{ "Items" }</h2>
                <table class="table">
                    <thead>
                        <tr>
                            <th>{ "Product" }</th>
                            <th>{ "Unit Price" }</th>
                            <th>{ "Qty" }</th>
                            <th>{ "Total" }</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for o.items.iter().map(|item| html! {
                            <tr>
                                <td>
                                    <Link<Route> to={Route::ProductDetail { id: item.product_id.clone() }}>
                                        { &item.product_title }
                                    </Link<Route>>
                                </td>
                                <td>{ format!("${:.2}", item.unit_price) }</td>
                                <td>{ item.quantity }</td>
                                <td>{ format!("${:.2}", item.line_total) }</td>
                            </tr>
                        })}
                    </tbody>
                </table>
            </div>

            // Actions
            <div class="order-actions">
                { if can_pay {
                    html! {
                        <button
                            class="btn btn-primary"
                            onclick={on_simulate_pay}
                            disabled={*action_loading}
                        >
                            { if *action_loading { "Processing..." } else { "Simulate Payment" } }
                        </button>
                    }
                } else { html! {} }}
                { if can_cancel {
                    html! {
                        <button
                            class="btn btn-danger"
                            onclick={on_cancel}
                            disabled={*action_loading}
                        >
                            { "Cancel Order" }
                        </button>
                    }
                } else { html! {} }}
                { if can_return {
                    html! {
                        <button
                            class="btn btn-secondary"
                            onclick={on_toggle_return}
                            disabled={*action_loading}
                        >
                            { "Request Return" }
                        </button>
                    }
                } else { html! {} }}
            </div>

            // Return form
            { if *show_return_form {
                html! {
                    <div class="return-form-section">
                        <h3>{ "Request Return" }</h3>
                        <form onsubmit={on_submit_return}>
                            <div class="form-group">
                                <label class="form-label">{ "Reason Code" }</label>
                                <select class="form-select" onchange={on_return_reason}>
                                    <option value="Defective" selected={*return_reason == "Defective"}>{ "Defective" }</option>
                                    <option value="WrongItem" selected={*return_reason == "WrongItem"}>{ "Wrong Item" }</option>
                                    <option value="NotAsDescribed" selected={*return_reason == "NotAsDescribed"}>{ "Not As Described" }</option>
                                    <option value="ChangedMind" selected={*return_reason == "ChangedMind"}>{ "Changed Mind" }</option>
                                    <option value="Other" selected={*return_reason == "Other"}>{ "Other" }</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label">{ "Additional Details" }</label>
                                <textarea
                                    class="form-textarea"
                                    rows="3"
                                    value={(*return_text).clone()}
                                    oninput={on_return_text}
                                    placeholder="Describe the reason for return..."
                                ></textarea>
                            </div>
                            <button
                                type="submit"
                                class="btn btn-primary"
                                disabled={*action_loading}
                            >
                                { if *action_loading { "Submitting..." } else { "Submit Return Request" } }
                            </button>
                        </form>
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}
