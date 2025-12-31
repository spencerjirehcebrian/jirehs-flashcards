//! Study endpoints

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::models::*;
use crate::routes::auth::AuthenticatedDevice;
use crate::AppState;
use flashcard_core::algorithm::{get_algorithm, SchedulingResult};

/// GET /api/study/queue
pub async fn queue(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Query(query): Query<StudyQueueQuery>,
) -> Result<Json<StudyQueueResponse>> {
    let settings = state
        .db
        .get_effective_settings(auth.device_id, query.deck_path.as_deref())
        .await?;

    let new_limit = settings.new_cards_per_day;
    let review_limit = settings.reviews_per_day;

    let new_cards = state
        .db
        .get_new_cards(auth.device_id, query.deck_path.as_deref(), new_limit)
        .await?;
    let review_cards = state
        .db
        .get_due_cards(auth.device_id, query.deck_path.as_deref(), review_limit)
        .await?;

    let new_count = new_cards.len();
    let review_count = review_cards.len();

    Ok(Json(StudyQueueResponse {
        new_cards: new_cards.into_iter().map(|c| c.to_api_card()).collect(),
        review_cards: review_cards.into_iter().map(|c| c.to_api_card()).collect(),
        limits: StudyLimits {
            new_remaining: (new_limit as usize).saturating_sub(new_count),
            review_remaining: (review_limit as usize).saturating_sub(review_count),
        },
    }))
}

/// POST /api/study/review
pub async fn review(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Json(payload): Json<SubmitReviewRequest>,
) -> Result<Json<SubmitReviewResponse>> {
    // Get the card
    let card = state
        .db
        .get_card(payload.card_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Card not found".to_string()))?;

    // Get current card state (or default if none)
    let current_state = state
        .db
        .get_card_state(payload.card_id, auth.device_id)
        .await?
        .map(|s| s.to_core_state())
        .unwrap_or_default();

    // Get effective settings for the algorithm
    let settings = state
        .db
        .get_effective_settings(auth.device_id, Some(&card.deck_path))
        .await?;

    // Get the algorithm
    let algorithm = get_algorithm(&settings.algorithm)
        .ok_or_else(|| ApiError::BadRequest(format!("Unknown algorithm: {}", settings.algorithm)))?;

    // Convert rating
    let rating = Rating::from_value(payload.rating as u8).unwrap_or(Rating::Good);

    // Calculate next state
    let now = Utc::now();
    let result: SchedulingResult = algorithm.schedule(&current_state, rating, now);

    // Convert to DB state and save
    let db_state = DbCardState::from_core_state(payload.card_id, auth.device_id, &result.new_state);
    state
        .db
        .upsert_card_state(payload.card_id, auth.device_id, &db_state)
        .await?;

    // Record the review
    let review = DbReview {
        id: Uuid::new_v4(),
        card_id: payload.card_id,
        device_id: auth.device_id,
        reviewed_at: now,
        rating: payload.rating,
        rating_scale: payload.rating_scale,
        answer_mode: payload.answer_mode,
        typed_answer: payload.typed_answer,
        was_correct: None,
        time_taken_ms: payload.time_taken_ms,
        interval_before: Some(current_state.interval_days),
        interval_after: Some(result.new_state.interval_days),
        ease_before: Some(current_state.ease_factor),
        ease_after: Some(result.new_state.ease_factor),
        algorithm: settings.algorithm,
        created_at: now,
    };
    state.db.insert_review(&review).await?;

    Ok(Json(SubmitReviewResponse {
        next_state: result.new_state,
        next_due: result.next_due,
    }))
}
