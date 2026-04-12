use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;

#[function_component(AdminDashboard)]
pub fn admin_dashboard() -> Html {
    html! {
        <div class="page-container">
            <h1 class="page-title">{ "Admin Dashboard" }</h1>

            <h2>{ "Management" }</h2>
            <div class="admin-nav-grid">
                <Link<Route> to={Route::AdminUsers} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F465}" }</div>
                    <h3>{ "Users" }</h3>
                    <p>{ "Manage user accounts and roles" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminTaxonomy} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F3F7}" }</div>
                    <h3>{ "Taxonomy" }</h3>
                    <p>{ "Topics and tags management" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminFields} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F4CB}" }</div>
                    <h3>{ "Custom Fields" }</h3>
                    <p>{ "Define and publish custom fields" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminModeration} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F6E1}" }</div>
                    <h3>{ "Moderation" }</h3>
                    <p>{ "Review and approve content" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminAudit} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F4DC}" }</div>
                    <h3>{ "Audit Log" }</h3>
                    <p>{ "View system activity log" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminReports} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F4CA}" }</div>
                    <h3>{ "Reports" }</h3>
                    <p>{ "Generate and view reports" }</p>
                </Link<Route>>
                <Link<Route> to={Route::AdminBackup} classes="admin-nav-card">
                    <div class="admin-nav-icon">{ "\u{1F4BE}" }</div>
                    <h3>{ "Backup" }</h3>
                    <p>{ "Database backup and restore" }</p>
                </Link<Route>>
            </div>
        </div>
    }
}
