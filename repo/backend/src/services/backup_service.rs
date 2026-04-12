// Backup encryption uses AES-256 with local key -- no third-party services

use base64::Engine;
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::models::backup::Backup;
use crate::services::encryption_service;

// ---------------------------------------------------------------------------
// Create backup
// ---------------------------------------------------------------------------

/// Creates an encrypted database backup:
/// 1. Exports key tables to JSON
/// 2. Encrypts the JSON payload with AES-256-GCM
/// 3. Writes to BACKUP_DIR
/// 4. Computes SHA-256 checksum
/// 5. Records in backups table
/// 6. Prunes old backups beyond BACKUP_RETENTION_COUNT
pub async fn create_backup(
    pool: &PgPool,
    config: &Config,
) -> Result<Backup, AppError> {
    let backup_id = Uuid::new_v4();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("backup_{}_{}.enc", timestamp, backup_id);
    let filepath = format!("{}/{}", config.backup_dir, filename);

    log::info!("Starting backup: {}", filename);

    // Export tables to JSON
    let data = export_database_json(pool).await?;
    let json_bytes = serde_json::to_vec(&data)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize backup data: {}", e)))?;

    // Encrypt
    let key = derive_backup_key(&config.backup_encryption_key);
    let encrypted = encryption_service::encrypt(
        &base64::engine::general_purpose::STANDARD.encode(&json_bytes),
        &key,
        1,
    )?;

    let encrypted_bytes = encrypted.as_bytes();

    // Compute checksum
    let mut hasher = Sha256::new();
    hasher.update(encrypted_bytes);
    let checksum = hex::encode(hasher.finalize());

    // Write to filesystem
    std::fs::create_dir_all(&config.backup_dir).map_err(|e| {
        AppError::InternalError(format!("Failed to create backup directory: {}", e))
    })?;

    std::fs::write(&filepath, encrypted_bytes).map_err(|e| {
        AppError::InternalError(format!("Failed to write backup file: {}", e))
    })?;

    let size_bytes = encrypted_bytes.len() as i64;

    // Record in database
    let backup = sqlx::query_as::<_, Backup>(
        "INSERT INTO backups (id, filename, checksum_sha256, size_bytes, status, created_at) \
         VALUES ($1, $2, $3, $4, 'Completed'::backup_status, NOW()) RETURNING *",
    )
    .bind(backup_id)
    .bind(&filename)
    .bind(&checksum)
    .bind(size_bytes)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to record backup: {}", e)))?;

    // Prune old backups
    prune_old_backups(pool, config).await?;

    log::info!(
        "Backup completed: id={}, file={}, size={}, checksum={}",
        backup_id,
        filename,
        size_bytes,
        &checksum[..8]
    );

    Ok(backup)
}

/// Derives a 32-byte key from the backup encryption key string using SHA-256.
fn derive_backup_key(key_str: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key_str.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Helper to export a single table to JSON rows.
async fn export_table(pool: &PgPool, table: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let sql = format!("SELECT row_to_json(t) FROM (SELECT * FROM {}) t", table);
    sqlx::query_scalar(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to export {}: {}", table, e)))
}

/// Exports all critical database tables to a JSON value.
/// Tables are exported in referential order so restore can insert them safely.
async fn export_database_json(pool: &PgPool) -> Result<serde_json::Value, AppError> {
    // Users — includes password_hash (already Argon2-hashed, safe to export)
    // so that restored users can log in without needing a password reset.
    let users: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT row_to_json(u) FROM \
         (SELECT id, username, email, password_hash, role, is_locked, legal_hold, created_at FROM users) u",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to export users: {}", e)))?;

    let topics = export_table(pool, "topics").await?;
    let tags = export_table(pool, "tags").await?;
    let products = export_table(pool, "products").await?;
    let product_topics = export_table(pool, "product_topics").await?;
    let product_tags = export_table(pool, "product_tags").await?;
    let custom_field_definitions = export_table(pool, "custom_field_definitions").await?;
    let custom_field_values = export_table(pool, "custom_field_values").await?;
    let carts = export_table(pool, "carts").await?;
    let cart_items = export_table(pool, "cart_items").await?;
    let orders = export_table(pool, "orders").await?;
    let order_items = export_table(pool, "order_items").await?;
    let order_lineage = export_table(pool, "order_lineage").await?;
    let invoices = export_table(pool, "invoices").await?;
    let payment_events = export_table(pool, "payment_events").await?;
    let ratings = export_table(pool, "ratings").await?;
    let rating_dimensions = export_table(pool, "rating_dimensions").await?;
    let product_scores = export_table(pool, "product_scores").await?;
    let review_templates = export_table(pool, "review_templates").await?;
    let review_rounds = export_table(pool, "review_rounds").await?;
    let review_submissions = export_table(pool, "review_submissions").await?;
    let review_submission_history = export_table(pool, "review_submission_history").await?;
    // Export attachments with file_data base64-encoded for JSON transport
    let review_attachments: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT row_to_json(a) FROM \
         (SELECT id, submission_id, filename, mime_type, size_bytes, \
          encode(file_data, 'base64') AS file_data_b64, \
          approval_status, uploaded_at \
          FROM review_attachments) a",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to export review_attachments: {}", e)))?;
    let risk_events = export_table(pool, "risk_events").await?;

    Ok(serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "users": users,
        "topics": topics,
        "tags": tags,
        "products": products,
        "product_topics": product_topics,
        "product_tags": product_tags,
        "custom_field_definitions": custom_field_definitions,
        "custom_field_values": custom_field_values,
        "carts": carts,
        "cart_items": cart_items,
        "orders": orders,
        "order_items": order_items,
        "order_lineage": order_lineage,
        "invoices": invoices,
        "payment_events": payment_events,
        "ratings": ratings,
        "rating_dimensions": rating_dimensions,
        "product_scores": product_scores,
        "review_templates": review_templates,
        "review_rounds": review_rounds,
        "review_submissions": review_submissions,
        "review_submission_history": review_submission_history,
        "review_attachments": review_attachments,
        "risk_events": risk_events,
    }))
}

/// Removes backups beyond the configured retention count, oldest first.
async fn prune_old_backups(pool: &PgPool, config: &Config) -> Result<(), AppError> {
    let to_delete = sqlx::query_as::<_, Backup>(
        "SELECT * FROM backups WHERE status = 'Completed' \
         ORDER BY created_at DESC OFFSET $1",
    )
    .bind(config.backup_retention_count as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to query old backups: {}", e))
    })?;

    for old in &to_delete {
        let path = format!("{}/{}", config.backup_dir, old.filename);
        if let Err(e) = std::fs::remove_file(&path) {
            log::warn!("Failed to delete old backup file {}: {}", old.filename, e);
        }
        sqlx::query("DELETE FROM backups WHERE id = $1")
            .bind(old.id)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to delete backup record: {}", e))
            })?;
    }

    if !to_delete.is_empty() {
        log::info!("Pruned {} old backups", to_delete.len());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Verify backup
// ---------------------------------------------------------------------------

/// Verifies a backup by reading the file and comparing its SHA-256 checksum
/// against the stored value.
pub async fn verify_backup(
    pool: &PgPool,
    config: &Config,
    backup_id: Uuid,
) -> Result<bool, AppError> {
    let backup = sqlx::query_as::<_, Backup>(
        "SELECT * FROM backups WHERE id = $1",
    )
    .bind(backup_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch backup: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Backup not found".to_string()))?;

    let filepath = format!("{}/{}", config.backup_dir, backup.filename);
    let data = std::fs::read(&filepath).map_err(|e| {
        AppError::InternalError(format!("Failed to read backup file: {}", e))
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let computed = hex::encode(hasher.finalize());

    let valid = computed == backup.checksum_sha256;
    if !valid {
        log::warn!(
            "Backup checksum mismatch: id={}, expected={}, computed={}",
            backup_id,
            &backup.checksum_sha256[..8],
            &computed[..8]
        );
    }

    Ok(valid)
}

// ---------------------------------------------------------------------------
// Restore backup
// ---------------------------------------------------------------------------

/// Result of a restore operation.
#[derive(Debug, serde::Serialize)]
pub struct RestoreResult {
    pub backup_id: Uuid,
    pub users_restored: usize,
    pub products_restored: usize,
    pub orders_restored: usize,
    pub tables_restored: std::collections::HashMap<String, usize>,
}

/// Restores a backup: verifies checksum, decrypts, and applies data.
///
/// The restore process:
/// 1. Verify the SHA-256 checksum against stored value.
/// 2. Decrypt the AES-256-GCM encrypted payload.
/// 3. Parse the JSON export.
/// 4. Upsert records back into the database within a transaction.
///
/// **Manual verification required** for production: the caller should confirm
/// intent before invoking this endpoint, as it modifies live data.
pub async fn restore_backup(
    pool: &PgPool,
    config: &Config,
    backup_id: Uuid,
) -> Result<RestoreResult, AppError> {
    // Verify checksum first
    let valid = verify_backup(pool, config, backup_id).await?;
    if !valid {
        return Err(AppError::BadRequest(
            "Backup checksum verification failed -- file may be corrupted".to_string(),
        ));
    }

    let backup = sqlx::query_as::<_, Backup>(
        "SELECT * FROM backups WHERE id = $1",
    )
    .bind(backup_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch backup: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Backup not found".to_string()))?;

    let filepath = format!("{}/{}", config.backup_dir, backup.filename);
    let encrypted_str = std::fs::read_to_string(&filepath).map_err(|e| {
        AppError::InternalError(format!("Failed to read backup file: {}", e))
    })?;

    // Decrypt
    let key = derive_backup_key(&config.backup_encryption_key);
    let mut keys = std::collections::HashMap::new();
    keys.insert(1u32, key.to_vec());

    let decrypted_b64 = encryption_service::decrypt(&encrypted_str, &keys)?;
    let json_bytes = base64::engine::general_purpose::STANDARD
        .decode(&decrypted_b64)
        .map_err(|e| AppError::InternalError(format!("Failed to decode backup payload: {}", e)))?;

    let data: serde_json::Value = serde_json::from_slice(&json_bytes)
        .map_err(|e| AppError::InternalError(format!("Failed to parse backup JSON: {}", e)))?;

    // Apply data within a transaction
    let mut tx = pool.begin().await
        .map_err(|e| AppError::InternalError(format!("Failed to start transaction: {}", e)))?;

    let mut users_restored = 0usize;
    let mut products_restored = 0usize;
    let mut orders_restored = 0usize;
    let mut tables_restored = std::collections::HashMap::<String, usize>::new();

    // Restore users (upsert by id) — uses actual password_hash from backup
    // so restored users can log in immediately without a reset.
    if let Some(users) = data.get("users").and_then(|v| v.as_array()) {
        for user in users {
            let id = user.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let email = user.get("email").and_then(|v| v.as_str()).unwrap_or("");
            let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("Shopper");
            // Use the original Argon2 hash from the backup; fall back to a random
            // unusable placeholder only if the backup predates password_hash export.
            let password_hash = user.get("password_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let password_hash = if password_hash.is_empty() {
                format!("LEGACY_BACKUP_NO_HASH_{}", Uuid::new_v4())
            } else {
                password_hash
            };

            if id.is_empty() || username.is_empty() {
                continue;
            }

            let uid: Uuid = id.parse().map_err(|_| {
                AppError::InternalError(format!("Invalid user ID in backup: {}", id))
            })?;

            let created_at = user.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, COALESCE($6::timestamptz, NOW()), NOW()) \
                 ON CONFLICT (id) DO UPDATE SET email = $3, password_hash = $4, role = $5, updated_at = NOW()",
            )
            .bind(uid)
            .bind(username)
            .bind(email)
            .bind(&password_hash)
            .bind(role)
            .bind(created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to restore user: {}", e)))?;

            users_restored += 1;
        }
        tables_restored.insert("users".into(), users_restored);
    }

    // Restore taxonomy (topics, tags) — must come before products
    if let Some(rows) = data.get("topics").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        // Sort by depth so parents are inserted first
        let mut sorted: Vec<&serde_json::Value> = rows.iter().collect();
        sorted.sort_by_key(|r| r.get("depth").and_then(|v| v.as_i64()).unwrap_or(0));
        for row in sorted {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let tid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid topic ID: {}", id)))?;
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let slug = row.get("slug").and_then(|v| v.as_str()).unwrap_or("");
            let parent_id = row.get("parent_id").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok());
            let depth = row.get("depth").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO topics (id, name, slug, parent_id, depth, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, COALESCE($6::timestamptz, NOW()), COALESCE($7::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(tid).bind(name).bind(slug).bind(parent_id).bind(depth).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore topic: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("topics".into(), count);
    }

    if let Some(rows) = data.get("tags").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let tid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid tag ID: {}", id)))?;
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let slug = row.get("slug").and_then(|v| v.as_str()).unwrap_or("");
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO tags (id, name, slug, created_at) VALUES ($1, $2, $3, COALESCE($4::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(tid).bind(name).bind(slug).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore tag: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("tags".into(), count);
    }

    // Restore products (upsert by id)
    if let Some(products) = data.get("products").and_then(|v| v.as_array()) {
        for product in products {
            let id = product.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let title = product.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || title.is_empty() { continue; }

            let pid: Uuid = id.parse().map_err(|_| {
                AppError::InternalError(format!("Invalid product ID in backup: {}", id))
            })?;
            let price = product.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let stock = product.get("stock").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            let created_at = product.get("created_at").and_then(|v| v.as_str());
            let updated_at = product.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO products (id, title, price, stock, is_active, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, TRUE, COALESCE($5::timestamptz, NOW()), COALESCE($6::timestamptz, NOW())) \
                 ON CONFLICT (id) DO UPDATE SET title = $2, price = $3, stock = $4, updated_at = COALESCE($6::timestamptz, NOW())",
            )
            .bind(pid).bind(title).bind(price).bind(stock).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore product: {}", e)))?;
            products_restored += 1;
        }
        tables_restored.insert("products".into(), products_restored);
    }

    // Restore product_topics, product_tags
    for (key, table) in [("product_topics", "product_topics"), ("product_tags", "product_tags")] {
        if let Some(rows) = data.get(key).and_then(|v| v.as_array()) {
            let mut count = 0usize;
            for row in rows {
                let pid = row.get("product_id").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok());
                let tid_key = if table == "product_topics" { "topic_id" } else { "tag_id" };
                let tid = row.get(tid_key).and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok());
                if let (Some(p), Some(t)) = (pid, tid) {
                    let sql = format!(
                        "INSERT INTO {} (product_id, {}) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                        table, tid_key
                    );
                    sqlx::query(&sql).bind(p).bind(t)
                        .execute(&mut *tx).await
                        .map_err(|e| AppError::InternalError(format!("Failed to restore {}: {}", table, e)))?;
                    count += 1;
                }
            }
            tables_restored.insert(table.into(), count);
        }
    }

    // Restore custom fields
    if let Some(rows) = data.get("custom_field_definitions").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let fid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid field def ID: {}", id)))?;
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let slug = row.get("slug").and_then(|v| v.as_str()).unwrap_or("");
            let field_type = row.get("field_type").and_then(|v| v.as_str()).unwrap_or("Text");
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO custom_field_definitions (id, name, slug, field_type, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4::field_type, COALESCE($5::timestamptz, NOW()), COALESCE($6::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(fid).bind(name).bind(slug).bind(field_type).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore custom field def: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("custom_field_definitions".into(), count);
    }

    if let Some(rows) = data.get("custom_field_values").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let vid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid field val ID: {}", id)))?;
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let fid: Uuid = row.get("field_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let value = row.get("value").cloned().unwrap_or(serde_json::json!(null));
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO custom_field_values (id, product_id, field_id, value, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, COALESCE($5::timestamptz, NOW()), COALESCE($6::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(vid).bind(pid).bind(fid).bind(&value).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore custom field val: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("custom_field_values".into(), count);
    }

    // Restore carts and cart_items
    if let Some(rows) = data.get("carts").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let cid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid cart ID: {}", id)))?;
            let uid: Uuid = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO carts (id, user_id, created_at, updated_at) VALUES ($1, $2, COALESCE($3::timestamptz, NOW()), COALESCE($4::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(cid).bind(uid).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore cart: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("carts".into(), count);
    }

    if let Some(rows) = data.get("cart_items").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let iid: Uuid = id.parse().map_err(|_| AppError::InternalError(format!("Invalid cart item ID: {}", id)))?;
            let cid: Uuid = row.get("cart_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let qty = row.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO cart_items (id, cart_id, product_id, quantity, created_at) VALUES ($1, $2, $3, $4, COALESCE($5::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(iid).bind(cid).bind(pid).bind(qty).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore cart item: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("cart_items".into(), count);
    }

    // Restore orders (insert only — don't overwrite existing orders)
    if let Some(orders) = data.get("orders").and_then(|v| v.as_array()) {
        for order in orders {
            let id = order.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let oid: Uuid = id.parse().map_err(|_| {
                AppError::InternalError(format!("Invalid order ID in backup: {}", id))
            })?;
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM orders WHERE id = $1)",
            )
            .bind(oid).fetch_one(&mut *tx).await.unwrap_or(false);

            if !exists {
                let user_id_str = order.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
                let status = order.get("status").and_then(|v| v.as_str()).unwrap_or("Cancelled");
                let parent_order_id = order.get("parent_order_id").and_then(|v| v.as_str()).and_then(|s| s.parse::<Uuid>().ok());
                let shipping_enc = order.get("shipping_address_encrypted").and_then(|v| v.as_str()).unwrap_or("");
                let total = order.get("total_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let discount = order.get("discount_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let reason_code = order.get("reason_code").and_then(|v| v.as_str());
                let payment_method = order.get("payment_method").and_then(|v| v.as_str());
                let reservation_expires = order.get("reservation_expires_at").and_then(|v| v.as_str());
                let paid_at = order.get("paid_at").and_then(|v| v.as_str());
                let shipped_at = order.get("shipped_at").and_then(|v| v.as_str());
                let delivered_at = order.get("delivered_at").and_then(|v| v.as_str());
                let completed_at = order.get("completed_at").and_then(|v| v.as_str());
                let cancelled_at = order.get("cancelled_at").and_then(|v| v.as_str());
                let refunded_at = order.get("refunded_at").and_then(|v| v.as_str());
                let legal_hold = order.get("legal_hold").and_then(|v| v.as_bool()).unwrap_or(false);
                let created_at = order.get("created_at").and_then(|v| v.as_str());
                let updated_at = order.get("updated_at").and_then(|v| v.as_str());
                if let Ok(uid) = user_id_str.parse::<Uuid>() {
                    sqlx::query(
                        "INSERT INTO orders (id, user_id, status, parent_order_id, \
                         shipping_address_encrypted, total_amount, discount_amount, \
                         reason_code, payment_method, \
                         reservation_expires_at, paid_at, shipped_at, delivered_at, \
                         completed_at, cancelled_at, refunded_at, legal_hold, \
                         created_at, updated_at) \
                         VALUES ($1, $2, $3::order_status, $4, $5, $6, $7, \
                         $8::return_reason, $9, \
                         $10::timestamptz, $11::timestamptz, $12::timestamptz, $13::timestamptz, \
                         $14::timestamptz, $15::timestamptz, $16::timestamptz, $17, \
                         COALESCE($18::timestamptz, NOW()), COALESCE($19::timestamptz, NOW())) \
                         ON CONFLICT DO NOTHING",
                    )
                    .bind(oid)              // $1
                    .bind(uid)              // $2
                    .bind(status)           // $3
                    .bind(parent_order_id)  // $4
                    .bind(shipping_enc)     // $5
                    .bind(total)            // $6
                    .bind(discount)         // $7
                    .bind(reason_code)      // $8
                    .bind(payment_method)   // $9
                    .bind(reservation_expires) // $10
                    .bind(paid_at)          // $11
                    .bind(shipped_at)       // $12
                    .bind(delivered_at)     // $13
                    .bind(completed_at)     // $14
                    .bind(cancelled_at)     // $15
                    .bind(refunded_at)      // $16
                    .bind(legal_hold)       // $17
                    .bind(created_at)       // $18
                    .bind(updated_at)       // $19
                    .execute(&mut *tx).await
                    .map_err(|e| AppError::InternalError(format!("Failed to restore order: {}", e)))?;
                    orders_restored += 1;
                }
            }
        }
        tables_restored.insert("orders".into(), orders_restored);
    }

    // Restore order_items
    if let Some(rows) = data.get("order_items").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let iid: Uuid = id.parse().unwrap_or_default();
            let oid: Uuid = row.get("order_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let qty = row.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let up = row.get("unit_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let tp = row.get("total_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, total_price, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(iid).bind(oid).bind(pid).bind(qty).bind(up).bind(tp).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore order item: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("order_items".into(), count);
    }

    // Restore order_lineage (must come after orders)
    if let Some(rows) = data.get("order_lineage").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let lid: Uuid = id.parse().unwrap_or_default();
            let parent: Uuid = row.get("parent_order_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let child: Uuid = row.get("child_order_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let op = row.get("operation").and_then(|v| v.as_str()).unwrap_or("split");
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO order_lineage (id, parent_order_id, child_order_id, operation, created_at) \
                 VALUES ($1, $2, $3, $4, COALESCE($5::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(lid).bind(parent).bind(child).bind(op).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore order lineage: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("order_lineage".into(), count);
    }

    // Restore invoices (must come after orders)
    if let Some(rows) = data.get("invoices").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let iid: Uuid = id.parse().unwrap_or_default();
            let oid: Uuid = row.get("order_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let inv_num = row.get("invoice_number").and_then(|v| v.as_str()).unwrap_or("");
            let total = row.get("total_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let line_items = row.get("line_items").cloned().unwrap_or(serde_json::json!([]));
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO invoices (id, order_id, invoice_number, total_amount, line_items, created_at) \
                 VALUES ($1, $2, $3, $4, $5, COALESCE($6::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(iid).bind(oid).bind(inv_num).bind(total).bind(&line_items).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore invoice: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("invoices".into(), count);
    }

    // Restore payment_events
    if let Some(rows) = data.get("payment_events").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let eid: Uuid = id.parse().unwrap_or_default();
            let oid: Uuid = row.get("order_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let ikey = row.get("idempotency_key").and_then(|v| v.as_str()).unwrap_or("");
            let amt = row.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("Pending");
            let pm = row.get("payment_method").and_then(|v| v.as_str()).unwrap_or("local_tender");
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO payment_events (id, order_id, idempotency_key, amount, status, payment_method, created_at) \
                 VALUES ($1, $2, $3, $4, $5::payment_status, $6, COALESCE($7::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(eid).bind(oid).bind(ikey).bind(amt).bind(status).bind(pm).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore payment event: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("payment_events".into(), count);
    }

    // Restore ratings and rating_dimensions
    if let Some(rows) = data.get("ratings").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let rid: Uuid = id.parse().unwrap_or_default();
            let uid: Uuid = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO ratings (id, user_id, product_id, created_at, updated_at) \
                 VALUES ($1, $2, $3, COALESCE($4::timestamptz, NOW()), COALESCE($5::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(rid).bind(uid).bind(pid).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore rating: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("ratings".into(), count);
    }

    if let Some(rows) = data.get("rating_dimensions").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let did: Uuid = id.parse().unwrap_or_default();
            let rid: Uuid = row.get("rating_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let dim = row.get("dimension_name").and_then(|v| v.as_str()).unwrap_or("");
            let score = row.get("score").and_then(|v| v.as_i64()).unwrap_or(5) as i32;
            sqlx::query(
                "INSERT INTO rating_dimensions (id, rating_id, dimension_name, score) \
                 VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING",
            )
            .bind(did).bind(rid).bind(dim).bind(score)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore rating dimension: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("rating_dimensions".into(), count);
    }

    // Restore product_scores
    if let Some(rows) = data.get("product_scores").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let avg = row.get("average_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let total = row.get("total_ratings").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO product_scores (product_id, average_score, total_ratings, updated_at) \
                 VALUES ($1, $2, $3, COALESCE($4::timestamptz, NOW())) ON CONFLICT (product_id) DO NOTHING",
            )
            .bind(pid).bind(avg).bind(total).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore product score: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("product_scores".into(), count);
    }

    // Restore review system
    if let Some(rows) = data.get("review_templates").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let tid: Uuid = id.parse().unwrap_or_default();
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let schema = row.get("schema").cloned().unwrap_or(serde_json::json!({}));
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO review_templates (id, name, schema, created_at, updated_at) \
                 VALUES ($1, $2, $3, COALESCE($4::timestamptz, NOW()), COALESCE($5::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(tid).bind(name).bind(&schema).bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore review template: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("review_templates".into(), count);
    }

    if let Some(rows) = data.get("review_rounds").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let rid: Uuid = id.parse().unwrap_or_default();
            let pid: Uuid = row.get("product_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let tid: Uuid = row.get("template_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let round_num = row.get("round_number").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let deadline = row.get("deadline").and_then(|v| v.as_str());
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO review_rounds (id, product_id, template_id, round_number, deadline, created_at) \
                 VALUES ($1, $2, $3, $4, COALESCE($5::timestamptz, NOW() + INTERVAL '30 days'), \
                 COALESCE($6::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(rid).bind(pid).bind(tid).bind(round_num).bind(deadline).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore review round: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("review_rounds".into(), count);
    }

    if let Some(rows) = data.get("review_submissions").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let sid: Uuid = id.parse().unwrap_or_default();
            let rid: Uuid = row.get("round_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let rev: Uuid = row.get("reviewer_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let tv = row.get("template_version").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let content = row.get("content").cloned().unwrap_or(serde_json::json!({}));
            let version = row.get("version").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("Draft");
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            let updated_at = row.get("updated_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO review_submissions (id, round_id, reviewer_id, template_version, content, version, status, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7::review_status, \
                 COALESCE($8::timestamptz, NOW()), COALESCE($9::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(sid).bind(rid).bind(rev).bind(tv).bind(&content).bind(version).bind(status)
            .bind(created_at).bind(updated_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore review submission: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("review_submissions".into(), count);
    }

    // Restore review_submission_history (must come after review_submissions)
    if let Some(rows) = data.get("review_submission_history").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let hid: Uuid = id.parse().unwrap_or_default();
            let sid: Uuid = row.get("submission_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let version = row.get("version").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
            let content = row.get("content").cloned().unwrap_or(serde_json::json!({}));
            let submitted_at = row.get("submitted_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO review_submission_history (id, submission_id, version, content, submitted_at) \
                 VALUES ($1, $2, $3, $4, COALESCE($5::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(hid).bind(sid).bind(version).bind(&content).bind(submitted_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore submission history: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("review_submission_history".into(), count);
    }

    // Restore review_attachments with full binary data (base64-decoded)
    // Must come after review_submissions
    if let Some(rows) = data.get("review_attachments").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let aid: Uuid = id.parse().unwrap_or_default();
            let sid: Uuid = row.get("submission_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let filename = row.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let mime = row.get("mime_type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream");
            let size = row.get("size_bytes").and_then(|v| v.as_i64()).unwrap_or(0);
            let approval = row.get("approval_status").and_then(|v| v.as_str()).unwrap_or("Pending");
            // Decode the base64-encoded file data back to raw bytes
            let file_bytes: Vec<u8> = row.get("file_data_b64")
                .and_then(|v| v.as_str())
                .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                .unwrap_or_default();
            let uploaded_at = row.get("uploaded_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO review_attachments (id, submission_id, filename, mime_type, size_bytes, \
                 file_data, approval_status, uploaded_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7::moderation_status, COALESCE($8::timestamptz, NOW())) \
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(aid).bind(sid).bind(filename).bind(mime).bind(size).bind(&file_bytes).bind(approval).bind(uploaded_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore review attachment: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("review_attachments".into(), count);
    }

    // Restore risk_events
    if let Some(rows) = data.get("risk_events").and_then(|v| v.as_array()) {
        let mut count = 0usize;
        for row in rows {
            let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() { continue; }
            let eid: Uuid = id.parse().unwrap_or_default();
            let uid: Uuid = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("").parse().unwrap_or_default();
            let etype = row.get("event_type").and_then(|v| v.as_str()).unwrap_or("BulkOrder");
            let status = row.get("status").and_then(|v| v.as_str()).unwrap_or("Flagged");
            let details = row.get("details").cloned();
            let created_at = row.get("created_at").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO risk_events (id, user_id, event_type, status, details, created_at) \
                 VALUES ($1, $2, $3::risk_event_type, $4::risk_event_status, $5, COALESCE($6::timestamptz, NOW())) ON CONFLICT (id) DO NOTHING",
            )
            .bind(eid).bind(uid).bind(etype).bind(status).bind(details).bind(created_at)
            .execute(&mut *tx).await
            .map_err(|e| AppError::InternalError(format!("Failed to restore risk event: {}", e)))?;
            count += 1;
        }
        tables_restored.insert("risk_events".into(), count);
    }

    tx.commit().await
        .map_err(|e| AppError::InternalError(format!("Failed to commit restore transaction: {}", e)))?;

    log::info!(
        "Backup restore applied: id={}, users={}, products={}, orders={}, additional_tables={}",
        backup_id, users_restored, products_restored, orders_restored, tables_restored.len()
    );

    Ok(RestoreResult {
        backup_id,
        users_restored,
        products_restored,
        orders_restored,
        tables_restored,
    })
}
