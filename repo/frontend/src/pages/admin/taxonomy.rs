use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::{CreateTagRequest, CreateTopicRequest, Tag, Topic};

#[function_component(AdminTaxonomyPage)]
pub fn admin_taxonomy_page() -> Html {
    let topics = use_state(Vec::<Topic>::new);
    let tags = use_state(Vec::<Tag>::new);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| Option::<String>::None);

    let new_topic_name = use_state(String::new);
    let new_topic_parent = use_state(String::new);
    let new_tag_name = use_state(String::new);
    let delete_replacement = use_state(String::new);
    let action_loading = use_state(|| false);

    let fetch_all = {
        let topics = topics.clone();
        let tags = tags.clone();
        let loading = loading.clone();
        let error = error.clone();
        move || {
            let topics = topics.clone();
            let tags = tags.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::admin::list_topics().await {
                    Ok(t) => topics.set(t),
                    Err(e) => error.set(Some(e.to_string())),
                }
                match api::admin::list_tags().await {
                    Ok(t) => tags.set(t),
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    };

    {
        let fetch = fetch_all.clone();
        use_effect_with((), move |_| {
            fetch();
            || ()
        });
    }

    let on_add_topic = {
        let new_topic_name = new_topic_name.clone();
        let new_topic_parent = new_topic_parent.clone();
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        let fetch = fetch_all.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_topic_name).clone();
            if name.is_empty() {
                error.set(Some("Topic name is required.".into()));
                return;
            }
            let parent = (*new_topic_parent).clone();
            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            let fetch = fetch.clone();
            let new_topic_name = new_topic_name.clone();
            action_loading.set(true);
            spawn_local(async move {
                let req = CreateTopicRequest {
                    name,
                    parent_id: if parent.is_empty() { None } else { Some(parent) },
                };
                match api::admin::create_topic(&req).await {
                    Ok(_) => {
                        success.set(Some("Topic created.".into()));
                        new_topic_name.set(String::new());
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    let on_delete_topic = {
        let error = error.clone();
        let success = success.clone();
        let delete_replacement = delete_replacement.clone();
        let fetch = fetch_all.clone();
        Callback::from(move |topic_id: String| {
            let error = error.clone();
            let success = success.clone();
            let replacement = (*delete_replacement).clone();
            let fetch = fetch.clone();
            spawn_local(async move {
                let repl = if replacement.is_empty() {
                    None
                } else {
                    Some(replacement.as_str())
                };
                match api::admin::delete_topic(&topic_id, repl).await {
                    Ok(_) => {
                        success.set(Some("Topic deleted.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    let on_add_tag = {
        let new_tag_name = new_tag_name.clone();
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        let fetch = fetch_all.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let name = (*new_tag_name).clone();
            if name.is_empty() {
                error.set(Some("Tag name is required.".into()));
                return;
            }
            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            let fetch = fetch.clone();
            let new_tag_name = new_tag_name.clone();
            action_loading.set(true);
            spawn_local(async move {
                let req = CreateTagRequest { name };
                match api::admin::create_tag(&req).await {
                    Ok(_) => {
                        success.set(Some("Tag created.".into()));
                        new_tag_name.set(String::new());
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(false);
            });
        })
    };

    let on_delete_tag = {
        let error = error.clone();
        let success = success.clone();
        let fetch = fetch_all.clone();
        Callback::from(move |tag_id: String| {
            let error = error.clone();
            let success = success.clone();
            let fetch = fetch.clone();
            spawn_local(async move {
                match api::admin::delete_tag(&tag_id).await {
                    Ok(_) => {
                        success.set(Some("Tag deleted.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            });
        })
    };

    /// Recursively render topics as a tree.
    fn render_topic_tree(
        topics: &[Topic],
        depth: u32,
        on_delete: &Callback<String>,
        delete_replacement: &UseStateHandle<String>,
        all_topics: &[Topic],
    ) -> Html {
        html! {
            <ul class="topic-tree">
                { for topics.iter().map(|t| {
                    let tid = t.id.clone();
                    let on_del = on_delete.clone();
                    let del_rep = delete_replacement.clone();
                    let tid_del = tid.clone();
                    let all = all_topics.to_vec();
                    html! {
                        <li class="topic-tree-item" style={format!("margin-left: {}px", depth * 16)}>
                            <div class="topic-row">
                                <span class="topic-name">{ &t.name }</span>
                                <div class="topic-actions">
                                    <select
                                        class="form-select form-select-sm"
                                        onchange={Callback::from(move |e: Event| {
                                            let el: HtmlSelectElement = e.target_unchecked_into();
                                            del_rep.set(el.value());
                                        })}
                                    >
                                        <option value="">{ "No replacement" }</option>
                                        { for all.iter().filter(|x| x.id != tid).map(|x| html! {
                                            <option value={x.id.clone()}>{ &x.name }</option>
                                        })}
                                    </select>
                                    <button
                                        class="btn btn-sm btn-danger"
                                        onclick={Callback::from(move |_| on_del.emit(tid_del.clone()))}
                                    >
                                        { "Delete" }
                                    </button>
                                </div>
                            </div>
                            { if !t.children.is_empty() {
                                render_topic_tree(&t.children, depth + 1, on_delete, delete_replacement, &all)
                            } else { html! {} }}
                        </li>
                    }
                })}
            </ul>
        }
    }

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Taxonomy Management" }</h1>
            </div>

            { if let Some(ref msg) = *success {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *loading {
                html! { <Loading /> }
            } else {
                html! {
                    <div class="taxonomy-layout">
                        // Topics
                        <div class="taxonomy-section">
                            <h2>{ "Topics" }</h2>
                            <form onsubmit={on_add_topic} class="inline-form">
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="New topic name"
                                    value={(*new_topic_name).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let el: HtmlInputElement = e.target_unchecked_into();
                                        new_topic_name.set(el.value());
                                    })}
                                    disabled={*action_loading}
                                />
                                <select
                                    class="form-select"
                                    onchange={Callback::from(move |e: Event| {
                                        let el: HtmlSelectElement = e.target_unchecked_into();
                                        new_topic_parent.set(el.value());
                                    })}
                                >
                                    <option value="">{ "No Parent (root)" }</option>
                                    { for (*topics).iter().map(|t| html! {
                                        <option value={t.id.clone()}>{ &t.name }</option>
                                    })}
                                </select>
                                <button type="submit" class="btn btn-primary btn-sm" disabled={*action_loading}>
                                    { "Add Topic" }
                                </button>
                            </form>

                            { if (*topics).is_empty() {
                                html! { <p class="text-muted">{ "No topics yet." }</p> }
                            } else {
                                let flat: Vec<Topic> = (*topics).clone();
                                render_topic_tree(&*topics, 0, &on_delete_topic, &delete_replacement, &flat)
                            }}
                        </div>

                        // Tags
                        <div class="taxonomy-section">
                            <h2>{ "Tags" }</h2>
                            <form onsubmit={on_add_tag} class="inline-form">
                                <input
                                    type="text"
                                    class="form-input"
                                    placeholder="New tag name"
                                    value={(*new_tag_name).clone()}
                                    oninput={Callback::from(move |e: InputEvent| {
                                        let el: HtmlInputElement = e.target_unchecked_into();
                                        new_tag_name.set(el.value());
                                    })}
                                    disabled={*action_loading}
                                />
                                <button type="submit" class="btn btn-primary btn-sm" disabled={*action_loading}>
                                    { "Add Tag" }
                                </button>
                            </form>

                            { if (*tags).is_empty() {
                                html! { <p class="text-muted">{ "No tags yet." }</p> }
                            } else {
                                html! {
                                    <div class="tag-list">
                                        { for (*tags).iter().map(|tag| {
                                            let tid = tag.id.clone();
                                            let on_del = on_delete_tag.clone();
                                            html! {
                                                <div class="tag-item">
                                                    <span class="chip">{ &tag.name }</span>
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={Callback::from(move |_| on_del.emit(tid.clone()))}
                                                    >
                                                        { "\u{00D7}" }
                                                    </button>
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            }}
                        </div>
                    </div>
                }
            }}
        </div>
    }
}
