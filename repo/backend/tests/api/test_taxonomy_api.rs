use actix_web::test;
use serde_json::json;

use super::common;

// ---------------------------------------------------------------------------
// GET /api/taxonomy/tags — create a tag, list all, assert tag appears
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_list_tags() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let tag_name = format!("Tag_{}", uuid::Uuid::new_v4());

    // Create a tag
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/tags")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &tag_name }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let tag_body: serde_json::Value = test::read_body_json(resp).await;
    let tag_id = tag_body["id"].as_str().unwrap().to_string();

    // List all tags
    let req = test::TestRequest::get()
        .uri("/api/taxonomy/tags")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let tags = body.as_array().unwrap();
    assert!(
        tags.iter().any(|t| t["id"].as_str() == Some(&tag_id)),
        "Created tag must appear in tag list"
    );
}

// ---------------------------------------------------------------------------
// GET /api/taxonomy/topics — create a topic, list all, assert topic appears
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_list_topics() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let topic_name = format!("Topic_{}", uuid::Uuid::new_v4());
    let topic_slug = format!("topic-{}", uuid::Uuid::new_v4());

    // Create a topic
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &topic_name, "slug": &topic_slug }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let topic_body: serde_json::Value = test::read_body_json(resp).await;
    let topic_id = topic_body["id"].as_str().unwrap().to_string();

    // List all topics
    let req = test::TestRequest::get()
        .uri("/api/taxonomy/topics")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let topics = body.as_array().unwrap();

    // Topic may be nested in a tree, so we need to search recursively
    fn find_topic_in_tree(items: &[serde_json::Value], target_id: &str) -> bool {
        for item in items {
            if item["id"].as_str() == Some(target_id) {
                return true;
            }
            if let Some(children) = item["children"].as_array() {
                if find_topic_in_tree(children, target_id) {
                    return true;
                }
            }
        }
        false
    }

    assert!(
        find_topic_in_tree(topics, &topic_id),
        "Created topic must appear in topic list"
    );
}

// ---------------------------------------------------------------------------
// DELETE /api/taxonomy/tags/:id — create a tag, delete it, verify absence
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_delete_tag() {
    let app = common::create_test_app().await;
    let token = common::admin_token();
    let tag_name = format!("DelTag_{}", uuid::Uuid::new_v4());

    // Create tag
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/tags")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &tag_name }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let tag_body: serde_json::Value = test::read_body_json(resp).await;
    let tag_id = tag_body["id"].as_str().unwrap().to_string();

    // Delete tag
    let req = test::TestRequest::delete()
        .uri(&format!("/api/taxonomy/tags/{}", tag_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Verify tag no longer in list
    let req = test::TestRequest::get()
        .uri("/api/taxonomy/tags")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let tags = body.as_array().unwrap();
    assert!(
        !tags.iter().any(|t| t["id"].as_str() == Some(&tag_id)),
        "Deleted tag must not appear in tag list"
    );
}

// ---------------------------------------------------------------------------
// DELETE /api/taxonomy/topics/:id — create a topic, delete it, verify absence
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_delete_topic() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    // Create a replacement topic first (deletion requires replacement_id)
    let replacement_name = format!("Replacement_{}", uuid::Uuid::new_v4());
    let replacement_slug = format!("replacement-{}", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &replacement_name, "slug": &replacement_slug }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let repl_body: serde_json::Value = test::read_body_json(resp).await;
    let replacement_id = repl_body["id"].as_str().unwrap().to_string();

    // Create topic to delete
    let topic_name = format!("DelTopic_{}", uuid::Uuid::new_v4());
    let topic_slug = format!("del-topic-{}", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &topic_name, "slug": &topic_slug }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let topic_body: serde_json::Value = test::read_body_json(resp).await;
    let topic_id = topic_body["id"].as_str().unwrap().to_string();

    // Delete topic (with replacement_id query param)
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/taxonomy/topics/{}?replacement_id={}",
            topic_id, replacement_id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    // Verify topic no longer in list
    let req = test::TestRequest::get()
        .uri("/api/taxonomy/topics")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let topics = body.as_array().unwrap();

    fn find_topic(items: &[serde_json::Value], target_id: &str) -> bool {
        for item in items {
            if item["id"].as_str() == Some(target_id) {
                return true;
            }
            if let Some(children) = item["children"].as_array() {
                if find_topic(children, target_id) {
                    return true;
                }
            }
        }
        false
    }

    assert!(
        !find_topic(topics, &topic_id),
        "Deleted topic must not appear in topic list"
    );
}

// ---------------------------------------------------------------------------
// PUT /api/taxonomy/topics/:id — create a topic, update it, verify fields
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_update_topic() {
    let app = common::create_test_app().await;
    let token = common::admin_token();

    // Create topic
    let topic_name = format!("UpdTopic_{}", uuid::Uuid::new_v4());
    let topic_slug = format!("upd-topic-{}", uuid::Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/taxonomy/topics")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &topic_name, "slug": &topic_slug }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let topic_body: serde_json::Value = test::read_body_json(resp).await;
    let topic_id = topic_body["id"].as_str().unwrap();

    // Update topic name
    let new_name = format!("UpdatedTopic_{}", uuid::Uuid::new_v4());
    let req = test::TestRequest::put()
        .uri(&format!("/api/taxonomy/topics/{}", topic_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({ "name": &new_name }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let upd_body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(upd_body["name"].as_str().unwrap(), new_name);
    assert_eq!(upd_body["id"].as_str().unwrap(), topic_id);
}
