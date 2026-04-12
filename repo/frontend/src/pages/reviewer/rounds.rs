use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::ReviewRound;

#[function_component(ReviewerRoundsPage)]
pub fn reviewer_rounds_page() -> Html {
    let rounds = use_state(Vec::<ReviewRound>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);

    {
        let rounds = rounds.clone();
        let loading = loading.clone();
        let error = error.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                match api::reviews::list_rounds().await {
                    Ok(data) => rounds.set(data),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    html! {
        <div class="page-container">
            <h1 class="page-title">{ "Review Rounds" }</h1>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *loading {
                html! { <Loading /> }
            } else if (*rounds).is_empty() {
                html! {
                    <div class="empty-state">
                        <p>{ "No active review rounds at this time." }</p>
                    </div>
                }
            } else {
                html! {
                    <div class="rounds-grid">
                        { for (*rounds).iter().map(|round| {
                            let status_class = if round.is_active {
                                "badge badge-success"
                            } else {
                                "badge badge-secondary"
                            };
                            let status_label = if round.is_active { "Active" } else { "Closed" };
                            html! {
                                <div class="round-card">
                                    <div class="round-card-header">
                                        <h3>{ format!("Round #{}", round.round_number) }</h3>
                                        <span class={status_class}>{ status_label }</span>
                                    </div>
                                    <div class="round-card-body">
                                        <div class="info-row">
                                            <span class="info-label">{ "Product:" }</span>
                                            <span>{ &round.product_id[..8.min(round.product_id.len())] }{ "..." }</span>
                                        </div>
                                        { if !round.template_name.is_empty() {
                                            html! {
                                                <div class="info-row">
                                                    <span class="info-label">{ "Template:" }</span>
                                                    <span>{ &round.template_name }</span>
                                                </div>
                                            }
                                        } else { html! {} }}
                                        { if let Some(deadline) = round.deadline {
                                            html! {
                                                <div class="info-row">
                                                    <span class="info-label">{ "Deadline:" }</span>
                                                    <span>{ deadline.format("%Y-%m-%d %H:%M UTC").to_string() }</span>
                                                </div>
                                            }
                                        } else { html! {} }}
                                    </div>
                                    <div class="round-card-actions">
                                        <Link<Route>
                                            to={Route::ReviewerSubmit { id: round.id.clone() }}
                                            classes="btn btn-primary btn-sm"
                                        >
                                            { "Submit Review" }
                                        </Link<Route>>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }
            }}
        </div>
    }
}
