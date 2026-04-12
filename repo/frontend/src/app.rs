use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::api;
use crate::components::navbar::Navbar;
use crate::components::toast::{use_toast, ToastContainer};
use crate::pages::admin::{
    audit_log::AdminAuditLogPage, backup::AdminBackupPage, dashboard::AdminDashboard,
    fields::AdminFieldsPage, moderation::AdminModerationPage, reports::AdminReportsPage,
    taxonomy::AdminTaxonomyPage, users::AdminUsersPage,
};
use crate::pages::cart::CartPage;
use crate::pages::checkout::CheckoutPage;
use crate::pages::home::HomePage;
use crate::pages::leaderboards::LeaderboardsPage;
use crate::pages::login::LoginPage;
use crate::pages::order_detail::OrderDetailPage;
use crate::pages::orders::OrdersPage;
use crate::pages::product_detail::ProductDetailPage;
use crate::pages::rate_product::RateProductPage;
use crate::pages::register::RegisterPage;
use crate::pages::reviewer::rounds::ReviewerRoundsPage;
use crate::pages::reviewer::submit::ReviewerSubmitPage;
use crate::store;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/register")]
    Register,
    #[at("/catalog")]
    Catalog,
    #[at("/products/:id")]
    ProductDetail { id: String },
    #[at("/cart")]
    Cart,
    #[at("/checkout")]
    Checkout,
    #[at("/orders")]
    Orders,
    #[at("/orders/:id")]
    OrderDetail { id: String },
    #[at("/ratings/:product_id")]
    RateProduct { product_id: String },
    #[at("/leaderboards")]
    Leaderboards,
    #[at("/reviewer/rounds")]
    ReviewerRounds,
    #[at("/reviewer/rounds/:id/submit")]
    ReviewerSubmit { id: String },
    #[at("/admin")]
    Admin,
    #[at("/admin/users")]
    AdminUsers,
    #[at("/admin/taxonomy")]
    AdminTaxonomy,
    #[at("/admin/fields")]
    AdminFields,
    #[at("/admin/moderation")]
    AdminModeration,
    #[at("/admin/audit")]
    AdminAudit,
    #[at("/admin/reports")]
    AdminReports,
    #[at("/admin/backup")]
    AdminBackup,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(route: Route) -> Html {
    match route {
        Route::Home | Route::Catalog => html! { <HomePage /> },
        Route::Login => html! { <LoginPage /> },
        Route::Register => html! { <RegisterPage /> },
        Route::ProductDetail { id } => html! { <ProductDetailPage id={id} /> },
        Route::Cart => html! { <CartPage /> },
        Route::Checkout => html! { <CheckoutPage /> },
        Route::Orders => html! { <OrdersPage /> },
        Route::OrderDetail { id } => html! { <OrderDetailPage id={id} /> },
        Route::RateProduct { product_id } => html! { <RateProductPage product_id={product_id} /> },
        Route::Leaderboards => html! { <LeaderboardsPage /> },
        Route::ReviewerRounds => html! { <ReviewerRoundsPage /> },
        Route::ReviewerSubmit { id } => html! { <ReviewerSubmitPage id={id} /> },
        Route::Admin => html! { <AdminDashboard /> },
        Route::AdminUsers => html! { <AdminUsersPage /> },
        Route::AdminTaxonomy => html! { <AdminTaxonomyPage /> },
        Route::AdminFields => html! { <AdminFieldsPage /> },
        Route::AdminModeration => html! { <AdminModerationPage /> },
        Route::AdminAudit => html! { <AdminAuditLogPage /> },
        Route::AdminReports => html! { <AdminReportsPage /> },
        Route::AdminBackup => html! { <AdminBackupPage /> },
        Route::NotFound => html! {
            <div class="page-center">
                <div class="empty-state">
                    <h1>{ "404" }</h1>
                    <p>{ "Page not found" }</p>
                    <Link<Route> to={Route::Home} classes="btn btn-primary">{ "Go Home" }</Link<Route>>
                </div>
            </div>
        },
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let cart_count = use_state(|| 0u32);
    let toast = use_toast();
    // Force re-render trigger on auth state change
    let auth_version = use_state(|| 0u32);

    // Fetch cart count periodically
    {
        let cart_count = cart_count.clone();
        let auth_ver = *auth_version;
        use_effect_with(auth_ver, move |_| {
            let cart_count = cart_count.clone();
            if store::is_authenticated() {
                spawn_local(async move {
                    if let Ok(c) = api::cart::get_cart().await {
                        cart_count.set(c.items.iter().map(|i| i.quantity).sum());
                    }
                });
            } else {
                cart_count.set(0);
            }
            || ()
        });
    }

    let on_logout = {
        let auth_version = auth_version.clone();
        let toast = toast.clone();
        Callback::from(move |_: ()| {
            let auth_version = auth_version.clone();
            let toast = toast.clone();
            spawn_local(async move {
                let _ = api::auth::logout().await;
                store::clear_tokens();
                auth_version.set(*auth_version + 1);
                toast.info("Logged out successfully.");
            });
        })
    };

    html! {
        <BrowserRouter>
            <div class="app-layout">
                <Navbar cart_count={*cart_count} on_logout={on_logout} />
                <main class="main-content">
                    <Switch<Route> render={switch} />
                </main>
                <ToastContainer manager={toast} />
            </div>
        </BrowserRouter>
    }
}
