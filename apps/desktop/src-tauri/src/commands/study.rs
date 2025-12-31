//! Study session Tauri commands.

use crate::db::{
    CardRepository, PendingReview, SettingsRepository, StateRepository, SyncRepository,
};
use crate::state::AppState;
use chrono::Utc;
use flashcard_core::algorithm::{get_algorithm, SpacedRepetitionAlgorithm};
use flashcard_core::matching::{compare_answers, word_diff, DiffSegment};
use flashcard_core::types::{Card, CardState, Rating, StudyQueue};
use tauri::State;

use super::deck::CommandError;

#[derive(Debug, serde::Deserialize)]
pub struct ReviewRequest {
    pub card_id: i64,
    pub rating: u8,
    pub rating_scale: String,
    pub answer_mode: String,
    #[serde(default)]
    pub typed_answer: Option<String>,
    #[serde(default)]
    pub time_taken_ms: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReviewResponse {
    pub new_state: CardState,
    pub next_due: String,
}

/// Get the study queue for a deck (or all decks).
#[tauri::command]
pub async fn get_study_queue(
    deck_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<StudyQueue, CommandError> {
    let repo = state.repository.lock().expect("repository lock");

    // Get limits from effective settings
    let settings = repo.get_effective_settings(deck_path.as_deref())?;
    let new_limit = settings.new_cards_per_day as usize;
    let review_limit = settings.reviews_per_day as usize;
    let daily_reset_hour = settings.daily_reset_hour;

    let new_cards = repo.get_new_cards(deck_path.as_deref(), new_limit)?;
    let review_cards = repo.get_due_cards(deck_path.as_deref(), review_limit, daily_reset_hour)?;

    Ok(StudyQueue {
        new_remaining: new_limit.saturating_sub(new_cards.len()),
        review_remaining: review_limit.saturating_sub(review_cards.len()),
        new_cards,
        review_cards,
    })
}

/// Submit a review for a card.
#[tauri::command]
pub async fn submit_review(
    request: ReviewRequest,
    state: State<'_, AppState>,
) -> Result<ReviewResponse, CommandError> {
    let repo = state.repository.lock().expect("repository lock");

    // Get card to find deck path
    let card = repo.get_card(request.card_id)?;
    let deck_path = card.map(|c| c.deck_path);

    // Get effective settings for the algorithm
    let settings = repo.get_effective_settings(deck_path.as_deref())?;

    // Get current card state
    let card_state = repo
        .get_card_state(request.card_id)?
        .unwrap_or_default();

    // Get algorithm from settings
    let algorithm_name = settings.algorithm.as_str();
    let algorithm = get_algorithm(algorithm_name).expect("algorithm should exist");

    // Convert rating
    let rating = Rating::from_value(request.rating).unwrap_or(Rating::Good);

    // Calculate next state
    let now = Utc::now();
    let result = algorithm.schedule(&card_state, rating, now);

    // Save new state
    repo.save_card_state(request.card_id, &result.new_state)?;

    // Save to pending_reviews for sync
    let pending_review = PendingReview {
        id: 0, // Will be assigned by database
        card_id: request.card_id,
        reviewed_at: now.to_rfc3339(),
        rating: request.rating as i32,
        rating_scale: request.rating_scale.clone(),
        answer_mode: request.answer_mode.clone(),
        typed_answer: request.typed_answer.clone(),
        was_correct: None, // Could be computed if needed
        time_taken_ms: request.time_taken_ms.map(|t| t as i32),
        interval_before: card_state.interval_days,
        interval_after: result.new_state.interval_days,
        ease_before: card_state.ease_factor,
        ease_after: result.new_state.ease_factor,
        algorithm: algorithm_name.to_string(),
    };
    repo.insert_pending_review(&pending_review)?;
    repo.increment_pending_changes()?;

    Ok(ReviewResponse {
        new_state: result.new_state,
        next_due: result.next_due.to_rfc3339(),
    })
}

/// Get a single card by ID.
#[tauri::command]
pub async fn get_card(
    card_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<Card>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_card(card_id).map_err(Into::into)
}

/// Get card state by ID.
#[tauri::command]
pub async fn get_card_state(
    card_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<CardState>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_card_state(card_id).map_err(Into::into)
}

/// Response from typed answer comparison.
#[derive(Debug, serde::Serialize)]
pub struct CompareAnswerResponse {
    pub is_correct: bool,
    pub similarity: f64,
    pub matching_mode: String,
    pub typed_normalized: String,
    pub correct_normalized: String,
    pub diff: Vec<DiffSegment>,
}

/// Compare a typed answer to the correct answer.
#[tauri::command]
pub async fn compare_typed_answer(
    typed_answer: String,
    correct_answer: String,
    deck_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<CompareAnswerResponse, CommandError> {
    let repo = state.repository.lock().expect("repository lock");

    // Get effective settings for matching mode
    let settings = repo.get_effective_settings(deck_path.as_deref())?;

    // Compare answers
    let result = compare_answers(
        &typed_answer,
        &correct_answer,
        settings.matching_mode,
        settings.fuzzy_threshold,
    );

    // Generate diff for display
    let diff = word_diff(&result.typed_normalized, &result.correct_normalized);

    let matching_mode_str = match settings.matching_mode {
        flashcard_core::types::MatchingMode::Exact => "exact",
        flashcard_core::types::MatchingMode::CaseInsensitive => "case_insensitive",
        flashcard_core::types::MatchingMode::Fuzzy => "fuzzy",
    };

    Ok(CompareAnswerResponse {
        is_correct: result.is_correct,
        similarity: result.similarity,
        matching_mode: matching_mode_str.to_string(),
        typed_normalized: result.typed_normalized,
        correct_normalized: result.correct_normalized,
        diff,
    })
}
