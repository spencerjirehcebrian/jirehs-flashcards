//! Study API tests.
//!
//! These tests require a running PostgreSQL database and S3 storage.
//! Set DATABASE_URL and S3_* environment variables before running.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;

use common::fixtures;
use common::TestContext;

/// Test study queue is empty for new device.
#[tokio::test]
#[ignore = "requires database"]
async fn test_study_queue_empty() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .get("/api/study/queue")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["new_cards"].as_array().unwrap().len(), 0);
    assert_eq!(body["review_cards"].as_array().unwrap().len(), 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test study queue with new cards after upload.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_study_queue_with_new_cards() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards
    let content = fixtures::sample_md_content(5, false);
    let file = fixtures::sync_file("test/cards.md", &content);
    let _ = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![file]))
        .await;

    let response = server
        .get("/api/study/queue")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["new_cards"].as_array().unwrap().len(), 5);
    assert!(body["limits"]["new_remaining"].as_i64().unwrap() > 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test study queue respects deck filter.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_study_queue_respects_deck_filter() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards to different decks
    let rust_content = fixtures::sample_md_content(3, false);
    let python_content = fixtures::sample_md_content(2, false);

    let _ = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![
            fixtures::sync_file("rust/basics.md", &rust_content),
            fixtures::sync_file("python/basics.md", &python_content),
        ]))
        .await;

    // Filter by rust deck
    let response = server
        .get("/api/study/queue?deck_path=rust")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["new_cards"].as_array().unwrap().len(), 3);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test submitting a review for non-existent card returns not found.
#[tokio::test]
#[ignore = "requires database"]
async fn test_submit_review_not_found() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .post("/api/study/review")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::submit_review_request(99999, 3, "4point", "flip"))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test study endpoint requires authentication.
#[tokio::test]
#[ignore = "requires database"]
async fn test_study_queue_requires_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server.get("/api/study/queue").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
