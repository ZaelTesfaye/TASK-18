use crate::api::client;
use crate::types::*;

pub async fn list_users(page: u64) -> Result<PaginatedResponse<User>, ApiError> {
    client::get(&format!("/admin/users?page={}&per_page=20", page)).await
}

pub async fn change_role(user_id: &str, role: &str) -> Result<User, ApiError> {
    let body = ChangeRoleRequest {
        role: role.to_string(),
    };
    client::put(&format!("/admin/users/{}/role", user_id), &body).await
}

pub async fn reset_password(user_id: &str) -> Result<serde_json::Value, ApiError> {
    client::post(&format!("/admin/users/{}/reset-password", user_id), &serde_json::json!({})).await
}

/// Backend returns {"message": "Account unlocked"}, not a full User object.
pub async fn unlock_user(user_id: &str) -> Result<serde_json::Value, ApiError> {
    client::post(&format!("/admin/users/{}/unlock", user_id), &serde_json::json!({})).await
}

pub async fn list_risk_events() -> Result<PaginatedResponse<RiskEvent>, ApiError> {
    client::get("/admin/risk-events").await
}

/// Backend returns {"message": "Risk event updated", "status": "..."}, not a full RiskEvent.
pub async fn override_risk_event(event_id: &str, justification: &str) -> Result<serde_json::Value, ApiError> {
    let body = serde_json::json!({ "justification": justification, "status": "Approved" });
    client::put(&format!("/admin/risk-events/{}", event_id), &body).await
}

// Audit — backend is /api/audit
pub async fn get_audit_log(query: &AuditLogQuery) -> Result<PaginatedResponse<AuditLogEntry>, ApiError> {
    let mut params = Vec::new();
    if let Some(ref a) = query.actor {
        if !a.is_empty() {
            params.push(format!("actor={}", a));
        }
    }
    if let Some(ref a) = query.action {
        if !a.is_empty() {
            params.push(format!("action={}", a));
        }
    }
    if let Some(ref f) = query.from {
        if !f.is_empty() {
            // Backend expects DateTime<Utc> (RFC3339). If the user entered a bare
            // date like "2024-01-01", append T00:00:00Z to make it a valid datetime.
            let from_dt = if f.contains('T') { f.clone() } else { format!("{}T00:00:00Z", f) };
            params.push(format!("from_date={}", from_dt));
        }
    }
    if let Some(ref t) = query.to {
        if !t.is_empty() {
            // Append end-of-day if bare date
            let to_dt = if t.contains('T') { t.clone() } else { format!("{}T23:59:59Z", t) };
            params.push(format!("to_date={}", to_dt));
        }
    }
    params.push(format!("page={}", query.page.unwrap_or(1)));
    let qs = if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    };
    client::get(&format!("/audit{}", qs)).await
}

// Reports — backend is /api/reports
pub async fn get_reports(query: &ReportQuery) -> Result<ReportResponse, ApiError> {
    let mut params = Vec::new();
    if let Some(ref t) = query.report_type {
        if !t.is_empty() {
            // Backend uses #[serde(rename = "type")] so the query param is "type"
            params.push(format!("type={}", t));
        }
    }
    if let Some(ref f) = query.from {
        if !f.is_empty() {
            params.push(format!("start_date={}", f));
        }
    }
    if let Some(ref t) = query.to {
        if !t.is_empty() {
            params.push(format!("end_date={}", t));
        }
    }
    let qs = if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    };
    client::get(&format!("/reports{}", qs)).await
}

// Backup — backend is /api/backup
pub async fn create_backup() -> Result<BackupResponse, ApiError> {
    client::post("/backup", &serde_json::json!({})).await
}

pub async fn list_backups() -> Result<Vec<BackupResponse>, ApiError> {
    client::get("/backup").await
}

pub async fn verify_backup(backup_id: &str) -> Result<serde_json::Value, ApiError> {
    client::post(&format!("/backup/{}/verify", backup_id), &serde_json::json!({})).await
}

pub async fn restore_backup(backup_id: &str) -> Result<serde_json::Value, ApiError> {
    client::post(&format!("/backup/{}/restore", backup_id), &serde_json::json!({})).await
}

// Moderation — backend is /api/admin/moderation/ratings/{id}
pub async fn moderate_rating(rating_id: &str, status: &str) -> Result<serde_json::Value, ApiError> {
    let body = serde_json::json!({ "status": status });
    client::post(&format!("/admin/moderation/ratings/{}", rating_id), &body).await
}

// Retention
pub async fn run_retention() -> Result<serde_json::Value, ApiError> {
    client::post("/admin/retention/run", &serde_json::json!({})).await
}

pub async fn set_legal_hold(order_id: &str, hold: bool) -> Result<serde_json::Value, ApiError> {
    let body = serde_json::json!({ "hold": hold });
    client::post(&format!("/admin/retention/legal-hold/{}", order_id), &body).await
}

// Taxonomy — backend is /api/taxonomy/topics and /api/taxonomy/tags
pub async fn list_topics() -> Result<Vec<Topic>, ApiError> {
    client::get("/taxonomy/topics").await
}

pub async fn create_topic(req: &CreateTopicRequest) -> Result<Topic, ApiError> {
    client::post("/taxonomy/topics", req).await
}

pub async fn delete_topic(topic_id: &str, replacement_id: Option<&str>) -> Result<(), ApiError> {
    let qs = replacement_id
        .map(|r| format!("?replacement_id={}", r))
        .unwrap_or_default();
    client::delete(&format!("/taxonomy/topics/{}{}", topic_id, qs)).await
}

pub async fn list_tags() -> Result<Vec<Tag>, ApiError> {
    client::get("/taxonomy/tags").await
}

pub async fn create_tag(req: &CreateTagRequest) -> Result<Tag, ApiError> {
    client::post("/taxonomy/tags", req).await
}

pub async fn delete_tag(tag_id: &str) -> Result<(), ApiError> {
    client::delete(&format!("/taxonomy/tags/{}", tag_id)).await
}

// Custom fields — backend is /api/custom-fields
pub async fn list_fields() -> Result<Vec<CustomFieldDefinition>, ApiError> {
    client::get("/custom-fields").await
}

pub async fn create_field(req: &CreateFieldRequest) -> Result<CustomFieldDefinition, ApiError> {
    client::post("/custom-fields", req).await
}

pub async fn update_field(
    field_id: &str,
    req: &UpdateFieldRequest,
) -> Result<CustomFieldDefinition, ApiError> {
    client::put(&format!("/custom-fields/{}", field_id), req).await
}

pub async fn publish_field(field_id: &str) -> Result<CustomFieldDefinition, ApiError> {
    client::post(
        &format!("/custom-fields/{}/publish", field_id),
        &serde_json::json!({}),
    )
    .await
}
