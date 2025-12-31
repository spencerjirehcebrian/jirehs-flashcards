//! Decks API tests.
//!
//! These tests require a running PostgreSQL database and S3 storage.
//! Set DATABASE_URL and S3_* environment variables before running.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;

use common::fixtures;
use common::TestContext;

/// Test list decks is empty for new device.
#[tokio::test]
#[ignore = "requires database"]
async fn test_list_decks_empty() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .get("/api/decks")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["decks"].as_array().unwrap().is_empty());

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test list decks after upload.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_list_decks_after_upload() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards to create decks
    let rust_content = fixtures::sample_md_content(5, false);
    let python_content = fixtures::sample_md_content(3, false);

    let _ = server
        .post("/api/sync/upload")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::sync_upload_request(vec![
            fixtures::sync_file("rust/basics.md", &rust_content),
            fixtures::sync_file("python/advanced.md", &python_content),
        ]))
        .await;

    let response = server
        .get("/api/decks")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let decks = body["decks"].as_array().unwrap();

    assert_eq!(decks.len(), 2);

    // Find rust deck and verify counts
    let rust_deck = decks.iter().find(|d| d["path"] == "rust").unwrap();
    assert_eq!(rust_deck["card_count"], 5);
    assert_eq!(rust_deck["new_count"], 5);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test deck stats.
#[tokio::test]
#[ignore = "requires database and storage"]
async fn test_deck_stats() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Upload cards
    let content = fixtures::sample_md_content(10, false);
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
        .get("/api/decks/test/stats")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["total_cards"], 10);
    assert_eq!(body["new_cards"], 10);
    assert_eq!(body["learning_cards"], 0);
    assert_eq!(body["review_cards"], 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test decks endpoint requires authentication.
#[tokio::test]
#[ignore = "requires database"]
async fn test_decks_requires_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server.get("/api/decks").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

/// Test deck stats for non-existent deck.
#[tokio::test]
#[ignore = "requires database"]
async fn test_deck_stats_not_found() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .get("/api/decks/nonexistent/stats")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    // Should return stats with zeros for non-existent deck
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total_cards"], 0);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}
