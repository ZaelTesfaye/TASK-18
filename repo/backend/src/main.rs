use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use chrono::Timelike;
use tracing_actix_web::TracingLogger;

mod config;
mod db;
mod errors;
mod logging;
mod middleware;
mod models;
mod routes;
mod services;

use config::Config;
use middleware::rate_limit::RateLimiter;

// Re-use the library-exported health handler so tests exercise the same code path.
use silverscreen_backend::health;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize configuration (reads all env vars once).
    let config = Config::get();

    // Initialize structured logging.
    logging::init_logging();

    // Validate secrets — reject placeholder values in production
    config.validate_secrets();

    tracing::info!(
        host = %config.server_host,
        port = %config.server_port,
        "Starting SilverScreen backend"
    );

    // Initialize database pool and run migrations.
    let pool = db::init_db(config).await;

    // Startup reconciliation: cancel expired unpaid orders from before downtime.
    match services::order_service::reconcile_expired_orders(&pool).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!(count, "Startup reconciliation: cancelled expired orders");
            } else {
                tracing::info!("Startup reconciliation: no expired orders found");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Startup reconciliation failed");
        }
    }

    // In-memory rate limiter shared across all workers.
    let rate_limiter = web::Data::new(RateLimiter::new());

    let bind_addr = format!("{}:{}", config.server_host, config.server_port);

    // Spawn background task for auto-cancel timer (check every 60 seconds).
    let bg_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match services::order_service::reconcile_expired_orders(&bg_pool).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(count, "Background: cancelled expired orders");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Background order reconciliation failed");
                }
            }
        }
    });

    // Spawn background task for nightly encrypted backups (check every hour).
    let backup_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        let mut last_backup_date = String::new();
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();
            let today = now.format("%Y-%m-%d").to_string();
            // Run backup once per day at the first hourly check after midnight UTC
            if now.hour() < 1 && last_backup_date != today {
                let config = Config::get();
                match services::backup_service::create_backup(&backup_pool, config).await {
                    Ok(backup) => {
                        tracing::info!(
                            filename = %backup.filename,
                            "Nightly backup completed successfully"
                        );
                        let _ = services::audit_service::log_action(
                            &backup_pool,
                            "SYSTEM",
                            "backup.nightly",
                            None,
                            "backup",
                            &backup.id.to_string(),
                            Some(serde_json::json!({
                                "filename": backup.filename,
                                "size_bytes": backup.size_bytes,
                                "scheduled": true
                            })),
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Nightly backup failed");
                    }
                }
                last_backup_date = today;
            }
        }
    });

    tracing::info!(bind = %bind_addr, "Server listening");

    HttpServer::new(move || {
        // Environment-aware CORS: restrict origins, methods, and headers
        let cors = {
            let config = Config::get();
            let mut cors_builder = Cors::default()
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec![
                    actix_web::http::header::AUTHORIZATION,
                    actix_web::http::header::CONTENT_TYPE,
                    actix_web::http::header::ACCEPT,
                ])
                .max_age(3600);
            if config.enable_tls {
                // Production: restrict to known frontend origin
                cors_builder = cors_builder.allowed_origin("https://localhost:8081");
            } else {
                // Development: allow local frontend origins
                cors_builder = cors_builder
                    .allowed_origin("http://localhost:8081")
                    .allowed_origin("http://127.0.0.1:8081");
            }
            cors_builder
        };

        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(rate_limiter.clone())
            .route("/health", web::get().to(health))
            .configure(routes::configure_routes)
    })
    .bind(&bind_addr)?
    .run()
    .await
}
