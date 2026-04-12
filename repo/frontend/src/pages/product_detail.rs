use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::components::pagination::Pagination;
use crate::components::rating_stars::RatingStars;
use crate::store;
use crate::types::{AddToCartRequest, Product, Rating};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[function_component(ProductDetailPage)]
pub fn product_detail_page(props: &Props) -> Html {
    let product = use_state(|| Option::<Product>::None);
    let ratings = use_state(Vec::<Rating>::new);
    let ratings_page = use_state(|| 1u64);
    let ratings_total_pages = use_state(|| 1u64);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let cart_loading = use_state(|| false);
    let cart_msg = use_state(|| Option::<String>::None);

    let id = props.id.clone();

    // Fetch product
    {
        let product = product.clone();
        let loading = loading.clone();
        let error = error.clone();
        let id = id.clone();
        use_effect_with(id.clone(), move |id| {
            let id = id.clone();
            let product = product.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::products::get_product(&id).await {
                    Ok(p) => product.set(Some(p)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
            || ()
        });
    }

    // Fetch ratings
    {
        let ratings = ratings.clone();
        let ratings_total_pages = ratings_total_pages.clone();
        let id = id.clone();
        let ratings_page = ratings_page.clone();
        use_effect_with((*ratings_page, id.clone()), move |(page, id)| {
            let ratings = ratings.clone();
            let ratings_total_pages = ratings_total_pages.clone();
            let id = id.clone();
            let page = *page;
            spawn_local(async move {
                if let Ok(resp) = api::ratings::get_product_ratings(&id, page).await {
                    ratings.set(resp.items);
                    ratings_total_pages.set(resp.total_pages);
                }
            });
            || ()
        });
    }

    let on_add_to_cart = {
        let cart_loading = cart_loading.clone();
        let cart_msg = cart_msg.clone();
        let error = error.clone();
        let id = id.clone();
        Callback::from(move |_: MouseEvent| {
            if !store::is_authenticated() {
                error.set(Some("Please log in to add items to cart.".into()));
                return;
            }
            let cart_loading = cart_loading.clone();
            let cart_msg = cart_msg.clone();
            let error = error.clone();
            let id = id.clone();
            cart_loading.set(true);
            spawn_local(async move {
                let req = AddToCartRequest {
                    product_id: id,
                    quantity: 1,
                };
                match api::cart::add_to_cart(&req).await {
                    Ok(_) => cart_msg.set(Some("Added to cart!".into())),
                    Err(e) => error.set(Some(e.to_string())),
                }
                cart_loading.set(false);
            });
        })
    };

    let on_ratings_page = {
        let ratings_page = ratings_page.clone();
        Callback::from(move |p: u64| ratings_page.set(p))
    };

    if *loading {
        return html! { <Loading /> };
    }

    if let Some(ref err) = *error {
        if (*product).is_none() {
            return html! {
                <div class="page-container">
                    <div class="alert alert-error">{ err }</div>
                    <Link<Route> to={Route::Catalog} classes="btn btn-secondary">{ "Back to Catalog" }</Link<Route>>
                </div>
            };
        }
    }

    let p = match &*product {
        Some(p) => p,
        None => return html! {
            <div class="page-container">
                <div class="alert alert-error">{ "Product not found" }</div>
            </div>
        },
    };

    let score = p.aggregate_score.unwrap_or(0.0);

    html! {
        <div class="page-container">
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}
            { if let Some(ref msg) = *cart_msg {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}

            <div class="product-detail">
                <div class="product-detail-image">
                    { if let Some(ref url) = p.image_url {
                        html! { <img src={url.clone()} alt={p.title.clone()} class="detail-img" /> }
                    } else {
                        html! {
                            <div class="detail-placeholder">
                                <span>{ "\u{1F3AC}" }</span>
                            </div>
                        }
                    }}
                </div>

                <div class="product-detail-info">
                    <h1 class="detail-title">{ &p.title }</h1>

                    { if !p.genre.is_empty() {
                        html! { <span class="badge badge-secondary">{ &p.genre }</span> }
                    } else { html! {} }}

                    <div class="detail-rating">
                        <RatingStars score={score} show_value={true} />
                    </div>

                    <div class="detail-price">{ format!("${:.2}", p.price) }</div>

                    <p class="detail-description">{ &p.description }</p>

                    { if !p.topics.is_empty() {
                        html! {
                            <div class="detail-section">
                                <h3>{ "Topics" }</h3>
                                <div class="tag-chips">
                                    { for p.topics.iter().map(|t| html! {
                                        <span class="chip">{ &t.name }</span>
                                    })}
                                </div>
                            </div>
                        }
                    } else { html! {} }}

                    { if !p.tags.is_empty() {
                        html! {
                            <div class="detail-section">
                                <h3>{ "Tags" }</h3>
                                <div class="tag-chips">
                                    { for p.tags.iter().map(|t| html! {
                                        <span class="chip">{ &t.name }</span>
                                    })}
                                </div>
                            </div>
                        }
                    } else { html! {} }}

                    // Dimension scores are displayed per-rating below, not on the product itself

                    { if !p.custom_fields.is_null() && p.custom_fields.is_object() {
                        html! {
                            <div class="detail-section">
                                <h3>{ "Details" }</h3>
                                <table class="table table-sm">
                                    { if let Some(obj) = p.custom_fields.as_object() {
                                        html! {
                                            <tbody>
                                                { for obj.iter().map(|(k, v)| html! {
                                                    <tr>
                                                        <td class="td-label">{ k }</td>
                                                        <td>{ v.as_str().unwrap_or(&v.to_string()) }</td>
                                                    </tr>
                                                })}
                                            </tbody>
                                        }
                                    } else { html! {} }}
                                </table>
                            </div>
                        }
                    } else { html! {} }}

                    <div class="detail-actions">
                        <button
                            class="btn btn-primary"
                            onclick={on_add_to_cart}
                            disabled={*cart_loading}
                        >
                            { if *cart_loading { "Adding..." } else { "Add to Cart" } }
                        </button>
                        { if store::is_authenticated() {
                            html! {
                                <Link<Route> to={Route::RateProduct { product_id: props.id.clone() }} classes="btn btn-secondary">
                                    { "Rate This Product" }
                                </Link<Route>>
                            }
                        } else { html! {} }}
                    </div>
                </div>
            </div>

            // Ratings section
            <div class="ratings-section">
                <h2>{ "Reviews" }</h2>
                { if (*ratings).is_empty() {
                    html! { <p class="text-muted">{ "No reviews yet." }</p> }
                } else {
                    html! {
                        <>
                            { for (*ratings).iter().map(|r| html! {
                                <div class="rating-card">
                                    <div class="rating-header">
                                        <span class="rating-author">{ format!("User: {}...", &r.user_id[..8.min(r.user_id.len())]) }</span>
                                        <RatingStars score={r.average} show_value={true} />
                                        { if let Some(dt) = r.created_at {
                                            html! { <span class="rating-date">{ dt.format("%Y-%m-%d").to_string() }</span> }
                                        } else { html! {} }}
                                    </div>
                                    { if !r.dimensions.is_empty() {
                                        html! {
                                            <div class="rating-dimensions">
                                                { for r.dimensions.iter().map(|d| html! {
                                                    <span class="dimension-chip">
                                                        { format!("{}: {}", d.dimension_name, d.score) }
                                                    </span>
                                                })}
                                            </div>
                                        }
                                    } else { html! {} }}
                                </div>
                            })}
                            <Pagination
                                current_page={*ratings_page}
                                total_pages={*ratings_total_pages}
                                on_page_change={on_ratings_page}
                            />
                        </>
                    }
                }}
            </div>
        </div>
    }
}
