use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::{ReportQuery, ReportResponse};

#[function_component(AdminReportsPage)]
pub fn admin_reports_page() -> Html {
    let report = use_state(|| Option::<ReportResponse>::None);
    let loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);

    let report_type = use_state(|| "summary".to_string());
    let from_date = use_state(String::new);
    let to_date = use_state(String::new);

    let on_generate = {
        let report = report.clone();
        let loading = loading.clone();
        let error = error.clone();
        let report_type = report_type.clone();
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let report = report.clone();
            let loading = loading.clone();
            let error = error.clone();
            let rt = (*report_type).clone();
            let from = (*from_date).clone();
            let to = (*to_date).clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let query = ReportQuery {
                    report_type: Some(rt),
                    from: if from.is_empty() { None } else { Some(from) },
                    to: if to.is_empty() { None } else { Some(to) },
                };
                match api::admin::get_reports(&query).await {
                    Ok(r) => report.set(Some(r)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        })
    };

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Reports" }</h1>
            </div>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="card">
                <h2>{ "Generate Report" }</h2>
                <form onsubmit={on_generate} class="report-form">
                    <div class="form-row">
                        <div class="form-group">
                            <label class="form-label">{ "Report Type" }</label>
                            <select
                                class="form-select"
                                onchange={Callback::from(move |e: Event| {
                                    let el: HtmlSelectElement = e.target_unchecked_into();
                                    report_type.set(el.value());
                                })}
                            >
                                <option value="summary">{ "Summary" }</option>
                                <option value="detailed">{ "Detailed" }</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label class="form-label">{ "From" }</label>
                            <input
                                type="date"
                                class="form-input"
                                value={(*from_date).clone()}
                                oninput={Callback::from(move |e: InputEvent| {
                                    let el: HtmlInputElement = e.target_unchecked_into();
                                    from_date.set(el.value());
                                })}
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">{ "To" }</label>
                            <input
                                type="date"
                                class="form-input"
                                value={(*to_date).clone()}
                                oninput={Callback::from(move |e: InputEvent| {
                                    let el: HtmlInputElement = e.target_unchecked_into();
                                    to_date.set(el.value());
                                })}
                            />
                        </div>
                    </div>
                    <button type="submit" class="btn btn-primary" disabled={*loading}>
                        { if *loading { "Generating..." } else { "Generate Report" } }
                    </button>
                </form>
            </div>

            { if *loading {
                html! { <Loading message="Generating report..." /> }
            } else if let Some(ref r) = *report {
                html! {
                    <div class="report-results">
                        <h2>{ format!("{} Report", r.report_type) }</h2>
                        <p class="text-muted">{ format!("{} to {}", r.start_date, r.end_date) }</p>

                        <div class="stats-grid">
                            <div class="stat-card">
                                <div class="stat-value">{ r.orders.total }</div>
                                <div class="stat-label">{ "Total Orders" }</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{ format!("${:.2}", r.revenue.total_revenue) }</div>
                                <div class="stat-label">{ "Total Revenue" }</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{ format!("${:.2}", r.revenue.net_revenue) }</div>
                                <div class="stat-label">{ "Net Revenue" }</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{ r.users.total_users }</div>
                                <div class="stat-label">{ "Total Users" }</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{ r.users.new_users_in_period }</div>
                                <div class="stat-label">{ "New Users" }</div>
                            </div>
                            <div class="stat-card">
                                <div class="stat-value">{ r.ratings.total_ratings }</div>
                                <div class="stat-label">{ "Total Ratings" }</div>
                            </div>
                        </div>

                        { if !r.orders.by_status.is_empty() {
                            html! {
                                <div class="report-data">
                                    <h3>{ "Orders by Status" }</h3>
                                    <div class="status-breakdown">
                                        { for r.orders.by_status.iter().map(|sc| html! {
                                            <div class="status-row">
                                                <span class="badge badge-secondary">{ &sc.status }</span>
                                                <span>{ sc.count }</span>
                                            </div>
                                        })}
                                    </div>
                                </div>
                            }
                        } else { html! {} }}
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}
