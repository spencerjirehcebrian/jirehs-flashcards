//! Sync endpoints

use axum::{extract::State, Extension, Json};
use chrono::Utc;
use uuid::Uuid;

use crate::error::Result;
use crate::models::*;
use crate::routes::auth::AuthenticatedDevice;
use crate::services::storage::StorageService;
use crate::services::sync::{extract_deck_path, hash_content, parse_md_content, regenerate_md_with_ids};
use crate::AppState;

/// POST /api/sync/pull
/// Pull latest state from server
pub async fn pull(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Json(payload): Json<SyncPullRequest>,
) -> Result<Json<SyncPullResponse>> {
    let cards = state
        .db
        .get_cards_since(auth.device_id, payload.last_sync_at)
        .await?;
    let card_states = state
        .db
        .get_card_states_since(auth.device_id, payload.last_sync_at)
        .await?;
    let global = state.db.get_global_settings(auth.device_id).await?;
    let decks = state.db.get_all_deck_settings(auth.device_id).await?;

    Ok(Json(SyncPullResponse {
        cards: cards.into_iter().map(|c| c.to_api_card()).collect(),
        card_states: card_states.into_iter().map(|s| s.to_core_state_with_id()).collect(),
        settings: SyncedSettings {
            global: global.to_api_settings(),
            decks: decks.into_iter().map(|d| d.to_api_settings()).collect(),
        },
    }))
}

/// POST /api/sync/push-reviews
/// Push pending reviews from client
pub async fn push_reviews(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Json(payload): Json<PushReviewsRequest>,
) -> Result<Json<PushReviewsResponse>> {
    let db_reviews: Vec<DbReview> = payload
        .reviews
        .into_iter()
        .map(|r| DbReview {
            id: Uuid::new_v4(),
            card_id: r.card_id,
            device_id: auth.device_id,
            reviewed_at: r.reviewed_at,
            rating: r.rating,
            rating_scale: r.rating_scale,
            answer_mode: r.answer_mode,
            typed_answer: r.typed_answer,
            was_correct: r.was_correct,
            time_taken_ms: r.time_taken_ms,
            interval_before: Some(r.interval_before),
            interval_after: Some(r.interval_after),
            ease_before: Some(r.ease_before),
            ease_after: Some(r.ease_after),
            algorithm: r.algorithm,
            created_at: chrono::Utc::now(),
        })
        .collect();

    let count = state.db.insert_reviews(&db_reviews).await?;

    Ok(Json(PushReviewsResponse { synced_count: count }))
}

/// POST /api/sync/confirm-delete
/// Confirm deletion of orphaned cards
pub async fn confirm_delete(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedDevice>,
    Json(payload): Json<ConfirmDeleteRequest>,
) -> Result<Json<ConfirmDeleteResponse>> {
    let count = state.db.soft_delete_cards(&payload.card_ids).await?;
    Ok(Json(ConfirmDeleteResponse { deleted_count: count }))
}

/// POST /api/sync/upload
/// Upload MD files, parse cards, generate IDs, and store in S3
pub async fn upload(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedDevice>,
    Json(payload): Json<SyncUploadRequest>,
) -> Result<Json<SyncUploadResponse>> {
    let mut updated_files = Vec::new();
    let mut all_new_ids = Vec::new();
    let mut all_card_ids = Vec::new();

    for file in &payload.files {
        // 1. Parse MD content to extract cards
        let parsed = parse_md_content(&file.content)?;

        // 2. For each card, generate ID if needed and upsert to database
        let mut file_new_ids = Vec::new();
        let deck_path = extract_deck_path(&file.path);
        let deck_path = if deck_path.is_empty() {
            file.path.trim_end_matches(".md").to_string()
        } else {
            deck_path
        };

        for card in &parsed.cards {
            let card_id = match card.id {
                Some(id) => id,
                None => {
                    // Generate new ID
                    let new_id = state.db.get_next_card_id().await?;
                    file_new_ids.push(NewIdAssignment {
                        path: file.path.clone(),
                        line: card.line,
                        id: new_id,
                    });
                    new_id
                }
            };

            all_card_ids.push(card_id);

            // Upsert card to database
            let db_card = DbCard {
                id: card_id,
                device_id: auth.device_id,
                deck_path: deck_path.clone(),
                question_text: card.question.clone(),
                answer_text: card.answer.clone(),
                question_hash: hash_content(&card.question),
                answer_hash: hash_content(&card.answer),
                source_file: file.path.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deleted_at: None,
            };
            state.db.upsert_card(&db_card).await?;
        }

        // 3. If new IDs were assigned, regenerate content
        let updated_content = if !file_new_ids.is_empty() {
            let new_content = regenerate_md_with_ids(&file.content, &file_new_ids);
            updated_files.push(UpdatedFile {
                path: file.path.clone(),
                content: new_content.clone(),
            });
            new_content
        } else {
            file.content.clone()
        };

        // 4. Upload to S3
        let s3_key = StorageService::make_key(&auth.device_id.to_string(), &file.path);
        state
            .storage
            .upload_file(&s3_key, updated_content.as_bytes(), Some("text/markdown"))
            .await
            .map_err(|e| crate::error::ApiError::Internal(e.to_string()))?;

        // 5. Update md_files tracking table
        let content_hash = hash_content(&updated_content);
        state
            .db
            .upsert_md_file(auth.device_id, &file.path, &s3_key, &content_hash)
            .await?;

        all_new_ids.extend(file_new_ids);
    }

    // 6. Detect orphaned cards (cards in DB but not in any uploaded file)
    let orphaned_cards = state
        .db
        .get_orphaned_cards(auth.device_id, &all_card_ids)
        .await?;

    Ok(Json(SyncUploadResponse {
        updated_files,
        new_ids: all_new_ids,
        orphaned_cards,
    }))
}
