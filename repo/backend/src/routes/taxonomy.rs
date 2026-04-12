use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::taxonomy::{
    CreateTagRequest, CreateTopicRequest, Tag, TagResponse, Topic, TopicResponse,
    UpdateTopicRequest,
};
use crate::services::audit_service;
use crate::services::taxonomy_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/taxonomy")
            .route("/topics", web::get().to(list_topics))
            .route("/topics", web::post().to(create_topic))
            .route("/topics/{id}", web::put().to(update_topic))
            .route("/topics/{id}", web::delete().to(delete_topic))
            .route("/tags", web::get().to(list_tags))
            .route("/tags", web::post().to(create_tag))
            .route("/tags/{id}", web::delete().to(delete_tag)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct DeleteTopicQuery {
    replacement_id: Uuid,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_topic_tree(topics: Vec<Topic>) -> Vec<TopicResponse> {
    let mut responses: Vec<TopicResponse> = topics
        .iter()
        .map(|t| TopicResponse {
            id: t.id,
            name: t.name.clone(),
            slug: t.slug.clone(),
            parent_id: t.parent_id,
            depth: t.depth,
            children: vec![],
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    // Build tree by assigning children to parents (bottom-up)
    // Collect IDs and their children first
    let ids: Vec<Uuid> = responses.iter().map(|r| r.id).collect();

    // Simple approach: iterate from deepest to shallowest
    // Sort by depth descending so children come before parents
    responses.sort_by(|a, b| b.depth.cmp(&a.depth));

    let mut result: Vec<TopicResponse> = Vec::new();
    let mut children_map: std::collections::HashMap<Uuid, Vec<TopicResponse>> =
        std::collections::HashMap::new();

    for topic in responses {
        // Attach any collected children
        let mut topic = topic;
        if let Some(children) = children_map.remove(&topic.id) {
            topic.children = children;
        }

        if let Some(parent_id) = topic.parent_id {
            children_map
                .entry(parent_id)
                .or_insert_with(Vec::new)
                .push(topic);
        } else {
            result.push(topic);
        }
    }

    // Sort roots by name
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

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
// GET /api/taxonomy/topics
// ---------------------------------------------------------------------------

async fn list_topics(
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let topics = sqlx::query_as::<_, Topic>(
        "SELECT * FROM topics ORDER BY depth ASC, name ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch topics: {}", e)))?;

    let tree = build_topic_tree(topics);
    Ok(HttpResponse::Ok().json(tree))
}

// ---------------------------------------------------------------------------
// POST /api/taxonomy/topics
// ---------------------------------------------------------------------------

async fn create_topic(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateTopicRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.name.trim().is_empty() {
        return Err(AppError::ValidationError("Topic name is required".to_string()));
    }

    let slug = slugify(&req.name);
    let topic_id = Uuid::new_v4();

    let mut depth = 0i32;

    if let Some(parent_id) = req.parent_id {
        // Check that parent exists
        let parent_depth = sqlx::query_scalar::<_, i32>(
            "SELECT depth FROM topics WHERE id = $1",
        )
        .bind(parent_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch parent: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Parent topic not found".to_string()))?;

        depth = parent_depth + 1;

        if depth > 5 {
            return Err(AppError::BadRequest(
                "Maximum topic hierarchy depth of 5 would be exceeded".to_string(),
            ));
        }

        // Check acyclic (not needed for new topics, but validates parent chain depth)
        taxonomy_service::check_acyclic(pool.get_ref(), topic_id, parent_id).await?;
    }

    let topic = sqlx::query_as::<_, Topic>(
        "INSERT INTO topics (id, name, slug, parent_id, depth, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) RETURNING *",
    )
    .bind(topic_id)
    .bind(&req.name)
    .bind(&slug)
    .bind(req.parent_id)
    .bind(depth)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "taxonomy.topic_create",
        Some(&ip),
        "topic",
        &topic_id.to_string(),
        Some(serde_json::json!({"name": req.name, "parent_id": req.parent_id})),
    )
    .await?;

    let response = TopicResponse {
        id: topic.id,
        name: topic.name,
        slug: topic.slug,
        parent_id: topic.parent_id,
        depth: topic.depth,
        children: vec![],
        created_at: topic.created_at,
        updated_at: topic.updated_at,
    };

    Ok(HttpResponse::Created().json(response))
}

// ---------------------------------------------------------------------------
// PUT /api/taxonomy/topics/{id}
// ---------------------------------------------------------------------------

async fn update_topic(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateTopicRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let topic_id = path.into_inner();
    let req = body.into_inner();

    let existing = sqlx::query_as::<_, Topic>(
        "SELECT * FROM topics WHERE id = $1",
    )
    .bind(topic_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Topic not found".to_string()))?;

    let name = req.name.unwrap_or(existing.name);
    let slug = slugify(&name);

    let mut depth = existing.depth;
    let parent_id = req.parent_id.or(existing.parent_id);

    // If parent changed, validate constraints
    if req.parent_id.is_some() && req.parent_id != existing.parent_id {
        if let Some(pid) = req.parent_id {
            taxonomy_service::check_acyclic(pool.get_ref(), topic_id, pid).await?;
            depth = taxonomy_service::calculate_depth(pool.get_ref(), pid).await? + 1;
            if depth > 5 {
                return Err(AppError::BadRequest(
                    "Maximum topic hierarchy depth of 5 would be exceeded".to_string(),
                ));
            }
        } else {
            depth = 0;
        }
    }

    let topic = sqlx::query_as::<_, Topic>(
        "UPDATE topics SET name = $1, slug = $2, parent_id = $3, depth = $4, \
         updated_at = NOW() WHERE id = $5 RETURNING *",
    )
    .bind(&name)
    .bind(&slug)
    .bind(parent_id)
    .bind(depth)
    .bind(topic_id)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "taxonomy.topic_update",
        Some(&ip),
        "topic",
        &topic_id.to_string(),
        Some(serde_json::json!({"name": name, "parent_id": parent_id})),
    )
    .await?;

    let response = TopicResponse {
        id: topic.id,
        name: topic.name,
        slug: topic.slug,
        parent_id: topic.parent_id,
        depth: topic.depth,
        children: vec![],
        created_at: topic.created_at,
        updated_at: topic.updated_at,
    };

    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// DELETE /api/taxonomy/topics/{id}
// ---------------------------------------------------------------------------

async fn delete_topic(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    query: web::Query<DeleteTopicQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let topic_id = path.into_inner();
    let replacement_id = query.into_inner().replacement_id;

    taxonomy_service::safe_delete_topic(pool.get_ref(), topic_id, replacement_id).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "taxonomy.topic_delete",
        Some(&ip),
        "topic",
        &topic_id.to_string(),
        Some(serde_json::json!({"replacement_id": replacement_id})),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Topic deleted and products reassigned",
        "replacement_id": replacement_id
    })))
}

// ---------------------------------------------------------------------------
// GET /api/taxonomy/tags
// ---------------------------------------------------------------------------

async fn list_tags(
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let tags = sqlx::query_as::<_, TagResponse>(
        "SELECT id, name, slug, created_at FROM tags ORDER BY name ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch tags: {}", e)))?;

    Ok(HttpResponse::Ok().json(tags))
}

// ---------------------------------------------------------------------------
// POST /api/taxonomy/tags
// ---------------------------------------------------------------------------

async fn create_tag(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateTagRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.name.trim().is_empty() {
        return Err(AppError::ValidationError("Tag name is required".to_string()));
    }

    let slug = slugify(&req.name);
    let tag_id = Uuid::new_v4();

    let tag = sqlx::query_as::<_, TagResponse>(
        "INSERT INTO tags (id, name, slug, created_at) \
         VALUES ($1, $2, $3, NOW()) RETURNING id, name, slug, created_at",
    )
    .bind(tag_id)
    .bind(&req.name)
    .bind(&slug)
    .fetch_one(pool.get_ref())
    .await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "taxonomy.tag_create",
        Some(&ip),
        "tag",
        &tag_id.to_string(),
        Some(serde_json::json!({"name": req.name})),
    )
    .await?;

    Ok(HttpResponse::Created().json(tag))
}

// ---------------------------------------------------------------------------
// DELETE /api/taxonomy/tags/{id}
// ---------------------------------------------------------------------------

async fn delete_tag(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let tag_id = path.into_inner();

    let mut tx = pool
        .get_ref()
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Remove product associations
    sqlx::query("DELETE FROM product_tags WHERE tag_id = $1")
        .bind(tag_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to remove tag associations: {}", e)))?;

    let rows = sqlx::query("DELETE FROM tags WHERE id = $1")
        .bind(tag_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete tag: {}", e)))?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Tag not found".to_string()));
    }

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "taxonomy.tag_delete",
        Some(&ip),
        "tag",
        &tag_id.to_string(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Tag deleted"})))
}
