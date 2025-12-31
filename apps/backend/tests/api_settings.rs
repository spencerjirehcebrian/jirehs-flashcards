//! Settings API tests.
//!
//! These tests require a running PostgreSQL database.
//! Set DATABASE_URL environment variable before running.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;

use common::fixtures;
use common::TestContext;

/// Test getting all settings returns defaults.
#[tokio::test]
#[ignore = "requires database"]
async fn test_get_all_settings_default() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .get("/api/settings")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    // Check default global settings
    assert_eq!(body["global"]["algorithm"], "sm2");
    assert_eq!(body["global"]["rating_scale"], "4point");
    assert_eq!(body["global"]["new_cards_per_day"], 20);
    assert!(body["decks"].as_object().unwrap().is_empty());

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test updating global settings.
#[tokio::test]
#[ignore = "requires database"]
async fn test_update_global_settings() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .put("/api/settings/global")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::update_global_settings_request(
            Some("fsrs"),
            Some(50),
        ))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["algorithm"], "fsrs");
    assert_eq!(body["new_cards_per_day"], 50);
    // Other fields should remain default
    assert_eq!(body["rating_scale"], "4point");

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test updating deck settings.
#[tokio::test]
#[ignore = "requires database"]
async fn test_update_deck_settings() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    let response = server
        .put("/api/settings/deck/rust%2Fbasics")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::update_deck_settings_request(Some("fsrs"), Some(10)))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert_eq!(body["deck_path"], "rust/basics");
    assert_eq!(body["algorithm"], "fsrs");

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test deleting deck settings.
#[tokio::test]
#[ignore = "requires database"]
async fn test_delete_deck_settings() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Create deck settings first
    let _ = server
        .put("/api/settings/deck/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::update_deck_settings_request(Some("fsrs"), None))
        .await;

    // Delete
    let response = server
        .delete("/api/settings/deck/test")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["deleted"].as_bool().unwrap());

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test deck settings show in all settings response.
#[tokio::test]
#[ignore = "requires database"]
async fn test_deck_settings_show_in_all_settings() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();
    let (device_id, token) = ctx.create_test_device(None).await;

    // Create deck settings
    let _ = server
        .put("/api/settings/deck/my-deck")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .json(&fixtures::update_deck_settings_request(None, Some(30)))
        .await;

    // Get all settings
    let response = server
        .get("/api/settings")
        .add_header(
            axum::http::header::AUTHORIZATION,
            TestContext::auth_header_value(&token),
        )
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();

    assert!(body["decks"].get("my-deck").is_some());
    assert_eq!(body["decks"]["my-deck"]["new_cards_per_day"], 30);

    // Cleanup
    ctx.cleanup_device(device_id).await;
}

/// Test settings endpoint requires authentication.
#[tokio::test]
#[ignore = "requires database"]
async fn test_settings_requires_auth() {
    let ctx = TestContext::new().await;
    let server = TestServer::new(ctx.router()).unwrap();

    let response = server.get("/api/settings").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}
