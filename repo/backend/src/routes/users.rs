use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::require_owner_or_admin;
use crate::models::user::{User, UserResponse};
use crate::services::audit_service;
use crate::services::encryption_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .route("/me", web::get().to(get_me))
            .route("/me", web::put().to(update_me))
            .route("/me/unmask", web::post().to(unmask))
            .route("/{id}", web::get().to(get_user_by_id)),
    );
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UpdateProfileRequest {
    phone: Option<String>,
    address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnmaskRequest {
    justification: String,
}

#[derive(Debug, serde::Serialize)]
struct UnmaskedResponse {
    id: Uuid,
    username: String,
    email: String,
    phone: Option<String>,
    address: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_encryption_keys() -> HashMap<u32, Vec<u8>> {
    let config = Config::get();
    let key_bytes = derive_key_bytes(&config.encryption_key);
    let mut keys = HashMap::new();
    keys.insert(config.encryption_key_version, key_bytes);
    keys
}

fn derive_key_bytes(key_str: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key_str.as_bytes());
    hasher.finalize().to_vec()
}

fn build_user_response(user: &User) -> UserResponse {
    let keys = build_encryption_keys();
    let phone_masked = user
        .phone_encrypted
        .as_deref()
        .map(|p| encryption_service::mask_phone(p, &keys));
    let address_masked = user
        .address_encrypted
        .as_deref()
        .map(|a| encryption_service::mask_address(a, &keys));

    UserResponse {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        phone_masked,
        address_masked,
        verified_possession: user.verified_possession,
        is_locked: user.is_locked,
        legal_hold: user.legal_hold,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }
}

// ---------------------------------------------------------------------------
// GET /api/users/me
// ---------------------------------------------------------------------------

async fn get_me(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let db_user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let response = build_user_response(&db_user);
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// PUT /api/users/me
// ---------------------------------------------------------------------------

async fn update_me(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, AppError> {
    let config = Config::get();
    let key_bytes = derive_key_bytes(&config.encryption_key);
    let req = body.into_inner();

    // Encrypt phone if provided
    let phone_encrypted = match req.phone {
        Some(ref phone) => {
            if phone.trim().is_empty() {
                None
            } else {
                Some(encryption_service::encrypt(
                    phone,
                    &key_bytes,
                    config.encryption_key_version,
                )?)
            }
        }
        None => None,
    };

    // Encrypt address if provided
    let address_encrypted = match req.address {
        Some(ref address) => {
            if address.trim().is_empty() {
                None
            } else {
                Some(encryption_service::encrypt(
                    address,
                    &key_bytes,
                    config.encryption_key_version,
                )?)
            }
        }
        None => None,
    };

    // Build dynamic update query
    let db_user = if phone_encrypted.is_some() && address_encrypted.is_some() {
        sqlx::query_as::<_, User>(
            "UPDATE users SET phone_encrypted = $1, address_encrypted = $2, updated_at = NOW() \
             WHERE id = $3 RETURNING *",
        )
        .bind(&phone_encrypted)
        .bind(&address_encrypted)
        .bind(user.user_id)
        .fetch_one(pool.get_ref())
        .await?
    } else if phone_encrypted.is_some() {
        sqlx::query_as::<_, User>(
            "UPDATE users SET phone_encrypted = $1, updated_at = NOW() \
             WHERE id = $2 RETURNING *",
        )
        .bind(&phone_encrypted)
        .bind(user.user_id)
        .fetch_one(pool.get_ref())
        .await?
    } else if address_encrypted.is_some() {
        sqlx::query_as::<_, User>(
            "UPDATE users SET address_encrypted = $1, updated_at = NOW() \
             WHERE id = $2 RETURNING *",
        )
        .bind(&address_encrypted)
        .bind(user.user_id)
        .fetch_one(pool.get_ref())
        .await?
    } else {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1",
        )
        .bind(user.user_id)
        .fetch_one(pool.get_ref())
        .await?
    };

    let response = build_user_response(&db_user);
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/users/{id}
// ---------------------------------------------------------------------------

async fn get_user_by_id(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();

    // Admin or self only
    require_owner_or_admin(&user, target_id)?;

    let db_user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(target_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let response = build_user_response(&db_user);
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/users/me/unmask
// ---------------------------------------------------------------------------

async fn unmask(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    body: web::Json<UnmaskRequest>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse, AppError> {
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let req = body.into_inner();

    if req.justification.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Justification is required to unmask sensitive data".to_string(),
        ));
    }

    let db_user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let keys = build_encryption_keys();

    let phone = db_user
        .phone_encrypted
        .as_deref()
        .and_then(|p| encryption_service::decrypt(p, &keys).ok());

    let address = db_user
        .address_encrypted
        .as_deref()
        .and_then(|a| encryption_service::decrypt(a, &keys).ok());

    // Audit log the unmask action
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "user.unmask_pii",
        Some(&ip),
        "user",
        &user.user_id.to_string(),
        Some(serde_json::json!({
            "justification": req.justification,
            "fields_unmasked": ["phone", "address"]
        })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(UnmaskedResponse {
        id: db_user.id,
        username: db_user.username,
        email: db_user.email,
        phone,
        address,
    }))
}
