use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::components::rating_stars::RatingStars;
use crate::types::{CreateRatingRequest, DimensionScore, Product};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub product_id: String,
}

const DIMENSIONS: &[&str] = &["Plot", "Acting", "Visuals", "Soundtrack", "Dialogue", "Pacing"];

#[function_component(RateProductPage)]
pub fn rate_product_page(props: &Props) -> Html {
    let navigator = use_navigator().unwrap();
    let product = use_state(|| Option::<Product>::None);
    let loading = use_state(|| true);
    let submitting = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| false);

    // Dimension scores: Vec<(dimension_name, score)>
    let scores = use_state(|| {
        DIMENSIONS
            .iter()
            .map(|d| (d.to_string(), 5u32))
            .collect::<Vec<_>>()
    });

    let pid = props.product_id.clone();

    // Fetch product
    {
        let product = product.clone();
        let loading = loading.clone();
        let error = error.clone();
        let pid = pid.clone();
        use_effect_with(pid.clone(), move |pid| {
            let pid = pid.clone();
            let product = product.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::products::get_product(&pid).await {
                    Ok(p) => product.set(Some(p)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    let overall_score = {
        let s = &*scores;
        if s.is_empty() {
            5.0
        } else {
            s.iter().map(|(_, v)| *v as f64).sum::<f64>() / s.len() as f64
        }
    };

    let on_dimension_change = {
        let scores = scores.clone();
        Callback::from(move |(dim, val): (String, u32)| {
            let mut current = (*scores).clone();
            if let Some(entry) = current.iter_mut().find(|(d, _)| *d == dim) {
                entry.1 = val;
            }
            scores.set(current);
        })
    };

    let on_submit = {
        let scores = scores.clone();
        let pid = pid.clone();
        let submitting = submitting.clone();
        let error = error.clone();
        let success = success.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let dims: Vec<DimensionScore> = (*scores)
                .iter()
                .map(|(d, s)| DimensionScore {
                    dimension_name: d.clone(),
                    score: *s,
                })
                .collect();
            let req = CreateRatingRequest {
                product_id: pid.clone(),
                dimensions: dims,
            };
            let submitting = submitting.clone();
            let error = error.clone();
            let success = success.clone();
            let navigator = navigator.clone();
            let pid = pid.clone();
            submitting.set(true);
            error.set(None);
            spawn_local(async move {
                match api::ratings::create_rating(&req).await {
                    Ok(_) => {
                        success.set(true);
                        // Navigate to product detail after a moment
                        navigator.push(&Route::ProductDetail { id: pid });
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                submitting.set(false);
            });
        })
    };

    if *loading {
        return html! { <Loading /> };
    }

    html! {
        <div class="page-container">
            <Link<Route> to={Route::ProductDetail { id: pid.clone() }} classes="btn btn-secondary btn-sm">
                { "\u{2190} Back to Product" }
            </Link<Route>>

            <h1 class="page-title">{ "Rate Product" }</h1>

            { if let Some(ref p) = *product {
                html! {
                    <div class="rate-product-header">
                        <h2>{ &p.title }</h2>
                        { if let Some(agg) = p.aggregate_score {
                            html! {
                                <div>
                                    <span class="text-muted">{ "Current average: " }</span>
                                    <RatingStars score={agg} show_value={true} />
                                </div>
                            }
                        } else { html! {} }}
                    </div>
                }
            } else { html! {} }}

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *success {
                html! { <div class="alert alert-success">{ "Rating submitted successfully!" }</div> }
            } else { html! {} }}

            <form onsubmit={on_submit} class="rating-form">
                <div class="overall-score-display">
                    <span class="overall-label">{ "Overall Score: " }</span>
                    <RatingStars score={overall_score} show_value={true} />
                </div>

                <div class="dimension-scores">
                    { for (*scores).iter().map(|(dim, val)| {
                        let dim_name = dim.clone();
                        let current_val = *val;
                        let on_change = on_dimension_change.clone();
                        html! {
                            <div class="dimension-row">
                                <label class="dimension-label">{ &dim_name }</label>
                                <input
                                    type="range"
                                    min="1"
                                    max="10"
                                    step="1"
                                    value={current_val.to_string()}
                                    class="dimension-slider"
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let el: HtmlInputElement = e.target_unchecked_into();
                                        if let Ok(v) = el.value().parse::<u32>() {
                                            on_change.emit((dim_name.clone(), v));
                                        }
                                    })}
                                    disabled={*submitting}
                                />
                                <span class="dimension-value">{ format!("{}", current_val) }</span>
                            </div>
                        }
                    })}
                </div>

                <button
                    type="submit"
                    class="btn btn-primary"
                    disabled={*submitting}
                >
                    { if *submitting { "Submitting..." } else { "Submit Rating" } }
                </button>
            </form>
        </div>
    }
}
