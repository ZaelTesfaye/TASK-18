use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::components::loading::Loading;
use crate::components::pagination::Pagination;
use crate::types::User;

#[function_component(AdminUsersPage)]
pub fn admin_users_page() -> Html {
    let users = use_state(Vec::<User>::new);
    let page = use_state(|| 1u64);
    let total_pages = use_state(|| 1u64);
    let loading = use_state(|| true);
    let error = use_state(|| Option::<String>::None);
    let action_loading = use_state(|| Option::<String>::None);
    let success_msg = use_state(|| Option::<String>::None);

    let fetch = {
        let users = users.clone();
        let total_pages = total_pages.clone();
        let loading = loading.clone();
        let error = error.clone();
        let page = page.clone();
        move || {
            let users = users.clone();
            let total_pages = total_pages.clone();
            let loading = loading.clone();
            let error = error.clone();
            let pg = *page;
            loading.set(true);
            spawn_local(async move {
                match api::admin::list_users(pg).await {
                    Ok(resp) => {
                        users.set(resp.items);
                        total_pages.set(resp.total_pages);
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                loading.set(false);
            });
        }
    };

    {
        let fetch = fetch.clone();
        use_effect_with(*page, move |_| {
            fetch();
            || ()
        });
    }

    let on_page = {
        let page = page.clone();
        Callback::from(move |p: u64| page.set(p))
    };

    let on_change_role = {
        let error = error.clone();
        let action_loading = action_loading.clone();
        let success_msg = success_msg.clone();
        let fetch = fetch.clone();
        Callback::from(move |(user_id, role): (String, String)| {
            let error = error.clone();
            let action_loading = action_loading.clone();
            let success_msg = success_msg.clone();
            let fetch = fetch.clone();
            action_loading.set(Some(user_id.clone()));
            spawn_local(async move {
                match api::admin::change_role(&user_id, &role).await {
                    Ok(_) => {
                        success_msg.set(Some(format!("Role changed to {}", role)));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    let on_reset_password = {
        let error = error.clone();
        let action_loading = action_loading.clone();
        let success_msg = success_msg.clone();
        Callback::from(move |user_id: String| {
            let error = error.clone();
            let action_loading = action_loading.clone();
            let success_msg = success_msg.clone();
            action_loading.set(Some(user_id.clone()));
            spawn_local(async move {
                match api::admin::reset_password(&user_id).await {
                    Ok(_) => success_msg.set(Some("Password reset successfully.".into())),
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    let on_unlock = {
        let error = error.clone();
        let action_loading = action_loading.clone();
        let success_msg = success_msg.clone();
        let fetch = fetch.clone();
        Callback::from(move |user_id: String| {
            let error = error.clone();
            let action_loading = action_loading.clone();
            let success_msg = success_msg.clone();
            let fetch = fetch.clone();
            action_loading.set(Some(user_id.clone()));
            spawn_local(async move {
                match api::admin::unlock_user(&user_id).await {
                    Ok(_) => {
                        success_msg.set(Some("User unlocked.".into()));
                        fetch();
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                action_loading.set(None);
            });
        })
    };

    let role_class = |role: &str| -> &str {
        match role.to_lowercase().as_str() {
            "admin" => "badge badge-danger",
            "reviewer" => "badge badge-info",
            "shopper" | "customer" | "user" => "badge badge-secondary",
            _ => "badge badge-secondary",
        }
    };

    html! {
        <div class="page-container">
            <div class="page-header">
                <Link<Route> to={Route::Admin} classes="btn btn-secondary btn-sm">
                    { "\u{2190} Dashboard" }
                </Link<Route>>
                <h1 class="page-title">{ "User Management" }</h1>
            </div>

            { if let Some(ref msg) = *success_msg {
                html! { <div class="alert alert-success">{ msg }</div> }
            } else { html! {} }}
            { if let Some(ref err) = *error {
                html! { <div class="alert alert-error">{ err }</div> }
            } else { html! {} }}

            { if *loading {
                html! { <Loading /> }
            } else {
                html! {
                    <>
                        <table class="table">
                            <thead>
                                <tr>
                                    <th>{ "Username" }</th>
                                    <th>{ "Email" }</th>
                                    <th>{ "Role" }</th>
                                    <th>{ "Status" }</th>
                                    <th>{ "Actions" }</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for (*users).iter().map(|user| {
                                    let uid = user.id.clone();
                                    let is_acting = (*action_loading).as_ref() == Some(&uid);
                                    let rc = role_class(&user.role);
                                    let change_role = on_change_role.clone();
                                    let reset = on_reset_password.clone();
                                    let unlock = on_unlock.clone();
                                    let uid_for_role = uid.clone();
                                    let uid_for_reset = uid.clone();
                                    let uid_for_unlock = uid.clone();
                                    let locked = user.locked;

                                    html! {
                                        <tr>
                                            <td>{ &user.username }</td>
                                            <td>{ &user.email }</td>
                                            <td><span class={rc}>{ &user.role }</span></td>
                                            <td>
                                                { if user.locked {
                                                    html! { <span class="badge badge-danger">{ "Locked" }</span> }
                                                } else {
                                                    html! { <span class="badge badge-success">{ "Active" }</span> }
                                                }}
                                            </td>
                                            <td class="actions-cell">
                                                <select
                                                    class="form-select form-select-sm"
                                                    disabled={is_acting}
                                                    onchange={Callback::from(move |e: Event| {
                                                        let el: HtmlSelectElement = e.target_unchecked_into();
                                                        let val = el.value();
                                                        if !val.is_empty() {
                                                            change_role.emit((uid_for_role.clone(), val));
                                                        }
                                                    })}
                                                >
                                                    <option value="">{ "Change Role" }</option>
                                                    <option value="Shopper">{ "Shopper" }</option>
                                                    <option value="Reviewer">{ "Reviewer" }</option>
                                                    <option value="Admin">{ "Admin" }</option>
                                                </select>
                                                <button
                                                    class="btn btn-sm btn-secondary"
                                                    disabled={is_acting}
                                                    onclick={Callback::from(move |_| reset.emit(uid_for_reset.clone()))}
                                                >
                                                    { "Reset Pwd" }
                                                </button>
                                                { if locked {
                                                    html! {
                                                        <button
                                                            class="btn btn-sm btn-warning"
                                                            disabled={is_acting}
                                                            onclick={Callback::from(move |_| unlock.emit(uid_for_unlock.clone()))}
                                                        >
                                                            { "Unlock" }
                                                        </button>
                                                    }
                                                } else { html! {} }}
                                            </td>
                                        </tr>
                                    }
                                })}
                            </tbody>
                        </table>
                        <Pagination
                            current_page={*page}
                            total_pages={*total_pages}
                            on_page_change={on_page}
                        />
                    </>
                }
            }}
        </div>
    }
}
