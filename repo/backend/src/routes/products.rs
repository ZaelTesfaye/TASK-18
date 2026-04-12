use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_role;
use crate::models::common::PaginatedResponse;
use crate::models::custom_field::CustomFieldValue;
use crate::models::product::{
    CreateProductRequest, Product, ProductFilter, ProductResponse, UpdateProductRequest,
};
use crate::models::taxonomy::{TagResponse, TopicResponse};
use crate::services::audit_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/products")
            .route("", web::get().to(list_products))
            .route("/{id}", web::get().to(get_product))
            .route("", web::post().to(create_product))
            .route("/{id}", web::put().to(update_product))
            .route("/{id}", web::delete().to(delete_product)),
    );
}

// ---------------------------------------------------------------------------
// GET /api/products
// ---------------------------------------------------------------------------

async fn list_products(
    pool: web::Data<PgPool>,
    query: web::Query<ProductFilter>,
) -> Result<HttpResponse, AppError> {
    let filter = query.into_inner();
    let page = filter.page();
    let per_page = filter.per_page();
    let offset = filter.offset();

    // Build dynamic WHERE clauses
    let mut conditions: Vec<String> = vec!["p.is_active = TRUE".to_string()];
    let mut param_idx = 1u32;

    // We'll use a Vec to track bind order
    if filter.topic_id.is_some() {
        conditions.push(format!(
            "p.id IN (SELECT product_id FROM product_topics WHERE topic_id = ${param_idx})"
        ));
        param_idx += 1;
    }
    if filter.tag_id.is_some() {
        conditions.push(format!(
            "p.id IN (SELECT product_id FROM product_tags WHERE tag_id = ${param_idx})"
        ));
        param_idx += 1;
    }
    if filter.genre.is_some() {
        conditions.push(format!("p.genre = ${param_idx}"));
        param_idx += 1;
    }
    if filter.min_price.is_some() {
        conditions.push(format!("p.price >= ${param_idx}"));
        param_idx += 1;
    }
    if filter.max_price.is_some() {
        conditions.push(format!("p.price <= ${param_idx}"));
        param_idx += 1;
    }
    if filter.search.is_some() {
        conditions.push(format!(
            "(p.title ILIKE ${param_idx} OR p.description ILIKE ${param_idx})"
        ));
        param_idx += 1;
    }
    if filter.custom_field_name.is_some() && filter.custom_field_value.is_some() {
        // Use to_jsonb() for reliable comparison — avoids text-cast issues
        // where JSON strings are double-quoted (e.g. '"hello"' vs 'hello').
        conditions.push(format!(
            "p.id IN (SELECT cfv.product_id FROM custom_field_values cfv \
             JOIN custom_field_definitions cfd ON cfd.id = cfv.field_id \
             WHERE cfd.slug = ${param_idx} AND cfv.value = to_jsonb(${next_param_idx}::text))",
            param_idx = param_idx,
            next_param_idx = param_idx + 1
        ));
        param_idx += 2;
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!(
        "SELECT COUNT(*) FROM products p WHERE {where_clause}"
    );
    let data_sql = format!(
        "SELECT p.id, p.title, p.description, p.price::float8 AS price, p.stock, \
         p.image_url, p.genre, p.release_year, p.is_active, \
         p.created_at, p.updated_at \
         FROM products p WHERE {where_clause} \
         ORDER BY p.created_at DESC \
         LIMIT ${param_idx} OFFSET ${next_param_idx}",
        where_clause = where_clause,
        param_idx = param_idx,
        next_param_idx = param_idx + 1
    );

    // Bind parameters for count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    let mut data_query = sqlx::query_as::<_, Product>(&data_sql);

    // Bind filter params in order
    if let Some(ref topic_id) = filter.topic_id {
        count_query = count_query.bind(topic_id);
        data_query = data_query.bind(topic_id);
    }
    if let Some(ref tag_id) = filter.tag_id {
        count_query = count_query.bind(tag_id);
        data_query = data_query.bind(tag_id);
    }
    if let Some(ref genre) = filter.genre {
        count_query = count_query.bind(genre);
        data_query = data_query.bind(genre);
    }
    if let Some(ref min_price) = filter.min_price {
        count_query = count_query.bind(min_price);
        data_query = data_query.bind(min_price);
    }
    if let Some(ref max_price) = filter.max_price {
        count_query = count_query.bind(max_price);
        data_query = data_query.bind(max_price);
    }
    if let Some(ref search) = filter.search {
        let pattern = format!("%{}%", search);
        count_query = count_query.bind(pattern.clone());
        data_query = data_query.bind(pattern);
    }
    if let Some(ref cf_name) = filter.custom_field_name {
        if let Some(ref cf_value) = filter.custom_field_value {
            count_query = count_query.bind(cf_name).bind(cf_value);
            data_query = data_query.bind(cf_name).bind(cf_value);
        }
    }

    // Bind limit and offset for data query only
    data_query = data_query.bind(per_page).bind(offset);

    let total = count_query
        .fetch_one(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to count products: {}", e)))?;

    let products = data_query
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch products: {}", e)))?;

    // Build lightweight product responses (without enrichments for list view)
    let items: Vec<ProductResponse> = products
        .iter()
        .map(|p| ProductResponse {
            id: p.id,
            title: p.title.clone(),
            description: p.description.clone(),
            price: p.price,
            stock: p.stock,
            image_url: p.image_url.clone(),
            genre: p.genre.clone(),
            release_year: p.release_year,
            is_active: p.is_active,
            topics: vec![],
            tags: vec![],
            custom_fields: vec![],
            average_score: None,
            total_ratings: 0,
            created_at: p.created_at,
            updated_at: p.updated_at,
        })
        .collect();

    let response = PaginatedResponse::new(items, total, page, per_page);
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/products/{id}
// ---------------------------------------------------------------------------

async fn get_product(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let product_id = path.into_inner();

    let product = sqlx::query_as::<_, Product>(
        "SELECT id, title, description, price::float8 AS price, stock, \
         image_url, genre, release_year, is_active, created_at, updated_at \
         FROM products WHERE id = $1",
    )
    .bind(product_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Fetch topics
    let topics = sqlx::query_as::<_, TopicRow>(
        "SELECT t.id, t.name, t.slug, t.parent_id, t.depth, t.created_at, t.updated_at \
         FROM topics t \
         JOIN product_topics pt ON pt.topic_id = t.id \
         WHERE pt.product_id = $1",
    )
    .bind(product_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch topics: {}", e)))?;

    let topic_responses: Vec<TopicResponse> = topics
        .into_iter()
        .map(|t| TopicResponse {
            id: t.id,
            name: t.name,
            slug: t.slug,
            parent_id: t.parent_id,
            depth: t.depth,
            children: vec![],
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    // Fetch tags
    let tags = sqlx::query_as::<_, TagResponse>(
        "SELECT t.id, t.name, t.slug, t.created_at \
         FROM tags t \
         JOIN product_tags pt ON pt.tag_id = t.id \
         WHERE pt.product_id = $1",
    )
    .bind(product_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch tags: {}", e)))?;

    // Fetch custom fields
    let custom_fields = sqlx::query_as::<_, CustomFieldValue>(
        "SELECT * FROM custom_field_values WHERE product_id = $1",
    )
    .bind(product_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch custom fields: {}", e)))?;

    // Fetch aggregate score
    let score = sqlx::query_as::<_, ScoreRow>(
        "SELECT average_score::float8 AS average_score, total_ratings \
         FROM product_scores WHERE product_id = $1",
    )
    .bind(product_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch score: {}", e)))?;

    let response = ProductResponse {
        id: product.id,
        title: product.title,
        description: product.description,
        price: product.price,
        stock: product.stock,
        image_url: product.image_url,
        genre: product.genre,
        release_year: product.release_year,
        is_active: product.is_active,
        topics: topic_responses,
        tags,
        custom_fields,
        average_score: score.as_ref().and_then(|s| s.average_score),
        total_ratings: score.map(|s| s.total_ratings).unwrap_or(0),
        created_at: product.created_at,
        updated_at: product.updated_at,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, sqlx::FromRow)]
struct TopicRow {
    id: Uuid,
    name: String,
    slug: String,
    parent_id: Option<Uuid>,
    depth: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ScoreRow {
    average_score: Option<f64>,
    total_ratings: i32,
}

// ---------------------------------------------------------------------------
// POST /api/products
// ---------------------------------------------------------------------------

async fn create_product(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<CreateProductRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.title.trim().is_empty() {
        return Err(AppError::ValidationError("Title is required".to_string()));
    }
    if req.price < 0.0 {
        return Err(AppError::ValidationError("Price must be non-negative".to_string()));
    }

    let product_id = Uuid::new_v4();
    let stock = req.stock.unwrap_or(0);

    let product = sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, title, description, price, stock, image_url, genre, \
         release_year, is_active, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE, NOW(), NOW()) \
         RETURNING id, title, description, price::float8 AS price, stock, image_url, \
         genre, release_year, is_active, created_at, updated_at",
    )
    .bind(product_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.price)
    .bind(stock)
    .bind(&req.image_url)
    .bind(&req.genre)
    .bind(req.release_year)
    .fetch_one(pool.get_ref())
    .await?;

    // Assign topics
    if let Some(ref topic_ids) = req.topic_ids {
        for tid in topic_ids {
            sqlx::query(
                "INSERT INTO product_topics (product_id, topic_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(product_id)
            .bind(tid)
            .execute(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to assign topic: {}", e)))?;
        }
    }

    // Assign tags
    if let Some(ref tag_ids) = req.tag_ids {
        for tid in tag_ids {
            sqlx::query(
                "INSERT INTO product_tags (product_id, tag_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(product_id)
            .bind(tid)
            .execute(pool.get_ref())
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to assign tag: {}", e)))?;
        }
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "product.create",
        Some(&ip),
        "product",
        &product_id.to_string(),
        Some(serde_json::json!({"title": req.title, "price": req.price})),
    )
    .await?;

    let response = ProductResponse {
        id: product.id,
        title: product.title,
        description: product.description,
        price: product.price,
        stock: product.stock,
        image_url: product.image_url,
        genre: product.genre,
        release_year: product.release_year,
        is_active: product.is_active,
        topics: vec![],
        tags: vec![],
        custom_fields: vec![],
        average_score: None,
        total_ratings: 0,
        created_at: product.created_at,
        updated_at: product.updated_at,
    };

    Ok(HttpResponse::Created().json(response))
}

// ---------------------------------------------------------------------------
// PUT /api/products/{id}
// ---------------------------------------------------------------------------

async fn update_product(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<UpdateProductRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let product_id = path.into_inner();
    let req = body.into_inner();

    // Ensure product exists
    let existing = sqlx::query_as::<_, Product>(
        "SELECT id, title, description, price::float8 AS price, stock, image_url, \
         genre, release_year, is_active, created_at, updated_at \
         FROM products WHERE id = $1",
    )
    .bind(product_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let title = req.title.unwrap_or(existing.title);
    let description = req.description.or(existing.description);
    let price = req.price.unwrap_or(existing.price);
    let stock = req.stock.unwrap_or(existing.stock);
    let image_url = req.image_url.or(existing.image_url);
    let genre = req.genre.or(existing.genre);
    let release_year = req.release_year.or(existing.release_year);
    let is_active = req.is_active.unwrap_or(existing.is_active);

    let product = sqlx::query_as::<_, Product>(
        "UPDATE products SET title = $1, description = $2, price = $3, stock = $4, \
         image_url = $5, genre = $6, release_year = $7, is_active = $8, updated_at = NOW() \
         WHERE id = $9 \
         RETURNING id, title, description, price::float8 AS price, stock, image_url, \
         genre, release_year, is_active, created_at, updated_at",
    )
    .bind(&title)
    .bind(&description)
    .bind(price)
    .bind(stock)
    .bind(&image_url)
    .bind(&genre)
    .bind(release_year)
    .bind(is_active)
    .bind(product_id)
    .fetch_one(pool.get_ref())
    .await?;

    // Update topics if provided
    if let Some(ref topic_ids) = req.topic_ids {
        sqlx::query("DELETE FROM product_topics WHERE product_id = $1")
            .bind(product_id)
            .execute(pool.get_ref())
            .await?;
        for tid in topic_ids {
            sqlx::query(
                "INSERT INTO product_topics (product_id, topic_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(product_id)
            .bind(tid)
            .execute(pool.get_ref())
            .await?;
        }
    }

    // Update tags if provided
    if let Some(ref tag_ids) = req.tag_ids {
        sqlx::query("DELETE FROM product_tags WHERE product_id = $1")
            .bind(product_id)
            .execute(pool.get_ref())
            .await?;
        for tid in tag_ids {
            sqlx::query(
                "INSERT INTO product_tags (product_id, tag_id) VALUES ($1, $2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(product_id)
            .bind(tid)
            .execute(pool.get_ref())
            .await?;
        }
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "product.update",
        Some(&ip),
        "product",
        &product_id.to_string(),
        Some(serde_json::json!({"title": title, "price": price})),
    )
    .await?;

    let response = ProductResponse {
        id: product.id,
        title: product.title,
        description: product.description,
        price: product.price,
        stock: product.stock,
        image_url: product.image_url,
        genre: product.genre,
        release_year: product.release_year,
        is_active: product.is_active,
        topics: vec![],
        tags: vec![],
        custom_fields: vec![],
        average_score: None,
        total_ratings: 0,
        created_at: product.created_at,
        updated_at: product.updated_at,
    };

    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// DELETE /api/products/{id}
// ---------------------------------------------------------------------------

async fn delete_product(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let product_id = path.into_inner();

    let rows = sqlx::query(
        "UPDATE products SET is_active = FALSE, updated_at = NOW() WHERE id = $1",
    )
    .bind(product_id)
    .execute(pool.get_ref())
    .await?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "product.soft_delete",
        Some(&ip),
        "product",
        &product_id.to_string(),
        None,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Product deactivated"})))
}
