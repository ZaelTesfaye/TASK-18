use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::types::BackupResponse;

#[function_component(AdminBackupPage)]
pub fn admin_backup_page() -> Html {
    let backups = use_state(Vec::<BackupResponse>::new);
    let loading = use_state(|| true);
    let creating = use_state(|| false);
    let error = use_state(|| Option::<String>::None);
    let success = use_state(|| Option::<String>::None);
    let action_loading = use_state(|| Option::<String>::None);

    let fetch = {
        let backups = backups.clone();
        let loading = loading.clone();
        let error = error.clone();
        move || {
            let backups = backups.clone();
            let loading = loading.clone();
            let error = error.clone();
            loading.set(true);
            spawn_local(async move {
                match api::admin::list_backups().await {
                    Ok(b) => backups.set(b),
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
        let creating = creating.clone();
        let error = error.clone();
        let success = success.clone();
        let fetch = fetch.clone();
        Callback::from(move |_: MouseEvent| {
            let creating = creating.clone();
            let error = error.clone();
            let success = success.clone();
            let fetch = fetch.clone();
            creating.set(true);
            spawn_local(async move {
                match api::admin::create_backup().await {
                    Ok(_) => {
                        success.set(Some("Backup created successfully.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                creating.set(false);
            });
        })
    };

    let on_verify = {
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        let fetch = fetch.clone();
        Callback::from(move |backup_id: String| {
            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            let fetch = fetch.clone();
            action_loading.set(Some(backup_id.clone()));
            spawn_local(async move {
                match api::admin::verify_backup(&backup_id).await {
                    Ok(_) => {
                        success.set(Some("Backup verified.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    let on_restore = {
        let error = error.clone();
        let success = success.clone();
        let action_loading = action_loading.clone();
        Callback::from(move |backup_id: String| {
            let error = error.clone();
            let success = success.clone();
            let action_loading = action_loading.clone();
            action_loading.set(Some(backup_id.clone()));
            spawn_local(async move {
                match api::admin::restore_backup(&backup_id).await {
                    Ok(_) => {
                        success.set(Some("Backup restore initiated.".into()));
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    fn format_size(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "Backup Management" }</h1>
            </div>

            { if let Some(ref msg) = *success {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            <div class="backup-actions">
                <button
                    class="btn btn-primary"
                    onclick={on_create}
                    disabled={*creating}
                >
                    { if *creating { "Creating Backup..." } else { "Create New Backup" } }
                </button>
            </div>

            { if *loading {
                html! { <Loading /> }
            } else if (*backups).is_empty() {
                html! { <p class="text-muted">{ "No backups available." }</p> }
            } else {
                html! {
                    <table class="table">
                        <thead>
                            <tr>
                                <th>{ "ID" }</th>
                                <th>{ "Filename" }</th>
                                <th>{ "Status" }</th>
                                <th>{ "Size" }</th>
                                <th>{ "Created" }</th>
                                <th>{ "Verified" }</th>
                                <th>{ "Actions" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for (*backups).iter().map(|b| {
                                let bid = b.id.clone();
                                let is_acting = (*action_loading).as_ref() == Some(&bid);
                                let on_ver = on_verify.clone();
                                let on_res = on_restore.clone();
                                let bid_v = bid.clone();
                                let bid_r = bid.clone();
                                let status_cls = match b.status.to_lowercase().as_str() {
                                    "completed" | "ready" => "badge badge-success",
                                    "in_progress" | "creating" => "badge badge-warning",
                                    "failed" => "badge badge-danger",
                                    _ => "badge badge-secondary",
                                };
                                html! {
                                    <tr>
                                        <td class="text-mono">{ &b.id[..8.min(b.id.len())] }</td>
                                        <td>{ &b.filename }</td>
                                        <td><span class={status_cls}>{ &b.status }</span></td>
                                        <td>{ format_size(b.size_bytes) }</td>
                                        <td>
                                            { b.created_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default() }
                                        </td>
                                        <td>
                                            { if b.verified {
                                                html! { <span class="badge badge-success">{ "Yes" }</span> }
                                            } else {
                                                html! { <span class="badge badge-secondary">{ "No" }</span> }
                                            }}
                                        </td>
                                        <td class="actions-cell">
                                            <button
                                                class="btn btn-sm btn-secondary"
                                                disabled={is_acting}
                                                onclick={Callback::from(move |_| on_ver.emit(bid_v.clone()))}
                                            >
                                                { "Verify" }
                                            </button>
                                            <button
                                                class="btn btn-sm btn-warning"
                                                disabled={is_acting}
                                                onclick={Callback::from(move |_| on_res.emit(bid_r.clone()))}
                                            >
                                                { "Restore" }
                                            </button>
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
