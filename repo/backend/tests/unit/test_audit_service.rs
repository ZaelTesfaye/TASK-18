use chrono::Timelike;
use silverscreen_backend::models::audit::AuditQuery;

// ---------------------------------------------------------------------------
// AuditQuery pagination parsing
// ---------------------------------------------------------------------------

#[test]
fn test_audit_query_default_page() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: None,
    };
    assert_eq!(query.page(), 1, "Default page should be 1");
}

#[test]
fn test_audit_query_default_per_page() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: None,
    };
    assert_eq!(query.per_page(), 20, "Default per_page should be 20");
}

#[test]
fn test_audit_query_custom_page() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(3),
        per_page: None,
    };
    assert_eq!(query.page(), 3, "Custom page should be used");
}

#[test]
fn test_audit_query_per_page_clamped_to_max() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: Some(500),
    };
    assert_eq!(query.per_page(), 100, "per_page should be clamped to max 100");
}

#[test]
fn test_audit_query_per_page_clamped_to_min() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: Some(0),
    };
    assert_eq!(query.per_page(), 1, "per_page should be clamped to min 1");
}

#[test]
fn test_audit_query_page_clamped_to_min() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(0),
        per_page: None,
    };
    assert_eq!(query.page(), 1, "page should be clamped to min 1");
}

#[test]
fn test_audit_query_negative_page() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(-5),
        per_page: None,
    };
    assert_eq!(query.page(), 1, "Negative page should be clamped to 1");
}

#[test]
fn test_audit_query_offset_calculation() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(3),
        per_page: Some(20),
    };
    assert_eq!(query.offset(), 40, "Page 3 with per_page 20 should have offset 40");
}

#[test]
fn test_audit_query_offset_first_page() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(1),
        per_page: Some(10),
    };
    assert_eq!(query.offset(), 0, "First page should have offset 0");
}

// ---------------------------------------------------------------------------
// Date parsing
// ---------------------------------------------------------------------------

#[test]
fn test_audit_query_parse_rfc3339_from_date() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: Some("2024-06-01T00:00:00Z".to_string()),
        to_date: None,
        page: None,
        per_page: None,
    };
    let parsed = query.parsed_from_date();
    assert!(parsed.is_some(), "RFC3339 from_date should parse successfully");
}

#[test]
fn test_audit_query_parse_bare_date_from() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: Some("2024-06-01".to_string()),
        to_date: None,
        page: None,
        per_page: None,
    };
    let parsed = query.parsed_from_date();
    assert!(parsed.is_some(), "Bare YYYY-MM-DD from_date should parse successfully");
    // Bare date should resolve to start of day (00:00:00)
    let dt = parsed.unwrap();
    assert_eq!(dt.time().hour(), 0);
    assert_eq!(dt.time().minute(), 0);
}

#[test]
fn test_audit_query_parse_bare_date_to() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: Some("2024-12-31".to_string()),
        page: None,
        per_page: None,
    };
    let parsed = query.parsed_to_date();
    assert!(parsed.is_some(), "Bare YYYY-MM-DD to_date should parse successfully");
    // Bare to_date should resolve to end of day (23:59:59)
    let dt = parsed.unwrap();
    assert_eq!(dt.time().hour(), 23);
    assert_eq!(dt.time().minute(), 59);
}

#[test]
fn test_audit_query_parse_invalid_date() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: Some("not-a-date".to_string()),
        to_date: None,
        page: None,
        per_page: None,
    };
    let parsed = query.parsed_from_date();
    assert!(parsed.is_none(), "Invalid date string should return None");
}

#[test]
fn test_audit_query_no_dates() {
    let query = AuditQuery {
        actor: None,
        action: None,
        target_type: None,
        target_id: None,
        from_date: None,
        to_date: None,
        page: None,
        per_page: None,
    };
    assert!(query.parsed_from_date().is_none());
    assert!(query.parsed_to_date().is_none());
}

// ---------------------------------------------------------------------------
// Audit log entry structure
// ---------------------------------------------------------------------------

#[test]
fn test_audit_log_entry_structure() {
    use chrono::Utc;
    use uuid::Uuid;

    // Verify AuditLogEntry can be constructed and serialized
    let entry = silverscreen_backend::models::audit::AuditLogEntry {
        id: Uuid::new_v4(),
        actor: "admin_user".to_string(),
        action: "order.status_change".to_string(),
        timestamp: Utc::now(),
        ip_address: Some("192.168.1.1".to_string()),
        target_type: Some("order".to_string()),
        target_id: Some(Uuid::new_v4().to_string()),
        change_summary: Some(serde_json::json!({"from": "Paid", "to": "Processing"})),
        metadata: None,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["actor"], "admin_user");
    assert_eq!(json["action"], "order.status_change");
    assert!(json["target_type"].is_string());
    assert!(json["change_summary"].is_object());
}

#[test]
fn test_audit_log_entry_with_filters() {
    let query = AuditQuery {
        actor: Some("SYSTEM".to_string()),
        action: Some("retention.order_archived".to_string()),
        target_type: Some("order".to_string()),
        target_id: None,
        from_date: None,
        to_date: None,
        page: Some(1),
        per_page: Some(50),
    };
    assert_eq!(query.actor.as_deref(), Some("SYSTEM"));
    assert_eq!(query.action.as_deref(), Some("retention.order_archived"));
    assert_eq!(query.target_type.as_deref(), Some("order"));
    assert!(query.target_id.is_none());
    assert_eq!(query.per_page(), 50);
}
