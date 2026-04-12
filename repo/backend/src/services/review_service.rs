use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::review::{ReviewSubmission, ReviewSubmissionHistory, SubmitReviewRequest};

// ---------------------------------------------------------------------------
// Submit review
// ---------------------------------------------------------------------------

/// Submits (or resubmits) a review for a given round.
///
/// Steps:
/// 1. Verify the round deadline has not passed (UTC)
/// 2. Verify the reviewer role is appropriate
/// 3. Bind the current template version
/// 4. If replacing an existing submission, save old version to history
/// 5. Increment the submission version
pub async fn submit_review(
    pool: &PgPool,
    round_id: Uuid,
    reviewer_id: Uuid,
    request: SubmitReviewRequest,
) -> Result<ReviewSubmission, AppError> {
    // Fetch round and template
    let round = sqlx::query_as::<_, RoundRow>(
        "SELECT rr.id, rr.deadline, rr.is_active, rr.template_id, rt.version AS template_version \
         FROM review_rounds rr \
         JOIN review_templates rt ON rt.id = rr.template_id \
         WHERE rr.id = $1",
    )
    .bind(round_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch round: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Review round not found".to_string()))?;

    if !round.is_active {
        return Err(AppError::BadRequest(
            "This review round is no longer active".to_string(),
        ));
    }

    if Utc::now() > round.deadline {
        return Err(AppError::BadRequest(
            "The deadline for this review round has passed".to_string(),
        ));
    }

    // Validate content against template schema (if schema defines required fields)
    let template_schema = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT schema FROM review_templates WHERE id = $1",
    )
    .bind(round.template_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch template schema: {}", e)))?;

    if let Some(schema) = template_schema {
        validate_content_against_schema(&request.content, &schema)?;
    }

    // Verify reviewer role
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role::text FROM users WHERE id = $1",
    )
    .bind(reviewer_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch user: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Reviewer not found".to_string()))?;

    if role != "Reviewer" && role != "Admin" {
        return Err(AppError::Forbidden(
            "Only Reviewers and Admins may submit reviews".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to begin transaction: {}", e)))?;

    // Check for existing submission
    let existing = sqlx::query_as::<_, ReviewSubmission>(
        "SELECT * FROM review_submissions WHERE round_id = $1 AND reviewer_id = $2",
    )
    .bind(round_id)
    .bind(reviewer_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to check existing submission: {}", e))
    })?;

    let submission = if let Some(prev) = existing {
        // Save current version to history
        sqlx::query(
            "INSERT INTO review_submission_history (id, submission_id, version, content, submitted_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(prev.id)
        .bind(prev.version)
        .bind(&prev.content)
        .bind(prev.submitted_at.unwrap_or(prev.created_at))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to save submission history: {}", e))
        })?;

        let new_version = prev.version + 1;

        // Update existing submission
        let updated = sqlx::query_as::<_, ReviewSubmission>(
            "UPDATE review_submissions SET \
                content = $1, version = $2, template_version = $3, \
                status = 'Submitted', submitted_at = NOW(), updated_at = NOW() \
             WHERE id = $4 RETURNING *",
        )
        .bind(&request.content)
        .bind(new_version)
        .bind(round.template_version)
        .bind(prev.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to update submission: {}", e))
        })?;

        updated
    } else {
        // Create new submission
        let created = sqlx::query_as::<_, ReviewSubmission>(
            "INSERT INTO review_submissions \
                (id, round_id, reviewer_id, template_version, content, version, \
                 status, submitted_at, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, 1, 'Submitted', NOW(), NOW(), NOW()) \
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(round_id)
        .bind(reviewer_id)
        .bind(round.template_version)
        .bind(&request.content)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to create submission: {}", e))
        })?;

        created
    };

    tx.commit()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to commit: {}", e)))?;

    log::info!(
        "Review submitted: submission_id={}, round_id={}, reviewer_id={}, version={}",
        submission.id,
        round_id,
        reviewer_id,
        submission.version
    );

    Ok(submission)
}

#[derive(Debug, sqlx::FromRow)]
struct RoundRow {
    #[allow(dead_code)]
    id: Uuid,
    deadline: chrono::DateTime<chrono::Utc>,
    is_active: bool,
    template_id: Uuid,
    template_version: i32,
}

// ---------------------------------------------------------------------------
// Template schema validation
// ---------------------------------------------------------------------------

/// Validates that `content` satisfies the template `schema`.
///
/// The schema is expected to be a JSON object where each key is a field name
/// and the value is an object with optional `type` (string, number, boolean)
/// and `required` (bool) properties.  Example:
///
/// ```json
/// {
///   "summary":        { "type": "string",  "required": true },
///   "recommendation": { "type": "string",  "required": true },
///   "score":          { "type": "number",  "required": false }
/// }
/// ```
fn validate_content_against_schema(
    content: &serde_json::Value,
    schema: &serde_json::Value,
) -> Result<(), AppError> {
    let schema_obj = match schema.as_object() {
        Some(o) => o,
        None => return Ok(()), // non-object schemas are treated as unconstrained
    };

    let content_obj = content.as_object().ok_or_else(|| {
        AppError::ValidationError("Submission content must be a JSON object".to_string())
    })?;

    let mut missing = Vec::new();
    let mut wrong_type = Vec::new();

    for (field_name, field_def) in schema_obj {
        let required = field_def
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let expected_type = field_def
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string");

        match content_obj.get(field_name) {
            None | Some(serde_json::Value::Null) if required => {
                missing.push(field_name.clone());
            }
            Some(val) if !val.is_null() => {
                let type_ok = match expected_type {
                    "string" => val.is_string(),
                    "number" => val.is_number(),
                    "boolean" | "bool" => val.is_boolean(),
                    _ => true, // unknown type => accept anything
                };
                if !type_ok {
                    wrong_type.push(format!(
                        "'{}' expected {}, got {}",
                        field_name,
                        expected_type,
                        value_type_name(val)
                    ));
                }
            }
            _ => {} // optional and missing — OK
        }
    }

    if !missing.is_empty() || !wrong_type.is_empty() {
        let mut parts = Vec::new();
        if !missing.is_empty() {
            parts.push(format!("Missing required fields: {}", missing.join(", ")));
        }
        if !wrong_type.is_empty() {
            parts.push(format!("Type errors: {}", wrong_type.join("; ")));
        }
        return Err(AppError::ValidationError(parts.join(". ")));
    }

    Ok(())
}

fn value_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Null => "null",
    }
}

// ---------------------------------------------------------------------------
// Submission history
// ---------------------------------------------------------------------------

/// Returns the full version history for a submission, ordered by version ascending.
pub async fn get_submission_history(
    pool: &PgPool,
    submission_id: Uuid,
) -> Result<Vec<ReviewSubmissionHistory>, AppError> {
    let history = sqlx::query_as::<_, ReviewSubmissionHistory>(
        "SELECT * FROM review_submission_history \
         WHERE submission_id = $1 ORDER BY version ASC",
    )
    .bind(submission_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        AppError::InternalError(format!("Failed to fetch submission history: {}", e))
    })?;

    Ok(history)
}

// ---------------------------------------------------------------------------
// Attachment validation
// ---------------------------------------------------------------------------

/// Validates a file attachment before upload.
///
/// Constraints:
/// - Allowed MIME types: application/pdf, image/png, image/jpeg, image/jpg
/// - Maximum file size: 10 MB (10,485,760 bytes)
/// - Maximum 5 attachments per submission (caller must check count)
pub fn validate_attachment(
    filename: &str,
    size: i64,
    mime: &str,
) -> Result<(), AppError> {
    let allowed_mimes = [
        "application/pdf",
        "image/png",
        "image/jpeg",
        "image/jpg",
    ];

    if !allowed_mimes.contains(&mime) {
        return Err(AppError::ValidationError(format!(
            "File type '{}' is not allowed. Accepted types: PDF, PNG, JPG, JPEG",
            mime
        )));
    }

    const MAX_SIZE: i64 = 10 * 1024 * 1024; // 10 MB
    if size > MAX_SIZE {
        return Err(AppError::ValidationError(format!(
            "File '{}' exceeds the 10 MB limit ({} bytes)",
            filename, size
        )));
    }

    if size <= 0 {
        return Err(AppError::ValidationError(
            "File size must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Watermark header
// ---------------------------------------------------------------------------

/// Returns a watermark header string: `"{username}:{utc_timestamp}"`.
pub fn get_watermark_header(username: &str) -> String {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    format!("{}:{}", username, timestamp)
}
