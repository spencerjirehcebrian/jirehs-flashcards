//! Settings endpoints

use axum::{
    extract::{Path, State},
    Extension, Json,
};
use uuid::Uuid;

use crate::error::Result;
use crate::models::*;
use crate::routes::auth::AuthenticatedDevice;
use crate::AppState;

/// GET /api/settings
pub async fn get_all(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
) -> Result<Json<AllSettingsResponse>> {
    let global = state.db.get_global_settings(auth.device_id).await?;
    let deck_list = state.db.get_all_deck_settings(auth.device_id).await?;

    let decks = deck_list
        .into_iter()
        .map(|s| (s.deck_path.clone(), s.to_api_settings()))
        .collect();

    Ok(Json(AllSettingsResponse {
        global: global.to_api_settings(),
        decks,
    }))
}

/// PUT /api/settings/global
pub async fn update_global(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Json(request): Json<UpdateGlobalSettingsRequest>,
) -> Result<Json<GlobalSettings>> {
    // Get current settings
    let mut current = state.db.get_global_settings(auth.device_id).await?;

    // Apply updates
    if let Some(algorithm) = request.algorithm {
        current.algorithm = algorithm;
    }
    if let Some(rating_scale) = request.rating_scale {
        current.rating_scale = rating_scale;
    }
    if let Some(matching_mode) = request.matching_mode {
        current.matching_mode = matching_mode;
    }
    if let Some(fuzzy_threshold) = request.fuzzy_threshold {
        current.fuzzy_threshold = fuzzy_threshold;
    }
    if let Some(new_cards_per_day) = request.new_cards_per_day {
        current.new_cards_per_day = new_cards_per_day;
    }
    if let Some(reviews_per_day) = request.reviews_per_day {
        current.reviews_per_day = reviews_per_day;
    }
    if let Some(daily_reset_hour) = request.daily_reset_hour {
        current.daily_reset_hour = daily_reset_hour;
    }

    // Save
    state
        .db
        .upsert_global_settings(auth.device_id, &current)
        .await?;

    Ok(Json(current.to_api_settings()))
}

/// PUT /api/settings/deck/:path
pub async fn update_deck(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Path(deck_path): Path<String>,
    Json(request): Json<UpdateDeckSettingsRequest>,
) -> Result<Json<DeckSettings>> {
    // Get current settings or create new
    let mut current = state
        .db
        .get_deck_settings(auth.device_id, &deck_path)
        .await?
        .unwrap_or_else(|| DbDeckSettings {
            id: Uuid::new_v4(),
            device_id: auth.device_id,
            deck_path: deck_path.clone(),
            algorithm: None,
            rating_scale: None,
            matching_mode: None,
            fuzzy_threshold: None,
            new_cards_per_day: None,
            reviews_per_day: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });

    // Apply updates (None values clear the override)
    current.algorithm = request.algorithm;
    current.rating_scale = request.rating_scale;
    current.matching_mode = request.matching_mode;
    current.fuzzy_threshold = request.fuzzy_threshold;
    current.new_cards_per_day = request.new_cards_per_day;
    current.reviews_per_day = request.reviews_per_day;

    // Save
    state
        .db
        .upsert_deck_settings(auth.device_id, &current)
        .await?;

    Ok(Json(current.to_api_settings()))
}

/// DELETE /api/settings/deck/:path
pub async fn delete_deck(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Path(deck_path): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let deleted = state
        .db
        .delete_deck_settings(auth.device_id, &deck_path)
        .await?;

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
