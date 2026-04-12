use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::Rating;

/// Admin moderation page. Fetches pending ratings and allows approve/reject
/// via POST /api/admin/moderation/ratings/{id}.
#[function_component(AdminModerationPage)]
pub fn admin_moderation_page() -> Html {
    let items = use_state(Vec::<Rating>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| Option::<String>::None);
    let action_loading = use_state(|| Option::<String>::None);
    let rating_id_input = use_state(String::new);

    // Note: there is no dedicated "list pending ratings" endpoint.
    // An admin can get ratings per product. This page provides a manual
    // moderation interface by rating ID.

    {
        let loading = loading.clone();
        use_effect_with((), move |_| {
            loading.set(false);
            || ()
        });
    }

    let on_moderate = {
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        Callback::from(move |(rating_id, status): (String, String)| {
            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            action_loading.set(Some(rating_id.clone()));
            spawn_local(async move {
                match api::admin::moderate_rating(&rating_id, &status).await {
                    Ok(_) => success.set(Some(format!("Rating {} {}.", &rating_id[..8.min(rating_id.len())], status))),
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    let on_id_input = {
        let rating_id_input = rating_id_input.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            rating_id_input.set(el.value());
        })
    };

    let on_approve = {
        let rating_id_input = rating_id_input.clone();
        let on_moderate = on_moderate.clone();
        Callback::from(move |_: MouseEvent| {
            let id = (*rating_id_input).clone();
            if !id.is_empty() {
                on_moderate.emit((id, "Approved".into()));
            }
        })
    };

    let on_reject = {
        let rating_id_input = rating_id_input.clone();
        let on_moderate = on_moderate.clone();
        Callback::from(move |_: MouseEvent| {
            let id = (*rating_id_input).clone();
            if !id.is_empty() {
                on_moderate.emit((id, "Rejected".into()));
            }
        })
    };

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Moderation" }</h1>
            </div>

            { if let Some(ref msg) = *success {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="card">
                <h3>{ "Moderate Rating by ID" }</h3>
                <div class="form-group">
                    <label class="form-label">{ "Rating ID" }</label>
                    <input
                        type="text"
                        class="form-input"
                        placeholder="Enter rating UUID"
                        value={(*rating_id_input).clone()}
                        oninput={on_id_input}
                    />
                </div>
                <div class="btn-group">
                    <button
                        class="btn btn-success"
                        onclick={on_approve}
                        disabled={(*rating_id_input).is_empty() || (*action_loading).is_some()}
                    >
                        { "Approve" }
                    </button>
                    <button
                        class="btn btn-danger"
                        onclick={on_reject}
                        disabled={(*rating_id_input).is_empty() || (*action_loading).is_some()}
                    >
                        { "Reject" }
                    </button>
                </div>
            </div>
        </div>
    }
}
