//! Test fixtures and factory functions for creating test data.

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use jirehs_flashcards_backend::models::{NewIdAssignment, ReviewSubmission, SyncFile};
use jirehs_flashcards_backend::services::sync::hash_content;

/// Generate sample MD content with a specified number of cards.
///
/// # Arguments
/// * `num_cards` - Number of cards to generate
/// * `with_ids` - Whether to include ID lines
pub fn sample_md_content(num_cards: usize, with_ids: bool) -> String {
    (0..num_cards)
        .map(|i| {
            if with_ids {
                format!(
                    "ID: {}\nQ: Question {}?\nA: Answer {}.\n",
                    i + 1,
                    i + 1,
                    i + 1
                )
            } else {
                format!("Q: Question {}?\nA: Answer {}.\n", i + 1, i + 1)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Create a SyncFile for upload requests.
pub fn sync_file(path: &str, content: &str) -> SyncFile {
    SyncFile {
        path: path.to_string(),
        content: content.to_string(),
        hash: hash_content(content),
    }
}

/// Create a review submission for push-reviews requests.
pub fn review_submission(card_id: i64, rating: i32) -> ReviewSubmission {
    ReviewSubmission {
        card_id,
        reviewed_at: Utc::now(),
        rating,
        rating_scale: "4point".to_string(),
        answer_mode: "flip".to_string(),
        typed_answer: None,
        was_correct: None,
        time_taken_ms: Some(2500),
        interval_before: 0.0,
        interval_after: 1.0,
        ease_before: 2.5,
        ease_after: 2.5,
        algorithm: "sm2".to_string(),
    }
}

/// Create a device register request body.
pub fn device_register_request(name: Option<&str>) -> serde_json::Value {
    match name {
        Some(n) => json!({ "name": n }),
        None => json!({}),
    }
}

/// Create a sync upload request body.
pub fn sync_upload_request(files: Vec<SyncFile>) -> serde_json::Value {
    json!({ "files": files })
}

/// Create a sync pull request body.
pub fn sync_pull_request(last_sync_at: Option<chrono::DateTime<Utc>>) -> serde_json::Value {
    json!({ "last_sync_at": last_sync_at })
}

/// Create a push reviews request body.
pub fn push_reviews_request(reviews: Vec<ReviewSubmission>) -> serde_json::Value {
    json!({ "reviews": reviews })
}

/// Create a confirm delete request body.
pub fn confirm_delete_request(card_ids: Vec<i64>) -> serde_json::Value {
    json!({ "card_ids": card_ids })
}

/// Create a submit review request body.
pub fn submit_review_request(
    card_id: i64,
    rating: i32,
    rating_scale: &str,
    answer_mode: &str,
) -> serde_json::Value {
    json!({
        "card_id": card_id,
        "rating": rating,
        "rating_scale": rating_scale,
        "answer_mode": answer_mode,
        "time_taken_ms": 2000
    })
}

/// Create an update global settings request body.
pub fn update_global_settings_request(
    algorithm: Option<&str>,
    new_cards_per_day: Option<i32>,
) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    if let Some(a) = algorithm {
        obj.insert("algorithm".to_string(), json!(a));
    }
    if let Some(n) = new_cards_per_day {
        obj.insert("new_cards_per_day".to_string(), json!(n));
    }
    serde_json::Value::Object(obj)
}

/// Create an update deck settings request body.
pub fn update_deck_settings_request(
    algorithm: Option<&str>,
    new_cards_per_day: Option<i32>,
) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    if let Some(a) = algorithm {
        obj.insert("algorithm".to_string(), json!(a));
    }
    if let Some(n) = new_cards_per_day {
        obj.insert("new_cards_per_day".to_string(), json!(n));
    }
    serde_json::Value::Object(obj)
}

/// Generate a unique test deck path to avoid collisions.
pub fn unique_deck_path(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().to_string()[..8].to_string())
}

/// Generate a unique test file path.
pub fn unique_file_path(deck: &str) -> String {
    format!("{}/cards_{}.md", deck, Uuid::new_v4().to_string()[..8].to_string())
}
