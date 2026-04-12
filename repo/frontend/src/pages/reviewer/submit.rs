use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlTextAreaElement};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::{ReviewRound, ReviewSubmission, SubmitReviewRequest};

/// Reads files from a file input element and uploads them as attachments.
async fn upload_selected_files(
    submission_id: &str,
    uploaded_files: yew::UseStateHandle<Vec<String>>,
    error: yew::UseStateHandle<Option<String>>,
) {
    use wasm_bindgen::JsCast;
    use js_sys::Uint8Array;

    let doc = web_sys::window().unwrap().document().unwrap();
    let input = match doc.get_element_by_id("review-file-input") {
        Some(el) => el.dyn_into::<HtmlInputElement>().ok(),
        None => None,
    };

    let files = match input.and_then(|i| i.files()) {
        Some(f) => f,
        None => return,
    };

    let mut names = (*uploaded_files).clone();

    for i in 0..files.length().min(5) {
        if let Some(file) = files.get(i) {
            let fname = file.name();
            let mime = file.type_();

            // Read file bytes via FileReader
            let array_buffer = match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                Ok(ab) => ab,
                Err(_) => {
                    error.set(Some(format!("Failed to read file: {}", fname)));
                    continue;
                }
            };
            let bytes = Uint8Array::new(&array_buffer).to_vec();

            match api::reviews::upload_attachment(
                submission_id,
                &fname,
                &bytes,
                if mime.is_empty() { "application/octet-stream" } else { &mime },
            )
            .await
            {
                Ok(_info) => {
                    names.push(fname);
                    uploaded_files.set(names.clone());
                }
                Err(e) => {
                    error.set(Some(format!("Upload failed for {}: {}", fname, e)));
                }
            }
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub id: String,
}

#[function_component(ReviewerSubmitPage)]
pub fn reviewer_submit_page(props: &Props) -> Html {
    let navigator = use_navigator().unwrap();
    let round = use_state(|| Option::<ReviewRound>::None);
    let submissions = use_state(Vec::<ReviewSubmission>::new);
    let loading = use_state(|| true);
    let submitting = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| false);
    let field_values = use_state(|| serde_json::Map::new());
    let uploaded_files = use_state(Vec::<String>::new);

    let round_id = props.id.clone();

    // Fetch round and past submissions
    {
        let round = round.clone();
        let submissions = submissions.clone();
        let loading = loading.clone();
        let error = error.clone();
        let round_id = round_id.clone();
        use_effect_with(round_id.clone(), move |rid| {
            let rid = rid.clone();
            let round = round.clone();
            let submissions = submissions.clone();
            let loading = loading.clone();
            let error = error.clone();
            spawn_local(async move {
                match api::reviews::get_round(&rid).await {
                    Ok(r) => round.set(Some(r)),
                    Err(e) => error.set(Some(e.to_string())),
                }
                if let Ok(subs) = api::reviews::list_submissions(&rid).await {
                    submissions.set(subs);
                }
                loading.set(false);
            });
            || ()
        });
    }

    let on_field_change = {
        let field_values = field_values.clone();
        Callback::from(move |(name, value): (String, String)| {
            let mut map = (*field_values).clone();
            map.insert(name, serde_json::Value::String(value));
            field_values.set(map);
        })
    };

    let on_submit = {
        let field_values = field_values.clone();
        let round_id = round_id.clone();
        let submitting = submitting.clone();
        let error = error.clone();
        let success = success.clone();
        let uploaded_files = uploaded_files.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            // Content must not be empty
            if (*field_values).is_empty() {
                error.set(Some("Please fill in at least one field.".into()));
                return;
            }

            let req = SubmitReviewRequest {
                content: serde_json::Value::Object((*field_values).clone()),
            };
            let round_id = round_id.clone();
            let submitting = submitting.clone();
            let error = error.clone();
            let success = success.clone();
            let uploaded_files = uploaded_files.clone();
            let navigator = navigator.clone();
            submitting.set(true);
            error.set(None);
            spawn_local(async move {
                match api::reviews::submit_review(&round_id, &req).await {
                    Ok(submission) => {
                        // Upload any selected files now that we have a submission ID
                        upload_selected_files(
                            &submission.id,
                            uploaded_files,
                            error.clone(),
                        ).await;
                        success.set(true);
                        navigator.push(&Route::ReviewerRounds);
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

    let r = match &*round {
        Some(r) => r,
        None => return html! {
            <div class="page-container">
                { if let Some(ref err) = *error {
                    html! { <div class="alert alert-error">{ err }</div> }
                } else {
                    html! { <div class="alert alert-error">{ "Round not found" }</div> }
                }}
                <Link<Route> to={Route::ReviewerRounds} classes="btn btn-secondary">
                    { "Back to Rounds" }
                </Link<Route>>
            </div>
        },
    };

    html! {
        <div class="page-container">
            <Link<Route> to={Route::ReviewerRounds} classes="btn btn-secondary btn-sm">
                { "\u{2190} Back to Rounds" }
            </Link<Route>>

            <h1 class="page-title">{ format!("Submit Review — Round #{}", r.round_number) }</h1>

            { if !r.template_name.is_empty() {
                html! { <p class="text-muted">{ format!("Template: {}", r.template_name) }</p> }
            } else { html! {} }}

            { if let Some(deadline) = r.deadline {
                html! {
                    <div class="alert alert-warning">
                        { format!("Deadline: {}", deadline.format("%Y-%m-%d %H:%M UTC")) }
                    </div>
                }
            } else { html! {} }}

            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *success {
                html! { <div class="alert alert-success">{ "Review submitted successfully!" }</div> }
            } else { html! {} }}

            <form onsubmit={on_submit} class="review-form">
                { // Render fields from template schema if available, otherwise fall back to defaults
                    let template_fields: Vec<(String, String, String)> = r.template_schema
                        .as_ref()
                        .and_then(|s| s.as_object())
                        .map(|schema_obj| {
                            schema_obj.iter().map(|(key, def)| {
                                let label = key.chars().next()
                                    .map(|c| c.to_uppercase().to_string() + &key[1..])
                                    .unwrap_or_else(|| key.clone());
                                let field_type = def.get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("string");
                                let render_as = if field_type == "string" && key.contains("strength")
                                    || key.contains("weakness") || key.contains("detail") {
                                    "textarea"
                                } else {
                                    "text"
                                };
                                (key.clone(), label, render_as.to_string())
                            }).collect::<Vec<_>>()
                        })
                        .unwrap_or_else(|| vec![
                            ("summary".to_string(), "Summary".to_string(), "text".to_string()),
                            ("strengths".to_string(), "Strengths".to_string(), "textarea".to_string()),
                            ("weaknesses".to_string(), "Weaknesses".to_string(), "textarea".to_string()),
                            ("recommendation".to_string(), "Recommendation".to_string(), "text".to_string()),
                        ]);
                    html! { <>
                    { for template_fields.into_iter().map(|(name, label, ftype)| {
                        let current = (*field_values)
                            .get(&name)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let on_change = on_field_change.clone();
                        let fname = name.to_string();

                        html! {
                            <div class="form-group">
                                <label class="form-label">{ label }</label>
                                { if ftype == "textarea" {
                                    let fname = fname.clone();
                                    html! {
                                        <textarea
                                            class="form-textarea"
                                            rows="4"
                                            value={current}
                                            oninput={Callback::from(move |e: InputEvent| {
                                                let el: HtmlTextAreaElement = e.target_unchecked_into();
                                                on_change.emit((fname.clone(), el.value()));
                                            })}
                                            disabled={*submitting}
                                        ></textarea>
                                    }
                                } else {
                                    let fname = fname.clone();
                                    html! {
                                        <input
                                            type="text"
                                            class="form-input"
                                            value={current}
                                            oninput={Callback::from(move |e: InputEvent| {
                                                let el: HtmlInputElement = e.target_unchecked_into();
                                                on_change.emit((fname.clone(), el.value()));
                                            })}
                                            disabled={*submitting}
                                        />
                                    }
                                }}
                            </div>
                        }
                    })}
                    </> }
                }

                // File upload area — files are uploaded after submission via the attachment API
                <div class="form-group">
                    <label class="form-label">{ "Attachments (max 5 files, PDF/PNG/JPG, 10MB each)" }</label>
                    <div class="file-upload-area">
                        <input
                            type="file"
                            id="review-file-input"
                            multiple={true}
                            accept=".pdf,.png,.jpg,.jpeg"
                            class="file-input"
                            disabled={*submitting}
                        />
                        <p class="file-upload-hint">{ "Selected files will be uploaded automatically when you submit" }</p>
                    </div>
                    { if !(*uploaded_files).is_empty() {
                        html! {
                            <ul class="uploaded-files-list">
                                { for (*uploaded_files).iter().map(|f| html! {
                                    <li>{ f }</li>
                                })}
                            </ul>
                        }
                    } else { html! {} }}
                </div>

                <button
                    type="submit"
                    class="btn btn-primary"
                    disabled={*submitting}
                >
                    { if *submitting { "Submitting..." } else { "Submit Review" } }
                </button>
            </form>

            // Version history
            { if !(*submissions).is_empty() {
                html! {
                    <div class="version-history">
                        <h2>{ "Previous Submissions" }</h2>
                        { for (*submissions).iter().map(|sub| html! {
                            <div class="submission-card">
                                <div class="submission-header">
                                    <span class="badge badge-secondary">{ format!("v{}", sub.version) }</span>
                                    <span class={match sub.status.to_lowercase().as_str() {
                                        "approved" => "badge badge-success",
                                        "rejected" => "badge badge-danger",
                                        _ => "badge badge-warning",
                                    }}>{ &sub.status }</span>
                                    { if let Some(dt) = sub.created_at {
                                        html! { <span class="text-muted">{ dt.format("%Y-%m-%d %H:%M").to_string() }</span> }
                                    } else { html! {} }}
                                </div>
                            </div>
                        })}
                    </div>
                }
            } else { html! {} }}
        </div>
    }
}
