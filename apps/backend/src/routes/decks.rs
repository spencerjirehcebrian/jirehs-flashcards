//! Deck endpoints

use axum::{
    extract::{Path, State},
    Extension, Json,
};

use crate::error::Result;
use crate::models::*;
use crate::routes::auth::AuthenticatedDevice;
use crate::AppState;

/// GET /api/decks
pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
) -> Result<Json<DeckListResponse>> {
    let decks = state.db.get_all_decks(auth.device_id).await?;
    Ok(Json(DeckListResponse { decks }))
}

/// GET /api/decks/:path/stats
pub async fn stats(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Path(deck_path): Path<String>,
) -> Result<Json<DeckStatsResponse>> {
    let stats = state.db.get_deck_stats(auth.device_id, &deck_path).await?;
    Ok(Json(stats))
}
