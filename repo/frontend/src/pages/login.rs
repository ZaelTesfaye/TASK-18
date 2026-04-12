use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::store;
use crate::types::LoginRequest;

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let navigator = use_navigator().unwrap();
    let username = use_state(String::new);
    let password = use_state(String::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let on_username = {
        let username = username.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            username.set(input.value());
        })
    };

    let on_password = {
        let password = password.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    let on_submit = {
        let username = username.clone();
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let username_val = (*username).clone();
            let password_val = (*password).clone();
            if username_val.is_empty() || password_val.is_empty() {
                error.set(Some("Please enter both username and password.".into()));
                return;
            }
            let error = error.clone();
            let loading = loading.clone();
            let navigator = navigator.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let req = LoginRequest {
                    username: username_val,
                    password: password_val,
                };
                match api::auth::login(&req).await {
                    Ok(resp) => {
                        store::save_tokens(&resp.access_token, &resp.refresh_token);
                        if let Some(user) = resp.user {
                            store::save_user(&user);
                        } else {
                            // Fetch user profile
                            if let Ok(user) = api::auth::get_current_user().await {
                                store::save_user(&user);
                            }
                        }
                        navigator.push(&Route::Catalog);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                    }
                }
                loading.set(false);
            });
        })
    };

    html! {
        <div class="page-center">
            <div class="auth-card">
                <h1 class="auth-title">{ "Sign In" }</h1>
                <p class="auth-subtitle">{ "Welcome back to SilverScreen" }</p>

                { if let Some(ref err) = *error {
                    html! { <div class="alert alert-error">{ err }</div> }
                } else {
                    html! {}
                }}

                <form onsubmit={on_submit}>
                    <div class="form-group">
                        <label for="username" class="form-label">{ "Username" }</label>
                        <input
                            id="username"
                            type="text"
                            class="form-input"
                            placeholder="Enter your username"
                            value={(*username).clone()}
                            oninput={on_username}
                            disabled={*loading}
                        />
                    </div>
                    <div class="form-group">
                        <label for="password" class="form-label">{ "Password" }</label>
                        <input
                            id="password"
                            type="password"
                            class="form-input"
                            placeholder="Enter your password"
                            value={(*password).clone()}
                            oninput={on_password}
                            disabled={*loading}
                        />
                    </div>
                    <button
                        type="submit"
                        class="btn btn-primary btn-full"
                        disabled={*loading}
                    >
                        { if *loading { "Signing in..." } else { "Sign In" } }
                    </button>
                </form>

                <p class="auth-footer">
                    { "Don't have an account? " }
                    <Link<Route> to={Route::Register}>{ "Register" }</Link<Route>>
                </p>
            </div>
        </div>
    }
}
