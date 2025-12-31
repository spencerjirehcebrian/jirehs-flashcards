//! Device registration and status API tests.
//!
//! These tests require a running PostgreSQL database.
//! Set DATABASE_URL environment variable before running.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;

use common::fixtures;
use common::TestContext;

/// Test device registration without a name.
#[tokio::test]
#[ignore = "requires database"]
async fn test_register_device_without_name() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server
        .post("/api/device/register")
        .json(&fixtures::device_register_request(None))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert!(body.get("device_id").is_some());
    assert!(body.get("token").is_some());
    assert!(body["token"].as_str().unwrap().len() > 10);

    // Cleanup
    let device_id = body["device_id"].as_str().unwrap();
    let uuid = uuid::Uuid::parse_str(device_id).unwrap();
    ctx.cleanup_device(uuid).await;
}

/// Test device registration with a name.
#[tokio::test]
#[ignore = "requires database"]
async fn test_register_device_with_name() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server
        .post("/api/device/register")
        .json(&fixtures::device_register_request(Some("My Test Device")))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.get("device_id").is_some());

    // Cleanup
    let device_id = body["device_id"].as_str().unwrap();
    let uuid = uuid::Uuid::parse_str(device_id).unwrap();
    ctx.cleanup_device(uuid).await;
}

/// Test device status endpoint requires authentication.
#[tokio::test]
#[ignore = "requires database"]
async fn test_device_status_requires_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server.get("/api/device/status").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

/// Test device status with valid token.
#[tokio::test]
#[ignore = "requires database"]
async fn test_device_status_with_valid_token() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(Some("Test Device")).await;

    let response = server
        .get("/api/device/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["device_id"].as_str().unwrap(), device_id.to_string());

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test device status with invalid token.
#[tokio::test]
#[ignore = "requires database"]
async fn test_device_status_with_invalid_token() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server
        .get("/api/device/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            "Bearer invalid-token-here",
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

/// Test device status with malformed authorization header.
#[tokio::test]
#[ignore = "requires database"]
async fn test_device_status_malformed_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    // Missing "Bearer " prefix
    let response = server
        .get("/api/device/status")
        .add_header(axum::http::header::AUTHORIZATION, "some-token")
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

/// Test that device status updates last_seen timestamp.
#[tokio::test]
#[ignore = "requires database"]
async fn test_device_status_updates_last_seen() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // First request
    let _ = server
        .get("/api/device/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    // Get device and check last_seen
    let device = ctx.get_device_by_token(&token).await.unwrap();
    let first_seen = device.last_seen_at;

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Second request
    let _ = server
        .get("/api/device/status")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    // Check last_seen was updated
    let device = ctx.get_device_by_token(&token).await.unwrap();
    assert!(device.last_seen_at >= first_seen);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}
