pub mod auth;
pub mod users;
pub mod products;
pub mod cart;
pub mod orders;
pub mod ratings;
pub mod reviews;
pub mod leaderboards;
pub mod taxonomy;
pub mod custom_fields;
pub mod admin;
pub mod audit;
pub mod reports;
pub mod backup;
pub mod payment;

use actix_web::web;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(auth::configure)
            .configure(users::configure)
            .configure(products::configure)
            .configure(cart::configure)
            .configure(orders::configure)
            .configure(ratings::configure)
            .configure(reviews::configure)
            .configure(leaderboards::configure)
            .configure(taxonomy::configure)
            .configure(custom_fields::configure)
            .configure(admin::configure)
            .configure(audit::configure)
            .configure(reports::configure)
            .configure(backup::configure)
            .configure(payment::configure),
    );
}
