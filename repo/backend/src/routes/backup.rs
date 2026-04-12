use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::backup::Backup;
use crate::services::audit_service;
use crate::services::backup_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/backup")
            .route("", web::post().to(create_backup))
            .route("", web::get().to(list_backups))
            .route("/{id}/verify", web::post().to(verify_backup))
            .route("/{id}/restore", web::post().to(restore_backup)),
    );
}

// ---------------------------------------------------------------------------
// POST /api/backup
// ---------------------------------------------------------------------------

async fn create_backup(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let config = Config::get();
    let backup = backup_service::create_backup(pool.get_ref(), config).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "backup.create",
        Some(&ip),
        "backup",
        &backup.id.to_string(),
        Some(serde_json::json!({
            "filename": backup.filename,
            "size_bytes": backup.size_bytes
        })),
    )
    .await?;

    Ok(HttpResponse::Created().json(backup))
}

// ---------------------------------------------------------------------------
// GET /api/backup
// ---------------------------------------------------------------------------

async fn list_backups(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let backups = sqlx::query_as::<_, Backup>(
        "SELECT * FROM backups ORDER BY created_at DESC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch backups: {}", e)))?;

    Ok(HttpResponse::Ok().json(backups))
}

// ---------------------------------------------------------------------------
// POST /api/backup/{id}/verify
// ---------------------------------------------------------------------------

async fn verify_backup(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let backup_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let config = Config::get();

    let valid = backup_service::verify_backup(pool.get_ref(), config, backup_id).await?;

    // Use valid backup_status enum values: InProgress, Completed, Failed
    let db_status = if valid { "Completed" } else { "Failed" };
    sqlx::query(
        "UPDATE backups SET status = $1::backup_status WHERE id = $2",
    )
    .bind(db_status)
    .bind(backup_id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to update backup status: {}", e)))?;

    // Audit log the verification
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "backup.verify",
        Some(&ip),
        "backup",
        &backup_id.to_string(),
        Some(serde_json::json!({
            "valid": valid,
            "status": db_status
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "backup_id": backup_id,
        "valid": valid,
        "verified": true,
        "status": db_status
    })))
}

// ---------------------------------------------------------------------------
// POST /api/backup/{id}/restore
// ---------------------------------------------------------------------------

async fn restore_backup(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let backup_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let config = Config::get();

    let result = backup_service::restore_backup(pool.get_ref(), config, backup_id).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "backup.restore",
        Some(&ip),
        "backup",
        &backup_id.to_string(),
        Some(serde_json::json!({
            "users_restored": result.users_restored,
            "products_restored": result.products_restored,
            "orders_restored": result.orders_restored,
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Backup data restored successfully.",
        "backup_id": backup_id,
        "users_restored": result.users_restored,
        "products_restored": result.products_restored,
        "orders_restored": result.orders_restored,
        "tables_restored": result.tables_restored,
        "note": "Restored users retain their original password hashes and can log in immediately. Manual verification recommended."
    })))
}
