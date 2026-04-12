use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::app::Route;
use crate::types::RegisterRequest;

#[function_component(RegisterPage)]
pub fn register_page() -> Html {
    let navigator = use_navigator().unwrap();
    let username = use_state(String::new);
    let email = use_state(String::new);
    let password = use_state(String::new);
    let confirm = use_state(String::new);
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let on_username = {
        let s = username.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            s.set(el.value());
        })
    };
    let on_email = {
        let s = email.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            s.set(el.value());
        })
    };
    let on_password = {
        let s = password.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            s.set(el.value());
        })
    };
    let on_confirm = {
        let s = confirm.clone();
        Callback::from(move |e: InputEvent| {
            let el: HtmlInputElement = e.target_unchecked_into();
            s.set(el.value());
        })
    };

    let on_submit = {
        let username = username.clone();
        let email = email.clone();
        let password = password.clone();
        let confirm = confirm.clone();
        let error = error.clone();
        let loading = loading.clone();
        let navigator = navigator.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let u = (*username).clone();
            let em = (*email).clone();
            let p = (*password).clone();
            let c = (*confirm).clone();

            if u.is_empty() || em.is_empty() || p.is_empty() {
                error.set(Some("All fields are required.".into()));
                return;
            }
            if p.len() < 8 {
                error.set(Some("Password must be at least 8 characters, with at least one uppercase letter, one lowercase letter, one digit, and one special character.".into()));
                return;
            }
            if !p.chars().any(|c| c.is_uppercase()) {
                error.set(Some("Password must contain at least one uppercase letter.".into()));
                return;
            }
            if !p.chars().any(|c| c.is_lowercase()) {
                error.set(Some("Password must contain at least one lowercase letter.".into()));
                return;
            }
            if !p.chars().any(|c| c.is_ascii_digit()) {
                error.set(Some("Password must contain at least one digit.".into()));
                return;
            }
            if p.chars().all(|c| c.is_alphanumeric()) {
                error.set(Some("Password must contain at least one special character.".into()));
                return;
            }
            if p != c {
                error.set(Some("Passwords do not match.".into()));
                return;
            }
            if !em.contains('@') {
                error.set(Some("Please enter a valid email address.".into()));
                return;
            }

            let error = error.clone();
            let loading = loading.clone();
            let navigator = navigator.clone();
            loading.set(true);
            error.set(None);
            spawn_local(async move {
                let req = RegisterRequest {
                    username: u,
                    email: em,
                    password: p,
                };
                match api::auth::register(&req).await {
                    Ok(_) => {
                        navigator.push(&Route::Login);
                    }
                    Err(e) => {
                        error.set(Some(e.to_string()));
                    }
                }
                loading.set(false);
            });
        })
    };

    let password_mismatch = !(*confirm).is_empty() && *password != *confirm;

    html! {
        <div class="page-center">
            <div class="auth-card">
                <h1 class="auth-title">{ "Create Account" }</h1>
                <p class="auth-subtitle">{ "Join SilverScreen today" }</p>

                { if let Some(ref err) = *error {
                    html! { <div class="alert alert-error">{ err }</div> }
                } else {
                    html! {}
                }}

                <form onsubmit={on_submit}>
                    <div class="form-group">
                        <label for="reg-username" class="form-label">{ "Username" }</label>
                        <input
                            id="reg-username"
                            type="text"
                            class="form-input"
                            placeholder="Choose a username"
                            value={(*username).clone()}
                            oninput={on_username}
                            disabled={*loading}
                        />
                    </div>
                    <div class="form-group">
                        <label for="reg-email" class="form-label">{ "Email" }</label>
                        <input
                            id="reg-email"
                            type="email"
                            class="form-input"
                            placeholder="your@email.com"
                            value={(*email).clone()}
                            oninput={on_email}
                            disabled={*loading}
                        />
                    </div>
                    <div class="form-group">
                        <label for="reg-password" class="form-label">{ "Password" }</label>
                        <input
                            id="reg-password"
                            type="password"
                            class="form-input"
                            placeholder="Min 8 chars, upper+lower+digit+special"
                            value={(*password).clone()}
                            oninput={on_password}
                            disabled={*loading}
                        />
                    </div>
                    <div class="form-group">
                        <label for="reg-confirm" class="form-label">{ "Confirm Password" }</label>
                        <input
                            id="reg-confirm"
                            type="password"
                            class={if password_mismatch { "form-input input-error" } else { "form-input" }}
                            placeholder="Re-enter your password"
                            value={(*confirm).clone()}
                            oninput={on_confirm}
                            disabled={*loading}
                        />
                        { if password_mismatch {
                            html! { <span class="form-error">{ "Passwords do not match" }</span> }
                        } else {
                            html! {}
                        }}
                    </div>
                    <button
                        type="submit"
                        class="btn btn-primary btn-full"
                        disabled={*loading}
                    >
                        { if *loading { "Creating account..." } else { "Create Account" } }
                    </button>
                </form>

                <p class="auth-footer">
                    { "Already have an account? " }
                    <Link<Route> to={Route::Login}>{ "Sign In" }</Link<Route>>
                </p>
            </div>
        </div>
    }
}
