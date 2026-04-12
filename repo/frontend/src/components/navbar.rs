use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::store;

#[derive(Properties, PartialEq)]
pub struct NavbarProps {
    #[prop_or_default]
    pub cart_count: u32,
    #[prop_or_default]
    pub on_logout: Callback<()>,
}

#[function_component(Navbar)]
pub fn navbar(props: &NavbarProps) -> Html {
    let menu_open = use_state(|| false);
    let authenticated = store::is_authenticated();
    let user = store::get_user();
    let role = store::get_role().unwrap_or_default();

    let toggle_menu = {
        let menu_open = menu_open.clone();
        Callback::from(move |_: MouseEvent| {
            menu_open.set(!*menu_open);
        })
    };

    let close_menu = {
        let menu_open = menu_open.clone();
        Callback::from(move |_: MouseEvent| {
            menu_open.set(false);
        })
    };

    let on_logout = {
        let on_logout = props.on_logout.clone();
        let menu_open = menu_open.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            menu_open.set(false);
            on_logout.emit(());
        })
    };

    let nav_class = if *menu_open { "nav-links nav-open" } else { "nav-links" };

    html! {
        <nav class="navbar">
            <div class="navbar-container">
                <Link<Route> to={Route::Home} classes="navbar-brand">
                    { "\u{1F3AC} SilverScreen" }
                </Link<Route>>

                <button class="navbar-toggle" onclick={toggle_menu}>
                    { "\u{2630}" }
                </button>

                <div class={nav_class}>
                    <Link<Route> to={Route::Catalog} classes="nav-link" >
                        <span onclick={close_menu.clone()}>{ "Catalog" }</span>
                    </Link<Route>>

                    { if authenticated {
                        html! {
                            <>
                                <Link<Route> to={Route::Cart} classes="nav-link">
                                    <span onclick={close_menu.clone()}>
                                        { "Cart" }
                                        { if props.cart_count > 0 {
                                            html! { <span class="badge badge-primary cart-badge">{ props.cart_count }</span> }
                                        } else {
                                            html! {}
                                        }}
                                    </span>
                                </Link<Route>>
                                <Link<Route> to={Route::Orders} classes="nav-link">
                                    <span onclick={close_menu.clone()}>{ "Orders" }</span>
                                </Link<Route>>
                                <Link<Route> to={Route::Leaderboards} classes="nav-link">
                                    <span onclick={close_menu.clone()}>{ "Leaderboards" }</span>
                                </Link<Route>>
                            </>
                        }
                    } else {
                        html! {}
                    }}

                    { if role == "Reviewer" || role == "Admin" {
                        html! {
                            <Link<Route> to={Route::ReviewerRounds} classes="nav-link">
                                <span onclick={close_menu.clone()}>{ "Reviews" }</span>
                            </Link<Route>>
                        }
                    } else {
                        html! {}
                    }}

                    { if role == "Admin" {
                        html! {
                            <Link<Route> to={Route::Admin} classes="nav-link">
                                <span onclick={close_menu.clone()}>{ "Admin" }</span>
                            </Link<Route>>
                        }
                    } else {
                        html! {}
                    }}

                    <div class="nav-spacer"></div>

                    { if authenticated {
                        html! {
                            <div class="nav-user">
                                <span class="nav-username">
                                    { user.map(|u| u.username).unwrap_or_default() }
                                </span>
                                <span class="nav-role badge badge-secondary">{ &role }</span>
                                <button class="btn btn-secondary btn-sm" onclick={on_logout}>
                                    { "Logout" }
                                </button>
                            </div>
                        }
                    } else {
                        html! {
                            <div class="nav-auth">
                                <Link<Route> to={Route::Login} classes="btn btn-secondary btn-sm">
                                    <span onclick={close_menu.clone()}>{ "Login" }</span>
                                </Link<Route>>
                                <Link<Route> to={Route::Register} classes="btn btn-primary btn-sm">
                                    <span onclick={close_menu.clone()}>{ "Register" }</span>
                                </Link<Route>>
                            </div>
                        }
                    }}
                </div>
            </div>
        </nav>
    }
}
