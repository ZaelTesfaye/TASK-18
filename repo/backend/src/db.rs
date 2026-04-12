use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;

/// Creates a PostgreSQL connection pool and runs all pending migrations
/// from the `migrations/` directory.
pub async fn init_db(config: &Config) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to the database");

    tracing::info!("Connected to database, running migrations...");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations completed successfully");

    pool
}
