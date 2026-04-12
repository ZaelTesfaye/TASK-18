use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::components::pagination::Pagination;
use crate::types::{AuditLogEntry, AuditLogQuery};

#[function_component(AdminAuditLogPage)]
pub fn admin_audit_log_page() -> Html {
    let entries = use_state(Vec::<AuditLogEntry>::new);
    let page = use_state(|| 1u64);
    let total_pages = use_state(|| 1u64);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let selected_entry = use_state(|| Option::<AuditLogEntry>::None);

    let filter_actor = use_state(String::new);
    let filter_action = use_state(String::new);
    let filter_from = use_state(String::new);
    let filter_to = use_state(String::new);

    {
        let entries = entries.clone();
        let total_pages = total_pages.clone();
        let loading = loading.clone();
        let error = error.clone();
        let page = page.clone();
        let filter_actor = filter_actor.clone();
        let filter_action = filter_action.clone();
        let filter_from = filter_from.clone();
        let filter_to = filter_to.clone();

        let deps = (
            *page,
            (*filter_actor).clone(),
            (*filter_action).clone(),
            (*filter_from).clone(),
            (*filter_to).clone(),
        );

        use_effect_with(deps, move |deps| {
            let (pg, actor, action, from, to) = deps.clone();
            let entries = entries.clone();
            let total_pages = total_pages.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                let query = AuditLogQuery {
                    actor: if actor.is_empty() { None } else { Some(actor) },
                    action: if action.is_empty() { None } else { Some(action) },
                    from: if from.is_empty() { None } else { Some(from) },
                    to: if to.is_empty() { None } else { Some(to) },
                    page: Some(pg),
                };
                match api::admin::get_audit_log(&query).await {
                    Ok(resp) => {
                        entries.set(resp.items);
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

    let make_filter_input = |state: UseStateHandle<String>, placeholder: &str, input_type: &str| {
        let s = state.clone();
        let ph = placeholder.to_string();
        let it = input_type.to_string();
        let page = page.clone();
        html! {
            <input
                type={it}
                class="form-input"
                placeholder={ph}
                value={(*state).clone()}
                oninput={Callback::from(move |e: InputEvent| {
                    let el: HtmlInputElement = e.target_unchecked_into();
                    s.set(el.value());
                    page.set(1);
                })}
            />
        }
    };

    let on_row_click = {
        let selected_entry = selected_entry.clone();
        Callback::from(move |entry: AuditLogEntry| {
            let current = (*selected_entry).clone();
            if current.as_ref().map(|e| &e.id) == Some(&entry.id) {
                selected_entry.set(None);
            } else {
                selected_entry.set(Some(entry));
            }
        })
    };

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Audit Log" }</h1>
            </div>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="audit-filters">
                { make_filter_input(filter_actor.clone(), "Filter by actor...", "text") }
                { make_filter_input(filter_action.clone(), "Filter by action...", "text") }
                { make_filter_input(filter_from.clone(), "From date", "date") }
                { make_filter_input(filter_to.clone(), "To date", "date") }
            </div>

            { if *loading {
                html! { <Loading /> }
            } else if (*entries).is_empty() {
                html! { <p class="text-muted">{ "No audit log entries found." }</p> }
            } else {
                html! {
                    <>
                        <table class="table table-clickable">
                            <thead>
                                <tr>
                                    <th>{ "Timestamp" }</th>
                                    <th>{ "Actor" }</th>
                                    <th>{ "Action" }</th>
                                    <th>{ "Resource" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for (*entries).iter().map(|entry| {
                                    let e = entry.clone();
                                    let on_click = on_row_click.clone();
                                    let is_selected = (*selected_entry).as_ref().map(|s| &s.id) == Some(&entry.id);
                                    let cls = if is_selected { "table-row-selected" } else { "" };
                                    html! {
                                        <>
                                            <tr class={cls} onclick={Callback::from(move |_| on_click.emit(e.clone()))}>
                                                <td>
                                                    { entry.timestamp.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default() }
                                                </td>
                                                <td>{ &entry.actor }</td>
                                                <td><span class="badge badge-secondary">{ &entry.action }</span></td>
                                                <td>{ format!("{} ({})", entry.target_type.as_deref().unwrap_or("-"), &entry.target_id.as_deref().unwrap_or("-")[..8.min(entry.target_id.as_deref().unwrap_or("-").len())]) }</td>
                                            </tr>
                                            { if is_selected {
                                                html! {
                                                    <tr class="detail-row">
                                                        <td colspan="4">
                                                            <div class="audit-detail">
                                                                <h4>{ "Details" }</h4>
                                                                <pre class="code-block">
                                                                    { serde_json::to_string_pretty(&entry.details).unwrap_or_else(|_| entry.details.to_string()) }
                                                                </pre>
                                                                <div class="info-row">
                                                                    <span class="info-label">{ "Target ID:" }</span>
                                                                    <span>{ entry.target_id.as_deref().unwrap_or("-") }</span>
                                                                </div>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            } else { html! {} }}
                                        </>
                                    }
                                })}
                            </tbody>
                        </table>
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
