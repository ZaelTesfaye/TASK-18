use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::audit::AuditQuery;
use crate::services::audit_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/audit")
            .route("", web::get().to(query_audit_log)),
    );
}

// ---------------------------------------------------------------------------
// GET /api/audit
// ---------------------------------------------------------------------------

async fn query_audit_log(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    query: web::Query<AuditQuery>,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let q = query.into_inner();
    let result = audit_service::query_audit_log(pool.get_ref(), q).await?;

    Ok(HttpResponse::Ok().json(result))
}
