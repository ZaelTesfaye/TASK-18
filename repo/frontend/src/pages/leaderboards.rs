use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::api;
use crate::components::loading::Loading;
use crate::components::rating_stars::RatingStars;
use crate::types::{LeaderboardEntry, LeaderboardQuery};

#[function_component(LeaderboardsPage)]
pub fn leaderboards_page() -> Html {
    let entries = use_state(Vec::<LeaderboardEntry>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let period = use_state(|| "monthly".to_string());
    let genre = use_state(String::new);

    {
        let entries = entries.clone();
        let loading = loading.clone();
        let error = error.clone();
        let period = period.clone();
        let genre = genre.clone();
        let deps = ((*period).clone(), (*genre).clone());
        use_effect_with(deps, move |(p, g)| {
            let entries = entries.clone();
            let loading = loading.clone();
            let error = error.clone();
            let p = p.clone();
            let g = g.clone();
            loading.set(true);
            spawn_local(async move {
                let query = LeaderboardQuery {
                    period: Some(p),
                    genre: if g.is_empty() { None } else { Some(g) },
                    page: Some(1),
                    per_page: Some(50),
                };
                match api::ratings::get_leaderboard(&query).await {
                    Ok(paginated) => entries.set(paginated.items),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    let on_period = {
        let period = period.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            period.set(el.value());
        })
    };

    let on_genre = {
        let genre = genre.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            genre.set(el.value());
        })
    };

    let genres = vec!["Action", "Comedy", "Drama", "Horror", "Sci-Fi", "Romance", "Thriller", "Documentary", "Animation", "Fantasy"];

    html! {
        <div class="page-container">
            <h1 class="page-title">{ "Leaderboards" }</h1>

            <div class="leaderboard-filters">
                <div class="form-group form-inline">
                    <label class="form-label">{ "Period:" }</label>
                    <select class="form-select" onchange={on_period}>
                        <option value="weekly" selected={*period == "weekly"}>{ "Weekly" }</option>
                        <option value="monthly" selected={*period == "monthly"}>{ "Monthly" }</option>
                    </select>
                </div>
                <div class="form-group form-inline">
                    <label class="form-label">{ "Genre:" }</label>
                    <select class="form-select" onchange={on_genre}>
                        <option value="">{ "All Genres" }</option>
                        { for genres.iter().map(|g| html! {
                            <option value={g.to_string()} selected={**genre == *g}>{ g }</option>
                        })}
                    </select>
                </div>
            </div>

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *loading {
                html! { <Loading /> }
            } else if (*entries).is_empty() {
                html! {
                    <div class="empty-state">
                        <p>{ "No leaderboard data available for the selected criteria." }</p>
                    </div>
                }
            } else {
                html! {
                    <table class="table">
                        <thead>
                            <tr>
                                <th>{ "Rank" }</th>
                                <th>{ "Product" }</th>
                                <th>{ "Avg Score" }</th>
                                <th>{ "Total Ratings" }</th>
                                <th>{ "Last Activity" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for (*entries).iter().enumerate().map(|(i, entry)| {
                                let rank = if entry.rank > 0 { entry.rank } else { (i + 1) as u64 };
                                let rank_class = match rank {
                                    1 => "rank rank-gold",
                                    2 => "rank rank-silver",
                                    3 => "rank rank-bronze",
                                    _ => "rank",
                                };
                                html! {
                                    <tr>
                                        <td><span class={rank_class}>{ rank }</span></td>
                                        <td>{ &entry.product_title }</td>
                                        <td>
                                            <RatingStars score={entry.average_score} show_value={true} />
                                        </td>
                                        <td>{ entry.total_ratings }</td>
                                        <td>
                                            { entry.last_activity
                                                .map(|d| d.format("%Y-%m-%d").to_string())
                                                .unwrap_or_else(|| "-".to_string())
                                            }
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
