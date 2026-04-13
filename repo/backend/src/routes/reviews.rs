use actix_web::{web, HttpRequest, HttpResponse};
use actix_multipart::Multipart;
use futures::StreamExt;
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::middleware::rbac::{require_any_role, require_role};
use crate::models::review::{
    AttachmentInfo, ReviewAttachment, ReviewRound, ReviewRoundResponse, ReviewSubmission,
    ReviewSubmissionHistory, SubmissionResponse, SubmitReviewRequest,
};
use crate::services::audit_service;
use crate::services::review_service;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reviews")
            .route("/rounds", web::get().to(list_rounds))
            .route("/rounds/{id}", web::get().to(get_round))
            .route("/rounds/{id}/submit", web::post().to(submit_review))
            .route("/submissions/{id}", web::get().to(get_submission))
            .route("/submissions/{id}/history", web::get().to(get_history))
            .route(
                "/submissions/{id}/attachments",
                web::post().to(upload_attachment),
            )
            .route(
                "/attachments/{id}/download",
                web::get().to(download_attachment),
            )
            .route(
                "/attachments/{id}/approve",
                web::post().to(approve_attachment),
            ),
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct RoundDetailRow {
    id: Uuid,
    product_id: Uuid,
    template_id: Uuid,
    template_name: String,
    template_schema: Option<serde_json::Value>,
    round_number: i32,
    deadline: chrono::DateTime<chrono::Utc>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct SubmissionRow {
    id: Uuid,
    round_id: Uuid,
    reviewer_id: Uuid,
    reviewer_username: Option<String>,
    template_version: i32,
    content: serde_json::Value,
    version: i32,
    status: String,
    submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

async fn build_submission_response(
    pool: &PgPool,
    sub: &SubmissionRow,
) -> Result<SubmissionResponse, AppError> {
    let attachments = sqlx::query_as::<_, AttachmentInfoRow>(
        "SELECT id, filename, mime_type, size_bytes, approval_status, uploaded_at \
         FROM review_attachments WHERE submission_id = $1 \
         ORDER BY uploaded_at ASC",
    )
    .bind(sub.id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch attachments: {}", e)))?;

    let attachment_infos: Vec<AttachmentInfo> = attachments
        .into_iter()
        .map(|a| AttachmentInfo {
            id: a.id,
            filename: a.filename,
            mime_type: a.mime_type,
            size_bytes: a.size_bytes,
            approval_status: a.approval_status,
            uploaded_at: a.uploaded_at,
        })
        .collect();

    Ok(SubmissionResponse {
        id: sub.id,
        round_id: sub.round_id,
        reviewer_id: sub.reviewer_id,
        reviewer_username: sub.reviewer_username.clone(),
        template_version: sub.template_version,
        content: sub.content.clone(),
        version: sub.version,
        status: sub.status.clone(),
        attachments: attachment_infos,
        submitted_at: sub.submitted_at,
        created_at: sub.created_at,
        updated_at: sub.updated_at,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct AttachmentInfoRow {
    id: Uuid,
    filename: String,
    mime_type: String,
    size_bytes: i64,
    approval_status: String,
    uploaded_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// GET /api/reviews/rounds
// ---------------------------------------------------------------------------

async fn list_rounds(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    require_any_role(&user, &["Reviewer", "Admin"])?;
    let rounds = sqlx::query_as::<_, RoundDetailRow>(
        "SELECT rr.id, rr.product_id, rr.template_id, rt.name AS template_name, \
         rt.schema AS template_schema, \
         rr.round_number, rr.deadline, rr.is_active, rr.created_at \
         FROM review_rounds rr \
         JOIN review_templates rt ON rt.id = rr.template_id \
         WHERE rr.is_active = TRUE \
         ORDER BY rr.deadline ASC",
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch rounds: {}", e)))?;

    let mut responses = Vec::new();
    for round in &rounds {
        responses.push(ReviewRoundResponse {
            id: round.id,
            product_id: round.product_id,
            template_id: round.template_id,
            template_name: round.template_name.clone(),
            template_schema: round.template_schema.clone(),
            round_number: round.round_number,
            deadline: round.deadline,
            is_active: round.is_active,
            submissions: vec![], // List view omits submissions for brevity
            created_at: round.created_at,
        });
    }

    Ok(HttpResponse::Ok().json(responses))
}

// ---------------------------------------------------------------------------
// GET /api/reviews/rounds/{id}
// ---------------------------------------------------------------------------

async fn get_round(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    require_any_role(&user, &["Reviewer", "Admin"])?;
    let round_id = path.into_inner();

    let round = sqlx::query_as::<_, RoundDetailRow>(
        "SELECT rr.id, rr.product_id, rr.template_id, rt.name AS template_name, \
         rt.schema AS template_schema, \
         rr.round_number, rr.deadline, rr.is_active, rr.created_at \
         FROM review_rounds rr \
         JOIN review_templates rt ON rt.id = rr.template_id \
         WHERE rr.id = $1",
    )
    .bind(round_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch round: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Review round not found".to_string()))?;

    // Fetch submissions — scope by ownership: reviewers see only their own, admins see all
    let submissions = if user.role == "Admin" {
        sqlx::query_as::<_, SubmissionRow>(
            "SELECT rs.id, rs.round_id, rs.reviewer_id, u.username AS reviewer_username, \
             rs.template_version, rs.content, rs.version, rs.status, \
             rs.submitted_at, rs.created_at, rs.updated_at \
             FROM review_submissions rs \
             LEFT JOIN users u ON u.id = rs.reviewer_id \
             WHERE rs.round_id = $1 \
             ORDER BY rs.created_at ASC",
        )
        .bind(round_id)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch submissions: {}", e)))?
    } else {
        sqlx::query_as::<_, SubmissionRow>(
            "SELECT rs.id, rs.round_id, rs.reviewer_id, u.username AS reviewer_username, \
             rs.template_version, rs.content, rs.version, rs.status, \
             rs.submitted_at, rs.created_at, rs.updated_at \
             FROM review_submissions rs \
             LEFT JOIN users u ON u.id = rs.reviewer_id \
             WHERE rs.round_id = $1 AND rs.reviewer_id = $2 \
             ORDER BY rs.created_at ASC",
        )
        .bind(round_id)
        .bind(user.user_id)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch submissions: {}", e)))?
    };

    let mut sub_responses = Vec::new();
    for sub in &submissions {
        let resp = build_submission_response(pool.get_ref(), sub).await?;
        sub_responses.push(resp);
    }

    let response = ReviewRoundResponse {
        id: round.id,
        product_id: round.product_id,
        template_id: round.template_id,
        template_name: round.template_name,
        template_schema: round.template_schema,
        round_number: round.round_number,
        deadline: round.deadline,
        is_active: round.is_active,
        submissions: sub_responses,
        created_at: round.created_at,
    };

    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// POST /api/reviews/rounds/{id}/submit
// ---------------------------------------------------------------------------

async fn submit_review(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<SubmitReviewRequest>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_any_role(&user, &["Reviewer", "Admin"])?;

    let ip = http_req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let round_id = path.into_inner();
    let req = body.into_inner();

    let submission =
        review_service::submit_review(pool.get_ref(), round_id, user.user_id, req).await?;

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "review.submit",
        Some(&ip),
        "review_submission",
        &submission.id.to_string(),
        Some(serde_json::json!({
            "round_id": round_id,
            "version": submission.version
        })),
    )
    .await?;

    // Build response
    let sub_row = SubmissionRow {
        id: submission.id,
        round_id: submission.round_id,
        reviewer_id: submission.reviewer_id,
        reviewer_username: None,
        template_version: submission.template_version,
        content: submission.content,
        version: submission.version,
        status: submission.status,
        submitted_at: submission.submitted_at,
        created_at: submission.created_at,
        updated_at: submission.updated_at,
    };

    let response = build_submission_response(pool.get_ref(), &sub_row).await?;
    Ok(HttpResponse::Created().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/reviews/submissions/{id}
// ---------------------------------------------------------------------------

async fn get_submission(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let submission_id = path.into_inner();

    let sub = sqlx::query_as::<_, SubmissionRow>(
        "SELECT rs.id, rs.round_id, rs.reviewer_id, u.username AS reviewer_username, \
         rs.template_version, rs.content, rs.version, rs.status, \
         rs.submitted_at, rs.created_at, rs.updated_at \
         FROM review_submissions rs \
         LEFT JOIN users u ON u.id = rs.reviewer_id \
         WHERE rs.id = $1",
    )
    .bind(submission_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch submission: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Submission not found".to_string()))?;

    // Ownership check: only the owning reviewer or admin can view submission details
    if sub.reviewer_id != user.user_id && user.role != "Admin" {
        return Err(AppError::Forbidden(
            "You can only view your own submissions".to_string(),
        ));
    }

    let response = build_submission_response(pool.get_ref(), &sub).await?;
    Ok(HttpResponse::Ok().json(response))
}

// ---------------------------------------------------------------------------
// GET /api/reviews/submissions/{id}/history
// ---------------------------------------------------------------------------

async fn get_history(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let submission_id = path.into_inner();

    // Verify submission exists and check ownership
    let reviewer_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT reviewer_id FROM review_submissions WHERE id = $1",
    )
    .bind(submission_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check submission: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Submission not found".to_string()))?;

    // Ownership check: only the owning reviewer or admin can view history
    if reviewer_id != user.user_id && user.role != "Admin" {
        return Err(AppError::Forbidden(
            "You can only view history of your own submissions".to_string(),
        ));
    }

    let history = review_service::get_submission_history(pool.get_ref(), submission_id).await?;
    Ok(HttpResponse::Ok().json(history))
}

// ---------------------------------------------------------------------------
// POST /api/reviews/submissions/{id}/attachments
// ---------------------------------------------------------------------------

async fn upload_attachment(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    require_any_role(&user, &["Reviewer", "Admin"])?;

    let submission_id = path.into_inner();

    // Verify submission exists and belongs to user
    let reviewer_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT reviewer_id FROM review_submissions WHERE id = $1",
    )
    .bind(submission_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch submission: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Submission not found".to_string()))?;

    if reviewer_id != user.user_id && user.role != "Admin" {
        return Err(AppError::Forbidden(
            "You can only upload attachments to your own submissions".to_string(),
        ));
    }

    // Check current attachment count
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM review_attachments WHERE submission_id = $1",
    )
    .bind(submission_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to count attachments: {}", e)))?;

    if count >= 5 {
        return Err(AppError::ValidationError(
            "Maximum of 5 attachments per submission".to_string(),
        ));
    }

    // Process multipart upload
    let mut file_data: Vec<u8> = Vec::new();
    let mut filename = String::new();
    let mut mime_type = String::new();

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            AppError::BadRequest(format!("Failed to read multipart field: {}", e))
        })?;

        let content_disposition = field.content_disposition().clone();
        let field_name = content_disposition
            .get_name()
            .unwrap_or("")
            .to_string();

        if field_name == "file" {
            filename = content_disposition
                .get_filename()
                .unwrap_or("unnamed")
                .to_string();
            mime_type = field
                .content_type()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| {
                    AppError::InternalError(format!("Failed to read chunk: {}", e))
                })?;
                file_data.extend_from_slice(&data);
            }
        }
    }

    if file_data.is_empty() {
        return Err(AppError::ValidationError(
            "No file data received".to_string(),
        ));
    }

    let size_bytes = file_data.len() as i64;

    // Validate attachment
    review_service::validate_attachment(&filename, size_bytes, &mime_type)?;

    let attachment_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO review_attachments (id, submission_id, filename, mime_type, \
         size_bytes, file_data, uploaded_at) \
         VALUES ($1, $2, $3, $4, $5, $6, NOW())",
    )
    .bind(attachment_id)
    .bind(submission_id)
    .bind(&filename)
    .bind(&mime_type)
    .bind(size_bytes)
    .bind(&file_data)
    .execute(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to save attachment: {}", e)))?;

    let info = AttachmentInfo {
        id: attachment_id,
        filename,
        mime_type,
        size_bytes,
        approval_status: "Pending".to_string(),
        uploaded_at: chrono::Utc::now(),
    };

    Ok(HttpResponse::Created().json(info))
}

// ---------------------------------------------------------------------------
// GET /api/reviews/attachments/{id}/download
// ---------------------------------------------------------------------------

async fn download_attachment(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_any_role(&user, &["Reviewer", "Admin"])?;

    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let attachment_id = path.into_inner();

    let attachment = sqlx::query_as::<_, ReviewAttachment>(
        "SELECT * FROM review_attachments WHERE id = $1",
    )
    .bind(attachment_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch attachment: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Attachment not found".to_string()))?;

    // Ownership check: verify the requester owns the submission or is admin
    let submission_reviewer = sqlx::query_scalar::<_, Uuid>(
        "SELECT reviewer_id FROM review_submissions WHERE id = $1",
    )
    .bind(attachment.submission_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to check submission ownership: {}", e)))?
    .ok_or_else(|| AppError::NotFound("Associated submission not found".to_string()))?;

    if submission_reviewer != user.user_id && user.role != "Admin" {
        return Err(AppError::Forbidden(
            "You can only download attachments from your own submissions".to_string(),
        ));
    }

    // Non-admins can only download approved attachments
    if user.role != "Admin" && attachment.approval_status != "Approved" {
        return Err(AppError::Forbidden(
            "This attachment has not been approved for download yet".to_string(),
        ));
    }

    // Get username for watermark
    let username = sqlx::query_scalar::<_, String>(
        "SELECT username FROM users WHERE id = $1",
    )
    .bind(user.user_id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to fetch user: {}", e)))?;

    let watermark = review_service::get_watermark_header(&username);

    // Audit log
    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "review.attachment_download",
        Some(&ip),
        "review_attachment",
        &attachment_id.to_string(),
        Some(serde_json::json!({
            "filename": attachment.filename,
            "submission_id": attachment.submission_id
        })),
    )
    .await?;

    // Watermarking is HTTP-header-level: the X-Watermark header contains
    // "username:ISO-8601-timestamp" identifying who downloaded the file and when.
    // Content-level watermarking (e.g. PDF overlays, image EXIF tags) is not
    // implemented — the header approach was chosen as the baseline because it
    // works across all file types without modifying the original binary content.
    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", attachment.mime_type.as_str()))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", attachment.filename),
        ))
        .insert_header(("X-Watermark", watermark))
        .body(attachment.file_data))
}

// ---------------------------------------------------------------------------
// POST /api/reviews/attachments/{id}/approve
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ApproveAttachmentRequest {
    status: String, // "Approved" or "Rejected"
}

async fn approve_attachment(
    pool: web::Data<PgPool>,
    user: AuthenticatedUser,
    path: web::Path<Uuid>,
    body: web::Json<ApproveAttachmentRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    require_role(&user, "Admin")?;

    let attachment_id = path.into_inner();
    let ip = req.connection_info().peer_addr().unwrap_or("unknown").to_string();
    let action_req = body.into_inner();

    let valid_statuses = ["Approved", "Rejected"];
    if !valid_statuses.contains(&action_req.status.as_str()) {
        return Err(AppError::ValidationError(format!(
            "Invalid approval status '{}'. Must be 'Approved' or 'Rejected'",
            action_req.status
        )));
    }

    let rows = sqlx::query(
        "UPDATE review_attachments SET approval_status = $1::moderation_status WHERE id = $2",
    )
    .bind(&action_req.status)
    .bind(attachment_id)
    .execute(pool.get_ref())
    .await
    .map_err(|e| AppError::InternalError(format!("Failed to update attachment: {}", e)))?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound("Attachment not found".to_string()));
    }

    audit_service::log_action(
        pool.get_ref(),
        &user.user_id.to_string(),
        "review.attachment_approval",
        Some(&ip),
        "review_attachment",
        &attachment_id.to_string(),
        Some(serde_json::json!({ "approval_status": action_req.status })),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Attachment approval status updated",
        "approval_status": action_req.status
    })))
}
