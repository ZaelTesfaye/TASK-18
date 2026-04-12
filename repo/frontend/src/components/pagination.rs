use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PaginationProps {
    pub current_page: u64,
    pub total_pages: u64,
    pub on_page_change: Callback<u64>,
}

#[function_component(Pagination)]
pub fn pagination(props: &PaginationProps) -> Html {
    if props.total_pages <= 1 {
        return html! {};
    }

    let current = props.current_page;
    let total = props.total_pages;
    let on_change = props.on_page_change.clone();

    // Build page numbers: show first, last, and a window around current
    let mut pages: Vec<u64> = Vec::new();
    let window = 2u64;
    let start = if current > window + 1 { current - window } else { 1 };
    let end = if current + window < total { current + window } else { total };

    if start > 1 {
        pages.push(1);
        if start > 2 {
            pages.push(0); // ellipsis marker
        }
    }
    for p in start..=end {
        pages.push(p);
    }
    if end < total {
        if end < total - 1 {
            pages.push(0);
        }
        pages.push(total);
    }

    let prev_cb = {
        let on_change = on_change.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if current > 1 {
                on_change.emit(current - 1);
            }
        })
    };

    let next_cb = {
        let on_change = on_change.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            if current < total {
                on_change.emit(current + 1);
            }
        })
    };

    html! {
        <nav class="pagination">
            <button
                class="pagination-btn"
                disabled={current <= 1}
                onclick={prev_cb}
            >
                { "\u{2190} Prev" }
            </button>
            { for pages.iter().map(|&p| {
                if p == 0 {
                    html! { <span class="pagination-ellipsis">{ "..." }</span> }
                } else {
                    let on_change = on_change.clone();
                    let is_active = p == current;
                    let cls = if is_active { "pagination-btn pagination-active" } else { "pagination-btn" };
                    html! {
                        <button
                            class={cls}
                            disabled={is_active}
                            onclick={Callback::from(move |e: MouseEvent| {
                                e.prevent_default();
                                on_change.emit(p);
                            })}
                        >
                            { p }
                        </button>
                    }
                }
            })}
            <button
                class="pagination-btn"
                disabled={current >= total}
                onclick={next_cb}
            >
                { "Next \u{2192}" }
            </button>
        </nav>
    }
}
