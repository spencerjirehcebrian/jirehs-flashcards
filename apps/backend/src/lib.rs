pub mod db;
pub mod error;
pub mod models;
pub mod routes;
pub mod services;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::db::Database;
use crate::services::storage::StorageService;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub storage: Arc<StorageService>,
}

pub async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    tracing::info!("Connecting to database...");
    let db = Database::connect(&database_url).await?;

    tracing::info!("Running migrations...");
    db.run_migrations().await?;

    tracing::info!("Initializing S3 storage...");
    let storage = StorageService::new().await?;

    let state = AppState {
        db: Arc::new(db),
        storage: Arc::new(storage),
    };

    // Build router with protected routes
    let protected_routes = Router::new()
        // Device routes
        .route("/api/device/status", get(routes::device::status))
        // Study routes
        .route("/api/study/queue", get(routes::study::queue))
        .route("/api/study/review", post(routes::study::review))
        // Settings routes
        .route("/api/settings", get(routes::settings::get_all))
        .route("/api/settings/global", put(routes::settings::update_global))
        .route("/api/settings/deck/{path}", put(routes::settings::update_deck))
        .route("/api/settings/deck/{path}", delete(routes::settings::delete_deck))
        // Deck routes
        .route("/api/decks", get(routes::decks::list))
        .route("/api/decks/{path}/stats", get(routes::decks::stats))
        // Sync routes
        .route("/api/sync/pull", post(routes::sync::pull))
        .route("/api/sync/push-reviews", post(routes::sync::push_reviews))
        .route("/api/sync/confirm-delete", post(routes::sync::confirm_delete))
        .route("/api/sync/upload", post(routes::sync::upload))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            routes::auth::auth_middleware,
        ));

    // Build full router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/device/register", post(routes::device::register))
        .merge(protected_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
