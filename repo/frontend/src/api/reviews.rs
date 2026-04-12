use crate::api::client;
use crate::types::*;

pub async fn list_rounds() -> Result<Vec<ReviewRound>, ApiError> {
    client::get("/reviews/rounds").await
}

pub async fn get_round(round_id: &str) -> Result<ReviewRound, ApiError> {
    client::get(&format!("/reviews/rounds/{}", round_id)).await
}

pub async fn submit_review(
    round_id: &str,
    req: &SubmitReviewRequest,
) -> Result<ReviewSubmission, ApiError> {
    client::post(&format!("/reviews/rounds/{}/submit", round_id), req).await
}

pub async fn get_submission(submission_id: &str) -> Result<ReviewSubmission, ApiError> {
    client::get(&format!("/reviews/submissions/{}", submission_id)).await
}

/// List submissions for a round (fetched from the round detail endpoint).
pub async fn list_submissions(round_id: &str) -> Result<Vec<ReviewSubmission>, ApiError> {
    // The round detail response includes submissions
    let round: serde_json::Value = client::get(&format!("/reviews/rounds/{}", round_id)).await?;
    let submissions_value = round.get("submissions")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    serde_json::from_value::<Vec<ReviewSubmission>>(submissions_value)
        .map_err(|e| ApiError {
            error: "Deserialization error".into(),
            message: format!("Failed to parse submissions: {}", e),
            status: 0,
        })
}

pub async fn get_submission_history(
    submission_id: &str,
) -> Result<Vec<serde_json::Value>, ApiError> {
    client::get(&format!("/reviews/submissions/{}/history", submission_id)).await
}

/// Upload an attachment to a submission via multipart form.
/// Uses gloo_net directly because the generic client helper doesn't support multipart.
pub async fn upload_attachment(
    submission_id: &str,
    file_name: &str,
    file_bytes: &[u8],
    mime_type: &str,
) -> Result<AttachmentInfo, ApiError> {
    use gloo_net::http::Request;
    use crate::config::API_BASE_URL;
    use crate::store;

    let url = format!("{}/reviews/submissions/{}/attachments", API_BASE_URL, submission_id);

    let form_data = web_sys::FormData::new()
        .map_err(|_| ApiError { error: "FormData error".into(), message: "Failed to create FormData".into(), status: 0 })?;

    let uint8arr = ::js_sys::Uint8Array::new_with_length(file_bytes.len() as u32);
    uint8arr.copy_from(file_bytes);
    let blob_parts = ::js_sys::Array::new();
    blob_parts.push(&uint8arr);
    let mut options = web_sys::BlobPropertyBag::new();
    options.type_(mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
        .map_err(|_| ApiError { error: "Blob error".into(), message: "Failed to create Blob".into(), status: 0 })?;

    form_data.append_with_blob_and_filename("file", &blob, file_name)
        .map_err(|_| ApiError { error: "FormData error".into(), message: "Failed to append file".into(), status: 0 })?;

    let req = Request::post(&url);
    let req = if let Some(token) = store::get_access_token() {
        req.header("Authorization", &format!("Bearer {}", token))
    } else {
        req
    };
    let req = req.body(form_data)
        .map_err(|e| ApiError { error: "Request error".into(), message: e.to_string(), status: 0 })?;
    let resp = req.send().await
        .map_err(|e| ApiError { error: "Network error".into(), message: e.to_string(), status: 0 })?;

    if resp.status() >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(ApiError { error: format!("HTTP {}", resp.status()), message: body, status: resp.status() });
    }

    let body = resp.text().await
        .map_err(|e| ApiError { error: "Parse error".into(), message: e.to_string(), status: 0 })?;
    serde_json::from_str(&body)
        .map_err(|e| ApiError { error: "Deserialization error".into(), message: e.to_string(), status: 0 })
}

/// Get the download URL for an attachment (caller navigates to it).
pub fn attachment_download_url(attachment_id: &str) -> String {
    use crate::config::API_BASE_URL;
    format!("{}/reviews/attachments/{}/download", API_BASE_URL, attachment_id)
}
