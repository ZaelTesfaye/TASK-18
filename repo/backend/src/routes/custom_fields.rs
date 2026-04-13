use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::custom_field::{CustomFieldDefinition, CustomFieldValue, CreateFieldDefinitionRequest, SetFieldValueRequest};
use crate::services::audit_service;
use crate::services::field_migration_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/custom-fields")
            .route("", web::get().to(list_fields))
            .route("", web::post().to(create_field))
            .route("/{id}", web::put().to(update_field))
            .route("/{id}/publish", web::post().to(publish_field))
            .route("/{id}/conflicts", web::get().to(list_conflicts))
            .route(
                "/{id}/conflicts/{product_id}",
                web::put().to(resolve_conflict),
            ),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UpdateFieldRequest {
    name: Option<String>,
    field_type: Option<String>,
    allowed_values: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

// ---------------------------------------------------------------------------
// GET /api/custom-fields
// ---------------------------------------------------------------------------

async fn list_fields(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let fields = if user.role == "Admin" {
        // Admin sees all including drafts
        sqlx::query_as::<_, CustomFieldDefinition>(
            "SELECT * FROM custom_field_definitions ORDER BY name ASC",
        )
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch fields: {}", e)))?
    } else {
        // Non-admin sees only published
        sqlx::query_as::<_, CustomFieldDefinition>(
            "SELECT * FROM custom_field_definitions WHERE status = 'Published' ORDER BY name ASC",
        )
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch fields: {}", e)))?
    };

    Ok(HttpResponse::Ok().json(fields))
}

// ---------------------------------------------------------------------------
// POST /api/custom-fields
// ---------------------------------------------------------------------------

async fn create_field(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateFieldDefinitionRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.name.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Field name is required".to_string(),
        ));
    }

    // Validate field type
    field_migration_service::FieldType::from_str(&req.field_type)?;

    let slug = slugify(&req.name);
    let field_id = Uuid::new_v4();

    let allowed_values = req
        .allowed_values
        .map(|v| serde_json::json!(v));

    let field = sqlx::query_as::<_, CustomFieldDefinition>(
        "INSERT INTO custom_field_definitions \
         (id, name, slug, field_type, allowed_values, status, version, conflict_count, \
          created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, 'Draft', 1, 0, NOW(), NOW()) \
         RETURNING *",
    )
    .bind(field_id)
    .bind(&req.name)
    .bind(&slug)
    .bind(&req.field_type)
    .bind(&allowed_values)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "custom_field.create",
        Some(&ip),
        "custom_field_definition",
        &field_id.to_string(),
        Some(serde_json::json!({
            "name": req.name,
            "field_type": req.field_type
        })),
    )
    .await?;

    Ok(HttpResponse::Created().json(field))
}

// ---------------------------------------------------------------------------
// PUT /api/custom-fields/{id}
// ---------------------------------------------------------------------------

async fn update_field(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateFieldRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let field_id = path.into_inner();
    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    let existing = sqlx::query_as::<_, CustomFieldDefinition>(
        "SELECT * FROM custom_field_definitions WHERE id = $1",
    )
    .bind(field_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Field definition not found".to_string()))?;

    let name = req.name.unwrap_or(existing.name);
    let slug = slugify(&name);
    let field_type = req.field_type.unwrap_or(existing.field_type.clone());
    let allowed_values = req
        .allowed_values
        .clone()
        .map(|v| serde_json::json!(v))
        .or(existing.allowed_values.clone());

    // Validate field type
    let new_type = field_migration_service::FieldType::from_str(&field_type)?;

    // If type changed, run migration plan
    let type_changed = field_type != existing.field_type;
    let mut conflict_count = existing.conflict_count;

    if type_changed {
        let allowed_vec = req
            .allowed_values
            .clone()
            .or_else(|| {
                existing.allowed_values.as_ref().and_then(|v| {
                    serde_json::from_value::<Vec<String>>(v.clone()).ok()
                })
            });

        let plan = field_migration_service::plan_migration(
            pool.get_ref(),
            field_id,
            new_type,
            allowed_vec,
        )
        .await?;

        // Apply auto-converted values
        let mut tx = pool
            .get_ref()
            .begin()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

        for item in &plan.items {
            match &item.status {
                field_migration_service::MigrationItemStatus::AutoConverted => {
                    if let Some(ref converted) = item.converted_value {
                        sqlx::query(
                            "UPDATE custom_field_values SET value = $1, \
                             field_version = field_version + 1, \
                             conflict_status = 'Resolved', \
                             conflict_message = NULL, \
                             updated_at = NOW() \
                             WHERE field_id = $2 AND product_id = $3",
                        )
                        .bind(converted)
                        .bind(field_id)
                        .bind(item.product_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| {
                            AppError::InternalError(format!(
                                "Failed to apply conversion: {}",
                                e
                            ))
                        })?;
                    }
                }
                field_migration_service::MigrationItemStatus::Conflict { message } => {
                    sqlx::query(
                        "UPDATE custom_field_values SET \
                         conflict_status = 'Pending', \
                         conflict_message = $1, \
                         updated_at = NOW() \
                         WHERE field_id = $2 AND product_id = $3",
                    )
                    .bind(message)
                    .bind(field_id)
                    .bind(item.product_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        AppError::InternalError(format!("Failed to mark conflict: {}", e))
                    })?;
                }
            }
        }

        conflict_count = plan.conflicts as i32;

        tx.commit()
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;
    }

    let new_version = existing.version + 1;
    let previous_type = if type_changed {
        Some(existing.field_type.clone())
    } else {
        existing.previous_type.clone()
    };
    let previous_allowed = if type_changed {
        existing.allowed_values.clone()
    } else {
        existing.previous_allowed_values.clone()
    };

    let field = sqlx::query_as::<_, CustomFieldDefinition>(
        "UPDATE custom_field_definitions SET \
         name = $1, slug = $2, field_type = $3, allowed_values = $4, \
         version = $5, previous_type = $6, previous_allowed_values = $7, \
         conflict_count = $8, updated_at = NOW() \
         WHERE id = $9 RETURNING *",
    )
    .bind(&name)
    .bind(&slug)
    .bind(&field_type)
    .bind(&allowed_values)
    .bind(new_version)
    .bind(&previous_type)
    .bind(&previous_allowed)
    .bind(conflict_count)
    .bind(field_id)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "custom_field.update",
        Some(&ip),
        "custom_field_definition",
        &field_id.to_string(),
        Some(serde_json::json!({
            "name": name,
            "field_type": field_type,
            "type_changed": type_changed,
            "conflict_count": conflict_count
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(field))
}

// ---------------------------------------------------------------------------
// POST /api/custom-fields/{id}/publish
// ---------------------------------------------------------------------------

async fn publish_field(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let field_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();

    let can = field_migration_service::can_publish(pool.get_ref(), field_id).await?;
    if !can {
        return Err(AppError::Conflict(
            "Cannot publish field: there are unresolved conflicts".to_string(),
        ));
    }

    let field = sqlx::query_as::<_, CustomFieldDefinition>(
        "UPDATE custom_field_definitions SET status = 'Published', updated_at = NOW() \
         WHERE id = $1 RETURNING *",
    )
    .bind(field_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to publish field: {}", e)))?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "custom_field.publish",
        Some(&ip),
        "custom_field_definition",
        &field_id.to_string(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(field))
}

// ---------------------------------------------------------------------------
// GET /api/custom-fields/{id}/conflicts
// ---------------------------------------------------------------------------

async fn list_conflicts(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let field_id = path.into_inner();

    let conflicts = sqlx::query_as::<_, CustomFieldValue>(
        "SELECT * FROM custom_field_values \
         WHERE field_id = $1 AND conflict_status = 'Pending' \
         ORDER BY updated_at DESC",
    )
    .bind(field_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch conflicts: {}", e)))?;

    Ok(HttpResponse::Ok().json(conflicts))
}

// ---------------------------------------------------------------------------
// PUT /api/custom-fields/{id}/conflicts/{product_id}
// ---------------------------------------------------------------------------

async fn resolve_conflict(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<SetFieldValueRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let (field_id, product_id) = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    field_migration_service::resolve_conflict(
        pool.get_ref(),
        field_id,
        product_id,
        req.value,
    )
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "custom_field.resolve_conflict",
        Some(&ip),
        "custom_field_value",
        &format!("{}:{}", field_id, product_id),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Conflict resolved"})))
}
