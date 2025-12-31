//! Database models and API types

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use uuid::Uuid;

// Re-export shared types from flashcard-core
pub use flashcard_core::types::{
    Algorithm, AnswerMode, Card, CardState, CardStatus, DeckSettings, GlobalSettings,
    MatchingMode, Rating, RatingScale, RawCard,
};

// === Database Entity Types ===

/// Device registration info
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub token: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// Card stored in PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbCard {
    pub id: i64,
    pub device_id: Uuid,
    pub deck_path: String,
    pub question_text: String,
    pub answer_text: String,
    pub question_hash: String,
    pub answer_hash: String,
    pub source_file: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl DbCard {
    /// Convert to API card type
    pub fn to_api_card(&self) -> Card {
        Card {
            id: self.id,
            deck_path: self.deck_path.clone(),
            question: self.question_text.clone(),
            answer: self.answer_text.clone(),
            source_file: self.source_file.clone(),
            deleted_at: self.deleted_at,
        }
    }
}

/// Card state in PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbCardState {
    pub id: Uuid,
    pub card_id: i64,
    pub device_id: Uuid,
    pub status: String,
    pub interval_days: f64,
    pub ease_factor: f64,
    pub due_date: Option<NaiveDate>,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub lapses: i32,
    pub reviews_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbCardState {
    /// Create from flashcard-core CardState
    pub fn from_core_state(
        card_id: i64,
        device_id: Uuid,
        state: &CardState,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            card_id,
            device_id,
            status: match state.status {
                CardStatus::New => "new".to_string(),
                CardStatus::Learning => "learning".to_string(),
                CardStatus::Review => "review".to_string(),
                CardStatus::Relearning => "relearning".to_string(),
            },
            interval_days: state.interval_days,
            ease_factor: state.ease_factor,
            due_date: state.due_date.map(|d| d.date_naive()),
            stability: state.stability,
            difficulty: state.difficulty,
            lapses: state.lapses as i32,
            reviews_count: state.reviews_count as i32,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Convert to flashcard-core CardState
    pub fn to_core_state(&self) -> CardState {
        CardState {
            status: match self.status.as_str() {
                "learning" => CardStatus::Learning,
                "review" => CardStatus::Review,
                "relearning" => CardStatus::Relearning,
                _ => CardStatus::New,
            },
            interval_days: self.interval_days,
            ease_factor: self.ease_factor,
            stability: self.stability,
            difficulty: self.difficulty,
            lapses: self.lapses as u32,
            reviews_count: self.reviews_count as u32,
            due_date: self.due_date.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc()),
        }
    }

    /// Convert to CardState with card_id included (for sync responses)
    pub fn to_core_state_with_id(&self) -> CardStateWithId {
        CardStateWithId {
            card_id: self.card_id,
            state: self.to_core_state(),
        }
    }
}

/// CardState with associated card_id for sync responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardStateWithId {
    pub card_id: i64,
    #[serde(flatten)]
    pub state: CardState,
}

impl Default for DbCardState {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            card_id: 0,
            device_id: Uuid::nil(),
            status: "new".to_string(),
            interval_days: 0.0,
            ease_factor: 2.5,
            due_date: None,
            stability: None,
            difficulty: None,
            lapses: 0,
            reviews_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Review record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbReview {
    pub id: Uuid,
    pub card_id: i64,
    pub device_id: Uuid,
    pub reviewed_at: DateTime<Utc>,
    pub rating: i32,
    pub rating_scale: String,
    pub answer_mode: String,
    pub typed_answer: Option<String>,
    pub was_correct: Option<bool>,
    pub time_taken_ms: Option<i32>,
    pub interval_before: Option<f64>,
    pub interval_after: Option<f64>,
    pub ease_before: Option<f64>,
    pub ease_after: Option<f64>,
    pub algorithm: String,
    pub created_at: DateTime<Utc>,
}

/// Global settings in PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbGlobalSettings {
    pub device_id: Uuid,
    pub algorithm: String,
    pub rating_scale: String,
    pub matching_mode: String,
    pub fuzzy_threshold: f64,
    pub new_cards_per_day: i32,
    pub reviews_per_day: i32,
    pub daily_reset_hour: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbGlobalSettings {
    /// Create default settings for a device
    pub fn default_for_device(device_id: Uuid) -> Self {
        Self {
            device_id,
            algorithm: "sm2".to_string(),
            rating_scale: "4point".to_string(),
            matching_mode: "fuzzy".to_string(),
            fuzzy_threshold: 0.8,
            new_cards_per_day: 20,
            reviews_per_day: 200,
            daily_reset_hour: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Convert to API GlobalSettings
    pub fn to_api_settings(&self) -> GlobalSettings {
        GlobalSettings {
            algorithm: Algorithm::from_str(&self.algorithm).unwrap_or_default(),
            rating_scale: match self.rating_scale.as_str() {
                "2point" => RatingScale::TwoPoint,
                _ => RatingScale::FourPoint,
            },
            matching_mode: match self.matching_mode.as_str() {
                "exact" => MatchingMode::Exact,
                "case_insensitive" => MatchingMode::CaseInsensitive,
                _ => MatchingMode::Fuzzy,
            },
            fuzzy_threshold: self.fuzzy_threshold,
            new_cards_per_day: self.new_cards_per_day as u32,
            reviews_per_day: self.reviews_per_day as u32,
            daily_reset_hour: self.daily_reset_hour as u32,
        }
    }
}

/// Deck settings in PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbDeckSettings {
    pub id: Uuid,
    pub device_id: Uuid,
    pub deck_path: String,
    pub algorithm: Option<String>,
    pub rating_scale: Option<String>,
    pub matching_mode: Option<String>,
    pub fuzzy_threshold: Option<f64>,
    pub new_cards_per_day: Option<i32>,
    pub reviews_per_day: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbDeckSettings {
    /// Convert to API DeckSettings
    pub fn to_api_settings(&self) -> DeckSettings {
        DeckSettings {
            deck_path: self.deck_path.clone(),
            algorithm: self.algorithm.as_ref().and_then(|a| Algorithm::from_str(a)),
            rating_scale: self.rating_scale.as_ref().map(|s| match s.as_str() {
                "2point" => RatingScale::TwoPoint,
                _ => RatingScale::FourPoint,
            }),
            matching_mode: self.matching_mode.as_ref().map(|m| match m.as_str() {
                "exact" => MatchingMode::Exact,
                "case_insensitive" => MatchingMode::CaseInsensitive,
                _ => MatchingMode::Fuzzy,
            }),
            fuzzy_threshold: self.fuzzy_threshold,
            new_cards_per_day: self.new_cards_per_day.map(|n| n as u32),
            reviews_per_day: self.reviews_per_day.map(|n| n as u32),
        }
    }
}

/// Orphaned card info
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrphanedCard {
    pub id: i64,
    pub question_preview: String,
}

/// MD file tracking record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MdFile {
    pub id: Uuid,
    pub device_id: Uuid,
    pub file_path: String,
    pub s3_key: String,
    pub content_hash: String,
    pub uploaded_at: DateTime<Utc>,
}

/// Deck info with counts
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeckInfo {
    pub path: String,
    pub name: String,
    pub card_count: i32,
    pub new_count: i32,
    pub due_count: i32,
}

// === Effective Settings (merged global + deck) ===

/// Effective settings (global merged with deck overrides)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectiveSettings {
    pub algorithm: String,
    pub rating_scale: String,
    pub matching_mode: String,
    pub fuzzy_threshold: f64,
    pub new_cards_per_day: i32,
    pub reviews_per_day: i32,
    pub daily_reset_hour: i32,
}

impl EffectiveSettings {
    /// Merge global settings with optional deck settings
    pub fn merge(global: &DbGlobalSettings, deck: Option<&DbDeckSettings>) -> Self {
        match deck {
            Some(d) => Self {
                algorithm: d.algorithm.clone().unwrap_or_else(|| global.algorithm.clone()),
                rating_scale: d.rating_scale.clone().unwrap_or_else(|| global.rating_scale.clone()),
                matching_mode: d.matching_mode.clone().unwrap_or_else(|| global.matching_mode.clone()),
                fuzzy_threshold: d.fuzzy_threshold.unwrap_or(global.fuzzy_threshold),
                new_cards_per_day: d.new_cards_per_day.unwrap_or(global.new_cards_per_day),
                reviews_per_day: d.reviews_per_day.unwrap_or(global.reviews_per_day),
                daily_reset_hour: global.daily_reset_hour,
            },
            None => Self {
                algorithm: global.algorithm.clone(),
                rating_scale: global.rating_scale.clone(),
                matching_mode: global.matching_mode.clone(),
                fuzzy_threshold: global.fuzzy_threshold,
                new_cards_per_day: global.new_cards_per_day,
                reviews_per_day: global.reviews_per_day,
                daily_reset_hour: global.daily_reset_hour,
            },
        }
    }
}

// === API Request/Response Types ===

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceRegisterRequest {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceRegisterResponse {
    pub device_id: Uuid,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceStatusResponse {
    pub device_id: Uuid,
    pub last_seen_at: DateTime<Utc>,
}

// Sync types
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncUploadRequest {
    pub files: Vec<SyncFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncFile {
    pub path: String,
    pub content: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncUploadResponse {
    pub updated_files: Vec<UpdatedFile>,
    pub new_ids: Vec<NewIdAssignment>,
    pub orphaned_cards: Vec<OrphanedCard>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatedFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewIdAssignment {
    pub path: String,
    pub line: usize,
    pub id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmDeleteRequest {
    pub card_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmDeleteResponse {
    pub deleted_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncPullRequest {
    pub last_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncPullResponse {
    pub cards: Vec<Card>,
    pub card_states: Vec<CardStateWithId>,
    pub settings: SyncedSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncedSettings {
    pub global: GlobalSettings,
    pub decks: Vec<DeckSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushReviewsRequest {
    pub reviews: Vec<ReviewSubmission>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewSubmission {
    pub card_id: i64,
    pub reviewed_at: DateTime<Utc>,
    pub rating: i32,
    pub rating_scale: String,
    pub answer_mode: String,
    pub typed_answer: Option<String>,
    pub was_correct: Option<bool>,
    pub time_taken_ms: Option<i32>,
    pub interval_before: f64,
    pub interval_after: f64,
    pub ease_before: f64,
    pub ease_after: f64,
    pub algorithm: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushReviewsResponse {
    pub synced_count: usize,
}

// Study types
#[derive(Debug, Serialize, Deserialize)]
pub struct StudyQueueQuery {
    pub deck_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StudyQueueResponse {
    pub new_cards: Vec<Card>,
    pub review_cards: Vec<Card>,
    pub limits: StudyLimits,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StudyLimits {
    pub new_remaining: usize,
    pub review_remaining: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitReviewRequest {
    pub card_id: i64,
    pub rating: i32,
    pub rating_scale: String,
    pub answer_mode: String,
    pub typed_answer: Option<String>,
    pub time_taken_ms: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitReviewResponse {
    pub next_state: CardState,
    pub next_due: DateTime<Utc>,
}

// Deck types
#[derive(Debug, Serialize, Deserialize)]
pub struct DeckListResponse {
    pub decks: Vec<DeckInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeckStatsResponse {
    pub total_cards: usize,
    pub new_cards: usize,
    pub learning_cards: usize,
    pub review_cards: usize,
    pub average_ease: f64,
    pub average_interval: f64,
    pub retention_rate: f64,
    pub reviews_today: usize,
}

// Settings types
#[derive(Debug, Serialize, Deserialize)]
pub struct AllSettingsResponse {
    pub global: GlobalSettings,
    pub decks: HashMap<String, DeckSettings>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateGlobalSettingsRequest {
    pub algorithm: Option<String>,
    pub rating_scale: Option<String>,
    pub matching_mode: Option<String>,
    pub fuzzy_threshold: Option<f64>,
    pub new_cards_per_day: Option<i32>,
    pub reviews_per_day: Option<i32>,
    pub daily_reset_hour: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDeckSettingsRequest {
    pub algorithm: Option<String>,
    pub rating_scale: Option<String>,
    pub matching_mode: Option<String>,
    pub fuzzy_threshold: Option<f64>,
    pub new_cards_per_day: Option<i32>,
    pub reviews_per_day: Option<i32>,
}
