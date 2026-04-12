use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

use crate::api;
use crate::components::loading::Loading;
use crate::components::pagination::Pagination;
use crate::components::product_card::ProductCard;
use crate::types::{Product, ProductFilter, Topic, Tag};

/// Recursively renders topic options for the select dropdown.
/// Each level is indented with non-breaking spaces.
fn render_topic_options(topic: &Topic, depth: usize, selected_id: &str) -> Html {
    let indent = "\u{00A0}\u{00A0}".repeat(depth);
    let label = format!("{}{}", indent, &topic.name);
    let mut opts = vec![html! {
        <option value={topic.id.clone()} selected={selected_id == topic.id}>{ label }</option>
    }];
    for child in &topic.children {
        opts.push(render_topic_options(child, depth + 1, selected_id));
    }
    html! { <>{for opts}</> }
}

#[function_component(HomePage)]
pub fn home_page() -> Html {
    let products = use_state(Vec::<Product>::new);
    let total_pages = use_state(|| 1u64);
    let current_page = use_state(|| 1u64);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let toast_msg = use_state(|| Option::<String>::None);

    let search = use_state(String::new);
    let genre = use_state(String::new);
    let topic_id = use_state(String::new);
    let tag_filter = use_state(String::new);
    let min_price = use_state(String::new);
    let max_price = use_state(String::new);
    let custom_field_name = use_state(String::new);
    let custom_field_value = use_state(String::new);

    let topics = use_state(Vec::<Topic>::new);
    let tags = use_state(Vec::<Tag>::new);

    // Load topics and tags on mount
    {
        let topics = topics.clone();
        let tags = tags.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(t) = api::admin::list_topics().await {
                    topics.set(t);
                }
                if let Ok(t) = api::admin::list_tags().await {
                    tags.set(t);
                }
            });
            || ()
        });
    }

    // Fetch products when filters or page change
    {
        let products = products.clone();
        let total_pages = total_pages.clone();
        let loading = loading.clone();
        let error = error.clone();
        let search = search.clone();
        let genre = genre.clone();
        let topic_id = topic_id.clone();
        let tag_filter = tag_filter.clone();
        let min_price = min_price.clone();
        let max_price = max_price.clone();
        let custom_field_name = custom_field_name.clone();
        let custom_field_value = custom_field_value.clone();
        let current_page = current_page.clone();

        let deps = (
            (*search).clone(),
            (*genre).clone(),
            (*topic_id).clone(),
            (*tag_filter).clone(),
            (*min_price).clone(),
            (*max_price).clone(),
            (*custom_field_name).clone(),
            (*custom_field_value).clone(),
            *current_page,
        );
        use_effect_with(deps, move |deps| {
            let (s, g, t, tg, mn, mx, cfn, cfv, pg) = deps.clone();
            let products = products.clone();
            let total_pages = total_pages.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let filter = ProductFilter {
                    search: if s.is_empty() { None } else { Some(s) },
                    genre: if g.is_empty() { None } else { Some(g) },
                    topic_id: if t.is_empty() { None } else { Some(t) },
                    tag_id: if tg.is_empty() { None } else { Some(tg) },
                    min_price: mn.parse().ok(),
                    max_price: mx.parse().ok(),
                    custom_field_name: if cfn.is_empty() { None } else { Some(cfn) },
                    custom_field_value: if cfv.is_empty() { None } else { Some(cfv) },
                    page: Some(pg),
                    per_page: Some(12),
                };
                match api::products::list_products(&filter).await {
                    Ok(resp) => {
                        products.set(resp.items);
                        total_pages.set(resp.total_pages);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                    }
                }
                loading.set(false);
            });
            || ()
        });
    }

    let on_search = {
        let search = search.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            search.set(el.value());
            current_page.set(1);
        })
    };

    let on_genre = {
        let genre = genre.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            genre.set(el.value());
            current_page.set(1);
        })
    };

    let on_topic = {
        let topic_id = topic_id.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: Event| {
            let el: HtmlSelectElement = e.target_unchecked_into();
            topic_id.set(el.value());
            current_page.set(1);
        })
    };

    let on_min_price = {
        let min_price = min_price.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            min_price.set(el.value());
            current_page.set(1);
        })
    };

    let on_max_price = {
        let max_price = max_price.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            max_price.set(el.value());
            current_page.set(1);
        })
    };

    let on_page_change = {
        let current_page = current_page.clone();
        Callback::from(move |page: u64| {
            current_page.set(page);
        })
    };

    let on_custom_field_name = {
        let custom_field_name = custom_field_name.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            custom_field_name.set(el.value());
            current_page.set(1);
        })
    };

    let on_custom_field_value = {
        let custom_field_value = custom_field_value.clone();
        let current_page = current_page.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            custom_field_value.set(el.value());
            current_page.set(1);
        })
    };

    let on_tag_click = {
        let tag_filter = tag_filter.clone();
        let current_page = current_page.clone();
        Callback::from(move |tag: String| {
            let current = (*tag_filter).clone();
            if current == tag {
                tag_filter.set(String::new());
            } else {
                tag_filter.set(tag);
            }
            current_page.set(1);
        })
    };

    let on_added_to_cart = {
        let toast_msg = toast_msg.clone();
        Callback::from(move |_: String| {
            toast_msg.set(Some("Added to cart!".into()));
        })
    };

    let on_card_error = {
        let error = error.clone();
        Callback::from(move |msg: String| {
            error.set(Some(msg));
        })
    };

    let genres = vec!["Action", "Comedy", "Drama", "Horror", "Sci-Fi", "Romance", "Thriller", "Documentary", "Animation", "Fantasy"];

    html! {
        <div class="catalog-page">
            <div class="catalog-sidebar">
                <h2 class="sidebar-title">{ "Filters" }</h2>

                <div class="form-group">
                    <label class="form-label">{ "Search" }</label>
                    <input
                        type="text"
                        class="form-input"
                        placeholder="Search products..."
                        value={(*search).clone()}
                        oninput={on_search}
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">{ "Genre" }</label>
                    <select class="form-select" onchange={on_genre} value={(*genre).clone()}>
                        <option value="">{ "All Genres" }</option>
                        { for genres.iter().map(|g| html! {
                            <option value={g.to_string()} selected={**genre == *g}>{ g }</option>
                        })}
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">{ "Topic" }</label>
                    <select class="form-select" onchange={on_topic} value={(*topic_id).clone()}>
                        <option value="">{ "All Topics" }</option>
                        { for (*topics).iter().map(|t| render_topic_options(t, 0, &topic_id)) }
                    </select>
                </div>

                <div class="form-group">
                    <label class="form-label">{ "Custom Field Filter" }</label>
                    <input
                        type="text"
                        class="form-input"
                        placeholder="Field slug (e.g. director)"
                        value={(*custom_field_name).clone()}
                        oninput={on_custom_field_name}
                    />
                    <input
                        type="text"
                        class="form-input"
                        placeholder="Field value"
                        value={(*custom_field_value).clone()}
                        oninput={on_custom_field_value}
                        style="margin-top: 0.25rem;"
                    />
                </div>

                <div class="form-group">
                    <label class="form-label">{ "Price Range" }</label>
                    <div class="price-range">
                        <input
                            type="number"
                            class="form-input"
                            placeholder="Min"
                            value={(*min_price).clone()}
                            oninput={on_min_price}
                            min="0"
                            step="0.01"
                        />
                        <span class="price-separator">{ "-" }</span>
                        <input
                            type="number"
                            class="form-input"
                            placeholder="Max"
                            value={(*max_price).clone()}
                            oninput={on_max_price}
                            min="0"
                            step="0.01"
                        />
                    </div>
                </div>

                { if !(*tags).is_empty() {
                    html! {
                        <div class="form-group">
                            <label class="form-label">{ "Tags" }</label>
                            <div class="tag-chips">
                                { for (*tags).iter().map(|t| {
                                    let tag_id = t.id.clone();
                                    let active = *tag_filter == t.id;
                                    let on_click = on_tag_click.clone();
                                    let cls = if active { "chip chip-active" } else { "chip" };
                                    html! {
                                        <button
                                            class={cls}
                                            onclick={Callback::from(move |_| on_click.emit(tag_id.clone()))}
                                        >
                                            { &t.name }
                                        </button>
                                    }
                                })}
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}
            </div>

            <div class="catalog-content">
                <h1 class="page-title">{ "Catalog" }</h1>

                { if let Some(ref msg) = *toast_msg {
                    html! { <div class="alert alert-success">{ msg }</div> }
                } else {
                    html! {}
                }}

                { if let Some(ref err) = *error {
                    html! { <div class="alert alert-error">{ err }</div> }
                } else {
                    html! {}
                }}

                { if *loading {
                    html! { <Loading /> }
                } else if (*products).is_empty() {
                    html! {
                        <div class="empty-state">
                            <p>{ "No products found. Try adjusting your filters." }</p>
                        </div>
                    }
                } else {
                    html! {
                        <>
                            <div class="product-grid">
                                { for (*products).iter().map(|p| html! {
                                    <ProductCard
                                        product={p.clone()}
                                        on_added_to_cart={on_added_to_cart.clone()}
                                        on_error={on_card_error.clone()}
                                    />
                                })}
                            </div>
                            <Pagination
                                current_page={*current_page}
                                total_pages={*total_pages}
                                on_page_change={on_page_change}
                            />
                        </>
                    }
                }}
            </div>
        </div>
    }
}
