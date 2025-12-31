//! Sync API tests.
//!
//! These tests require a running PostgreSQL database and S3 storage.
//! Set DATABASE_URL and S3_* environment variables before running.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;

use common::fixtures;
use common::TestContext;

/// Test sync pull returns empty for new device.
#[tokio::test]
#[ignore = "requires database"]
async fn test_sync_pull_initial() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .post("/api/sync/pull")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_pull_request(None))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert!(body.get("cards").is_some());
    assert!(body.get("card_states").is_some());
    assert!(body.get("settings").is_some());

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test sync upload creates new cards.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_sync_upload_new_cards() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let content = fixtures::sample_md_content(3, false);
    let file = fixtures::sync_file("rust/basics.md", &content);

    let response = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    // Should have assigned 3 new IDs
    assert_eq!(body["new_ids"].as_array().unwrap().len(), 3);

    // Should have updated file content
    assert_eq!(body["updated_files"].as_array().unwrap().len(), 1);

    // No orphaned cards on first upload
    assert_eq!(body["orphaned_cards"].as_array().unwrap().len(), 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test sync upload with existing card IDs.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_sync_upload_existing_cards() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // First upload - cards without IDs
    let content = fixtures::sample_md_content(2, false);
    let file = fixtures::sync_file("test.md", &content);
    let _ = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    // Second upload - cards with IDs
    let content_with_ids = fixtures::sample_md_content(2, true);
    let file = fixtures::sync_file("test.md", &content_with_ids);
    let response = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    // No new IDs needed (cards already have IDs)
    assert_eq!(body["new_ids"].as_array().unwrap().len(), 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test sync upload detects orphaned cards.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_sync_upload_detects_orphans() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // First upload with 3 cards
    let content = fixtures::sample_md_content(3, false);
    let file = fixtures::sync_file("test.md", &content);
    let _ = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    // Second upload with only 2 cards (1 removed)
    let content = fixtures::sample_md_content(2, true);
    let file = fixtures::sync_file("test.md", &content);
    let response = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    // Should detect orphaned card
    assert!(body["orphaned_cards"].as_array().unwrap().len() >= 1);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test confirm delete soft-deletes cards.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_sync_confirm_delete() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards
    let content = fixtures::sample_md_content(3, false);
    let file = fixtures::sync_file("test.md", &content);
    let upload_response = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    let upload_body: serde_json::Value = upload_response.json();
    let card_ids: Vec<i64> = upload_body["new_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_i64().unwrap())
        .collect();

    // Delete first card
    let response = server
        .post("/api/sync/confirm-delete")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::confirm_delete_request(vec![card_ids[0]]))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["deleted_count"].as_i64().unwrap(), 1);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test push reviews.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_push_reviews() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards first
    let content = fixtures::sample_md_content(1, false);
    let file = fixtures::sync_file("test.md", &content);
    let upload_response = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    let upload_body: serde_json::Value = upload_response.json();
    let card_id = upload_body["new_ids"][0]["id"].as_i64().unwrap();

    // Push reviews
    let review = fixtures::review_submission(card_id, 3);
    let response = server
        .post("/api/sync/push-reviews")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::push_reviews_request(vec![review]))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["synced_count"].as_i64().unwrap(), 1);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test sync endpoints require authentication.
#[tokio::test]
#[ignore = "requires database"]
async fn test_sync_requires_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server
        .post("/api/sync/pull")
        .json(&fixtures::sync_pull_request(None))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
