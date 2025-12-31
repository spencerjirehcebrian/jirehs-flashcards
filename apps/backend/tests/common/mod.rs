//! Common test utilities and fixtures for integration tests.
//!
//! This module provides shared test infrastructure including:
//! - TestContext for setting up test environment with database
//! - Helper functions for creating test data
//! - Authentication helpers
//!
//! # Requirements
//! Integration tests require:
//! - PostgreSQL database (set DATABASE_URL env var)
//! - Optionally S3/R2 for storage tests (set S3_* env vars)

pub mod fixtures;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use uuid::Uuid;

use jirehs_flashcards_backend::db::Database;
use jirehs_flashcards_backend::models::Device;
use jirehs_flashcards_backend::routes;
use jirehs_flashcards_backend::services::storage::StorageService;
use jirehs_flashcards_backend::AppState;

/// Test context containing database connection and test server.
///
/// Use this to set up integration tests with a real database connection.
/// Requires DATABASE_URL environment variable to be set.
pub struct TestContext {
    pub db: Arc<Database>,
    app: Router,
}

impl TestContext {
    /// Create a new test context.
    ///
    /// # Panics
    /// Panics if DATABASE_URL is not set or database connection fails.
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let db = Database::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        db.run_migrations()
            .await
            .expect("Failed to run migrations");

        let db = Arc::new(db);

        // For integration tests, we'll skip storage or use a mock
        // In a real test, you'd either:
        // 1. Use a mock storage service
        // 2. Connect to localstack/minio
        // 3. Use the real S3/R2 with test credentials
        let storage = StorageService::new()
            .await
            .expect("Failed to initialize storage (set S3_* env vars)");

        let state = AppState {
            db: db.clone(),
            storage: Arc::new(storage),
        };

        let app = build_test_router(state);

        Self { db, app }
    }

    /// Create a new test context with storage disabled.
    ///
    /// Use this for tests that don't need S3/R2.
    /// Note: Sync upload tests will fail without storage.
    pub async fn new_without_storage() -> Self {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

        let db = Database::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        db.run_migrations()
            .await
            .expect("Failed to run migrations");

        let db = Arc::new(db);

        // Create a dummy storage - will panic if actually used
        // Tests using this context should not call storage methods
        std::env::set_var("S3_BUCKET", "test-bucket");
        std::env::set_var("S3_ACCESS_KEY", "test-key");
        std::env::set_var("S3_SECRET_KEY", "test-secret");
        std::env::set_var("S3_ENDPOINT", "http://localhost:9000");

        let storage = StorageService::new()
            .await
            .expect("Failed to create storage config");

        let state = AppState {
            db: db.clone(),
            storage: Arc::new(storage),
        };

        let app = build_test_router(state);

        Self { db, app }
    }

    /// Get the router for use with axum-test.
    pub fn router(&self) -> Router {
        self.app.clone()
    }

    /// Create a test device and return its ID and token.
    pub async fn create_test_device(&self, name: Option<&str>) -> (Uuid, String) {
        let device = self
            .db
            .create_device(name)
            .await
            .expect("Failed to create test device");
        (device.id, device.token)
    }

    /// Get device by token.
    pub async fn get_device_by_token(&self, token: &str) -> Option<Device> {
        self.db.get_device_by_token(token).await.ok().flatten()
    }

    /// Format authorization header value.
    pub fn auth_header_value(token: &str) -> String {
        format!("Bearer {}", token)
    }

    /// Clean up test data for a device.
    ///
    /// Call this after tests to remove test data.
    pub async fn cleanup_device(&self, device_id: Uuid) {
        // Delete in order due to foreign keys
        let _ = sqlx::query("DELETE FROM reviews WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM card_states WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM cards WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM deck_settings WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM global_settings WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM md_files WHERE device_id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;

        let _ = sqlx::query("DELETE FROM devices WHERE id = $1")
            .bind(device_id)
            .execute(self.db.pool())
            .await;
    }
}

/// Build the test router with all routes.
fn build_test_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/api/device/status", get(routes::device::status))
        .route("/api/study/queue", get(routes::study::queue))
        .route("/api/study/review", post(routes::study::review))
        .route("/api/settings", get(routes::settings::get_all))
        .route("/api/settings/global", put(routes::settings::update_global))
        .route(
            "/api/settings/deck/{path}",
            put(routes::settings::update_deck),
        )
        .route(
            "/api/settings/deck/{path}",
            delete(routes::settings::delete_deck),
        )
        .route("/api/decks", get(routes::decks::list))
        .route("/api/decks/{path}/stats", get(routes::decks::stats))
        .route("/api/sync/pull", post(routes::sync::pull))
        .route("/api/sync/push-reviews", post(routes::sync::push_reviews))
        .route(
            "/api/sync/confirm-delete",
            post(routes::sync::confirm_delete),
        )
        .route("/api/sync/upload", post(routes::sync::upload))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routes::auth::auth_middleware,
        ));

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/api/device/register", post(routes::device::register))
        .merge(protected_routes)
        .with_state(state)
}
