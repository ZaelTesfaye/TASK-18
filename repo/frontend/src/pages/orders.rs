use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::components::pagination::Pagination;
use crate::types::Order;

#[function_component(OrdersPage)]
pub fn orders_page() -> Html {
    let orders = use_state(Vec::<Order>::new);
    let page = use_state(|| 1u64);
    let total_pages = use_state(|| 1u64);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let status_filter = use_state(String::new);

    {
        let orders = orders.clone();
        let total_pages = total_pages.clone();
        let loading = loading.clone();
        let error = error.clone();
        let page = page.clone();
        use_effect_with(*page, move |p| {
            let pg = *p;
            let orders = orders.clone();
            let total_pages = total_pages.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::orders::list_orders(pg).await {
                    Ok(resp) => {
                        orders.set(resp.items);
                        total_pages.set(resp.total_pages);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    let on_page = {
        let page = page.clone();
        Callback::from(move |p: u64| page.set(p))
    };

    let on_status_filter = {
        let status_filter = status_filter.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            status_filter.set(el.value());
        })
    };

    let filtered: Vec<Order> = (*orders)
        .iter()
        .filter(|o| {
            if (*status_filter).is_empty() {
                true
            } else {
                o.status.to_lowercase() == (*status_filter).to_lowercase()
            }
        })
        .cloned()
        .collect();

    let status_class = |status: &str| -> &str {
        match status.to_lowercase().as_str() {
            "pending" | "reserved" => "badge badge-warning",
            "paid" | "completed" | "delivered" => "badge badge-success",
            "cancelled" | "failed" => "badge badge-danger",
            "returned" | "return_requested" => "badge badge-info",
            _ => "badge badge-secondary",
        }
    };

    html! {
        <div class="page-container">
            <h1 class="page-title">{ "My Orders" }</h1>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="orders-toolbar">
                <div class="form-group form-inline">
                    <label class="form-label">{ "Status:" }</label>
                    <select class="form-select" onchange={on_status_filter}>
                        <option value="">{ "All" }</option>
                        <option value="reserved">{ "Reserved" }</option>
                        <option value="pending">{ "Pending" }</option>
                        <option value="paid">{ "Paid" }</option>
                        <option value="completed">{ "Completed" }</option>
                        <option value="cancelled">{ "Cancelled" }</option>
                        <option value="return_requested">{ "Return Requested" }</option>
                        <option value="returned">{ "Returned" }</option>
                    </select>
                </div>
            </div>

            { if *loading {
                html! { <Loading /> }
            } else if filtered.is_empty() {
                html! {
                    <div class="empty-state">
                        <p>{ "No orders found." }</p>
                        <Link<Route> to={Route::Catalog} classes="btn btn-primary">
                            { "Browse Catalog" }
                        </Link<Route>>
                    </div>
                }
            } else {
                html! {
                    <>
                        <div class="orders-list">
                            { for filtered.iter().map(|o| {
                                let status_cls = status_class(&o.status);
                                html! {
                                    <Link<Route> to={Route::OrderDetail { id: o.id.clone() }} classes="order-card-link">
                                        <div class="order-card">
                                            <div class="order-card-header">
                                                <span class="order-id">{ format!("#{}", &o.id[..8.min(o.id.len())]) }</span>
                                                <span class={status_cls}>{ &o.status }</span>
                                            </div>
                                            <div class="order-card-body">
                                                <span class="order-date">
                                                    { o.created_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default() }
                                                </span>
                                                <span class="order-items-count">
                                                    { format!("{} item(s)", o.items.len()) }
                                                </span>
                                                <span class="order-total">
                                                    { format!("${:.2}", o.total) }
                                                </span>
                                            </div>
                                        </div>
                                    </Link<Route>>
                                }
                            })}
                        </div>
                        <Pagination
                            current_page={*page}
                            total_pages={*total_pages}
                            on_page_change={on_page}
                        />
                    </>
                }
            }}
        </div>
    }
}
