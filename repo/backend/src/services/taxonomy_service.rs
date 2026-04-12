use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

// ---------------------------------------------------------------------------
// Acyclic check
// ---------------------------------------------------------------------------

/// Verifies that setting `proposed_parent_id` as the parent of `child_id`
/// would not create a cycle in the topic DAG. Also enforces maximum depth <= 5.
///
/// Walks the ancestor chain from `proposed_parent_id` upward. If `child_id`
/// is encountered, a cycle would be formed.
pub async fn check_acyclic(
    pool: &PgPool,
    child_id: Uuid,
    proposed_parent_id: Uuid,
) -> Result<(), AppError> {
    if child_id == proposed_parent_id {
        return Err(AppError::BadRequest(
            "A topic cannot be its own parent".to_string(),
        ));
    }

    let mut current_id = Some(proposed_parent_id);
    let mut depth = 1; // the child will be one level below proposed_parent

    while let Some(id) = current_id {
        if id == child_id {
            return Err(AppError::BadRequest(
                "Setting this parent would create a cycle in the topic hierarchy".to_string(),
            ));
        }

        depth += 1;
        if depth > 5 {
            return Err(AppError::BadRequest(
                "Maximum topic hierarchy depth of 5 would be exceeded".to_string(),
            ));
        }

        let parent = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT parent_id FROM topics WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to fetch topic: {}", e))
        })?;

        // flatten Option<Option<Uuid>> -> Option<Uuid>
        current_id = parent.flatten();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Safe delete
// ---------------------------------------------------------------------------

/// Safely deletes a topic by first reassigning all its products to a
/// replacement topic, then removing the topic record.
pub async fn safe_delete_topic(
    pool: &PgPool,
    topic_id: Uuid,
    replacement_id: Uuid,
) -> Result<(), AppError> {
    if topic_id == replacement_id {
        return Err(AppError::BadRequest(
            "Replacement topic must differ from the topic being deleted".to_string(),
        ));
    }

    // Verify replacement exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM topics WHERE id = $1)",
    )
    .bind(replacement_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to verify topic: {}", e)))?;

    if !exists {
        return Err(AppError::NotFound(
            "Replacement topic not found".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Reassign products
    sqlx::query("UPDATE product_topics SET topic_id = $1 WHERE topic_id = $2")
        .bind(replacement_id)
        .bind(topic_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to reassign products: {}", e))
        })?;

    // Re-parent child topics
    sqlx::query("UPDATE topics SET parent_id = $1 WHERE parent_id = $2")
        .bind(replacement_id)
        .bind(topic_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to re-parent children: {}", e))
        })?;

    // Delete the topic
    sqlx::query("DELETE FROM topics WHERE id = $1")
        .bind(topic_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to delete topic: {}", e))
        })?;

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    log::info!(
        "Topic deleted: topic_id={}, replacement_id={}",
        topic_id,
        replacement_id
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Depth calculation
// ---------------------------------------------------------------------------

/// Calculates the depth of a topic by walking up its parent chain.
pub async fn calculate_depth(pool: &PgPool, topic_id: Uuid) -> Result<i32, AppError> {
    let mut depth = 0i32;
    let mut current_id = Some(topic_id);

    while let Some(id) = current_id {
        let parent = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT parent_id FROM topics WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to fetch topic: {}", e))
        })?;

        match parent {
            Some(maybe_parent) => {
                current_id = maybe_parent;
                if current_id.is_some() {
                    depth += 1;
                }
            }
            None => {
                return Err(AppError::NotFound(format!(
                    "Topic {} not found during depth calculation",
                    id
                )));
            }
        }
    }

    Ok(depth)
}
