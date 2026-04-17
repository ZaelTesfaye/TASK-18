use silverscreen_backend::services::review_service;

// ---------------------------------------------------------------------------
// Attachment validation
// ---------------------------------------------------------------------------

#[test]
fn test_validate_attachment_pdf_valid() {
    let result = review_service::validate_attachment("report.pdf", 1024, "application/pdf");
    assert!(result.is_ok(), "PDF under 10MB should be accepted");
}

#[test]
fn test_validate_attachment_png_valid() {
    let result = review_service::validate_attachment("screenshot.png", 500_000, "image/png");
    assert!(result.is_ok(), "PNG under 10MB should be accepted");
}

#[test]
fn test_validate_attachment_jpeg_valid() {
    let result = review_service::validate_attachment("photo.jpeg", 2_000_000, "image/jpeg");
    assert!(result.is_ok(), "JPEG under 10MB should be accepted");
}

#[test]
fn test_validate_attachment_jpg_valid() {
    let result = review_service::validate_attachment("photo.jpg", 2_000_000, "image/jpg");
    assert!(result.is_ok(), "JPG (image/jpg) under 10MB should be accepted");
}

#[test]
fn test_validate_attachment_disallowed_type() {
    let result = review_service::validate_attachment("malware.exe", 1024, "application/octet-stream");
    assert!(result.is_err(), "Executable MIME type should be rejected");
}

#[test]
fn test_validate_attachment_disallowed_gif() {
    let result = review_service::validate_attachment("anim.gif", 1024, "image/gif");
    assert!(result.is_err(), "GIF MIME type should be rejected");
}

#[test]
fn test_validate_attachment_exceeds_10mb() {
    let size = 10 * 1024 * 1024 + 1; // 10 MB + 1 byte
    let result = review_service::validate_attachment("large.pdf", size, "application/pdf");
    assert!(result.is_err(), "File exceeding 10MB limit should be rejected");
}

#[test]
fn test_validate_attachment_exactly_10mb() {
    let size = 10 * 1024 * 1024; // exactly 10 MB
    let result = review_service::validate_attachment("exact.pdf", size, "application/pdf");
    assert!(result.is_ok(), "File exactly at 10MB should be accepted");
}

#[test]
fn test_validate_attachment_zero_size() {
    let result = review_service::validate_attachment("empty.pdf", 0, "application/pdf");
    assert!(result.is_err(), "Zero-size file should be rejected");
}

#[test]
fn test_validate_attachment_negative_size() {
    let result = review_service::validate_attachment("neg.pdf", -1, "application/pdf");
    assert!(result.is_err(), "Negative file size should be rejected");
}

// ---------------------------------------------------------------------------
// Watermark header format
// ---------------------------------------------------------------------------

#[test]
fn test_watermark_header_format() {
    let header = review_service::get_watermark_header("alice");
    // Expected format: "alice:YYYY-MM-DDTHH:MM:SSZ"
    assert!(header.starts_with("alice:"), "Watermark should start with username");
    let parts: Vec<&str> = header.splitn(2, ':').collect();
    assert_eq!(parts.len(), 2, "Watermark should have username:timestamp format");

    let timestamp_part = &header["alice:".len()..];
    // Verify ISO-8601 structure: YYYY-MM-DDTHH:MM:SSZ
    assert!(timestamp_part.contains('T'), "Timestamp should contain 'T' separator");
    assert!(timestamp_part.ends_with('Z'), "Timestamp should end with 'Z' for UTC");
}

#[test]
fn test_watermark_header_different_users() {
    let header_a = review_service::get_watermark_header("bob");
    let header_b = review_service::get_watermark_header("carol");
    assert!(header_a.starts_with("bob:"), "Watermark for bob should start with 'bob:'");
    assert!(header_b.starts_with("carol:"), "Watermark for carol should start with 'carol:'");
}

#[test]
fn test_watermark_header_empty_username() {
    let header = review_service::get_watermark_header("");
    // Should still produce ":timestamp" format
    assert!(header.starts_with(':'), "Empty username produces colon-prefixed timestamp");
}

// ---------------------------------------------------------------------------
// Template schema validation concepts
// ---------------------------------------------------------------------------

#[test]
fn test_template_schema_structure() {
    // The schema follows { field_name: { type: "string", required: true } } pattern
    let schema = serde_json::json!({
        "summary": { "type": "string", "required": true },
        "recommendation": { "type": "string", "required": true },
        "score": { "type": "number", "required": false }
    });
    let obj = schema.as_object().unwrap();
    assert_eq!(obj.len(), 3, "Schema should have three fields");
    assert!(obj.contains_key("summary"), "Schema should contain 'summary' field");
    assert!(obj.contains_key("recommendation"), "Schema should contain 'recommendation' field");
    assert!(obj.contains_key("score"), "Schema should contain 'score' field");
}

#[test]
fn test_required_field_detection() {
    let schema = serde_json::json!({
        "title": { "type": "string", "required": true },
        "notes": { "type": "string", "required": false }
    });
    let title_def = schema.get("title").unwrap();
    let notes_def = schema.get("notes").unwrap();
    assert_eq!(
        title_def.get("required").and_then(|v| v.as_bool()),
        Some(true),
        "'title' should be required"
    );
    assert_eq!(
        notes_def.get("required").and_then(|v| v.as_bool()),
        Some(false),
        "'notes' should not be required"
    );
}
