use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::CreateFieldRequest;
use crate::types::CustomFieldDefinition;

#[function_component(AdminFieldsPage)]
pub fn admin_fields_page() -> Html {
    let fields = use_state(Vec::<CustomFieldDefinition>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| Option::<String>::None);
    let action_loading = use_state(|| false);

    // New field form
    let new_name = use_state(String::new);
    let new_type = use_state(|| "Text".to_string());
    let new_options = use_state(String::new);

    let fetch = {
        let fields = fields.clone();
        let loading = loading.clone();
        let error = error.clone();
        move || {
            let fields = fields.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::admin::list_fields().await {
                    Ok(f) => fields.set(f),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    };

    {
        let fetch = fetch.clone();
        use_effect_with((), move |_| {
            fetch();
            || ()
        });
    }

    let on_create = {
        let new_name = new_name.clone();
        let new_type = new_type.clone();
        let new_options = new_options.clone();
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        let fetch = fetch.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_name).clone();
            if name.is_empty() {
                error.set(Some("Field name is required.".into()));
                return;
            }
            let field_type = (*new_type).clone();
            let opts_str = (*new_options).clone();
            let allowed_values: Option<Vec<String>> = if opts_str.is_empty() {
                None
            } else {
                Some(opts_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            };

            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            let fetch = fetch.clone();
            let new_name = new_name.clone();
            let new_options = new_options.clone();
            action_loading.set(true);
            spawn_local(async move {
                let req = CreateFieldRequest {
                    name,
                    field_type,
                    allowed_values,
                };
                match api::admin::create_field(&req).await {
                    Ok(_) => {
                        success.set(Some("Field created.".into()));
                        new_name.set(String::new());
                        new_options.set(String::new());
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    let on_publish = {
        let error = error.clone();
        let success = success.clone();
        let fetch = fetch.clone();
        Callback::from(move |field_id: String| {
            let error = error.clone();
            let success = success.clone();
            let fetch = fetch.clone();
            spawn_local(async move {
                match api::admin::publish_field(&field_id).await {
                    Ok(_) => {
                        success.set(Some("Field published.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let status_class = |status: &str| -> &str {
        match status.to_lowercase().as_str() {
            "published" | "active" => "badge badge-success",
            "draft" => "badge badge-warning",
            "deprecated" => "badge badge-danger",
            _ => "badge badge-secondary",
        }
    };

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Custom Field Management" }</h1>
            </div>

            { if let Some(ref msg) = *success {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="card">
                <h2>{ "Create New Field" }</h2>
                <form onsubmit={on_create} class="field-create-form">
                    <div class="form-row">
                        <div class="form-group">
                            <label class="form-label">{ "Name" }</label>
                            <input
                                type="text"
                                class="form-input"
                                placeholder="Field name"
                                value={(*new_name).clone()}
                                oninput={Callback::from(move |e: InputEvent| {
                                    let el: HtmlInputElement = e.target_unchecked_into();
                                    new_name.set(el.value());
                                })}
                                disabled={*action_loading}
                            />
                        </div>
                        <div class="form-group">
                            <label class="form-label">{ "Type" }</label>
                            <select
                                class="form-select"
                                onchange={Callback::from(move |e: Event| {
                                    let el: HtmlSelectElement = e.target_unchecked_into();
                                    new_type.set(el.value());
                                })}
                                disabled={*action_loading}
                            >
                                <option value="Text">{ "Text" }</option>
                                <option value="Number">{ "Number" }</option>
                                <option value="Enum">{ "Enum" }</option>
                                <option value="Date">{ "Date" }</option>
                            </select>
                        </div>
                    </div>
                    <div class="form-group">
                        <label class="form-label">{ "Allowed Values (comma-separated, for Enum type)" }</label>
                        <input
                            type="text"
                            class="form-input"
                            placeholder="Option1, Option2, Option3"
                            value={(*new_options).clone()}
                            oninput={Callback::from(move |e: InputEvent| {
                                let el: HtmlInputElement = e.target_unchecked_into();
                                new_options.set(el.value());
                            })}
                            disabled={*action_loading}
                        />
                    </div>
                    <button type="submit" class="btn btn-primary" disabled={*action_loading}>
                        { "Create Field" }
                    </button>
                </form>
            </div>

            { if *loading {
                html! { <Loading /> }
            } else if (*fields).is_empty() {
                html! { <p class="text-muted">{ "No custom fields defined yet." }</p> }
            } else {
                html! {
                    <table class="table">
                        <thead>
                            <tr>
                                <th>{ "Name" }</th>
                                <th>{ "Type" }</th>
                                <th>{ "Status" }</th>
                                <th>{ "Allowed Values" }</th>
                                <th>{ "Actions" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for (*fields).iter().map(|f| {
                                let sc = status_class(&f.status);
                                let fid = f.id.clone();
                                let on_pub = on_publish.clone();
                                let is_draft = f.status.to_lowercase() == "draft";
                                let vals_display = f.allowed_values.as_ref()
                                    .and_then(|v| v.as_array())
                                    .map(|arr| arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect::<Vec<_>>()
                                        .join(", "))
                                    .unwrap_or_default();
                                html! {
                                    <tr>
                                        <td>{ &f.name }</td>
                                        <td>{ &f.field_type }</td>
                                        <td><span class={sc}>{ &f.status }</span></td>
                                        <td>{ vals_display }</td>
                                        <td>
                                            { if is_draft {
                                                html! {
                                                    <button
                                                        class="btn btn-sm btn-primary"
                                                        onclick={Callback::from(move |_| on_pub.emit(fid.clone()))}
                                                    >
                                                        { "Publish" }
                                                    </button>
                                                }
                                            } else { html! {} }}
                                            { if f.conflict_count > 0 {
                                                html! {
                                                    <span class="text-muted">
                                                        { format!("{} conflict(s)", f.conflict_count) }
                                                    </span>
                                                }
                                            } else { html! {} }}
                                        </td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>
                }
            }}
        </div>
    }
}
