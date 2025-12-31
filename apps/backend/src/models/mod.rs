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

#[cfg(test)]
mod tests {
    use super::*;

    // === DbCard tests ===

    #[test]
    fn test_db_card_to_api_card() {
        let db_card = DbCard {
            id: 42,
            device_id: Uuid::new_v4(),
            deck_path: "rust/basics".to_string(),
            question_text: "What is Rust?".to_string(),
            answer_text: "A systems language.".to_string(),
            question_hash: "abc123".to_string(),
            answer_hash: "def456".to_string(),
            source_file: "rust/basics.md".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };

        let api_card = db_card.to_api_card();

        assert_eq!(api_card.id, 42);
        assert_eq!(api_card.deck_path, "rust/basics");
        assert_eq!(api_card.question, "What is Rust?");
        assert_eq!(api_card.answer, "A systems language.");
        assert_eq!(api_card.source_file, "rust/basics.md");
        assert!(api_card.deleted_at.is_none());
    }

    #[test]
    fn test_db_card_to_api_card_with_deleted_at() {
        let deleted_time = Utc::now();
        let db_card = DbCard {
            id: 1,
            device_id: Uuid::new_v4(),
            deck_path: "test".to_string(),
            question_text: "Q".to_string(),
            answer_text: "A".to_string(),
            question_hash: "hash".to_string(),
            answer_hash: "hash".to_string(),
            source_file: "test.md".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: Some(deleted_time),
        };

        let api_card = db_card.to_api_card();
        assert!(api_card.deleted_at.is_some());
    }

    // === DbCardState tests ===

    #[test]
    fn test_db_card_state_from_core_state_new() {
        let core_state = CardState {
            status: CardStatus::New,
            interval_days: 0.0,
            ease_factor: 2.5,
            stability: None,
            difficulty: None,
            lapses: 0,
            reviews_count: 0,
            due_date: None,
        };

        let device_id = Uuid::new_v4();
        let db_state = DbCardState::from_core_state(42, device_id, &core_state);

        assert_eq!(db_state.card_id, 42);
        assert_eq!(db_state.device_id, device_id);
        assert_eq!(db_state.status, "new");
        assert_eq!(db_state.interval_days, 0.0);
        assert_eq!(db_state.ease_factor, 2.5);
        assert!(db_state.due_date.is_none());
    }

    #[test]
    fn test_db_card_state_from_core_state_learning() {
        let core_state = CardState {
            status: CardStatus::Learning,
            interval_days: 0.5,
            ease_factor: 2.3,
            stability: Some(1.5),
            difficulty: Some(0.5),
            lapses: 1,
            reviews_count: 3,
            due_date: Some(Utc::now()),
        };

        let db_state = DbCardState::from_core_state(1, Uuid::new_v4(), &core_state);

        assert_eq!(db_state.status, "learning");
        assert_eq!(db_state.stability, Some(1.5));
        assert_eq!(db_state.difficulty, Some(0.5));
        assert_eq!(db_state.lapses, 1);
        assert_eq!(db_state.reviews_count, 3);
    }

    #[test]
    fn test_db_card_state_to_core_state_roundtrip() {
        let original = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            ease_factor: 2.8,
            stability: Some(15.0),
            difficulty: Some(0.3),
            lapses: 2,
            reviews_count: 20,
            due_date: None,
        };

        let db_state = DbCardState::from_core_state(1, Uuid::new_v4(), &original);
        let roundtrip = db_state.to_core_state();

        assert_eq!(roundtrip.status, CardStatus::Review);
        assert_eq!(roundtrip.interval_days, 10.0);
        assert_eq!(roundtrip.ease_factor, 2.8);
        assert_eq!(roundtrip.stability, Some(15.0));
        assert_eq!(roundtrip.lapses, 2);
        assert_eq!(roundtrip.reviews_count, 20);
    }

    #[test]
    fn test_db_card_state_status_parsing() {
        let cases = [
            ("new", CardStatus::New),
            ("learning", CardStatus::Learning),
            ("review", CardStatus::Review),
            ("relearning", CardStatus::Relearning),
            ("unknown", CardStatus::New),
            ("", CardStatus::New),
        ];

        for (status_str, expected) in cases {
            let db_state = DbCardState {
                status: status_str.to_string(),
                ..Default::default()
            };
            assert_eq!(db_state.to_core_state().status, expected);
        }
    }

    #[test]
    fn test_db_card_state_with_id() {
        let db_state = DbCardState {
            card_id: 123,
            ..Default::default()
        };
        let with_id = db_state.to_core_state_with_id();
        assert_eq!(with_id.card_id, 123);
    }

    // === DbGlobalSettings tests ===

    #[test]
    fn test_db_global_settings_default() {
        let device_id = Uuid::new_v4();
        let settings = DbGlobalSettings::default_for_device(device_id);

        assert_eq!(settings.device_id, device_id);
        assert_eq!(settings.algorithm, "sm2");
        assert_eq!(settings.rating_scale, "4point");
        assert_eq!(settings.matching_mode, "fuzzy");
        assert_eq!(settings.fuzzy_threshold, 0.8);
        assert_eq!(settings.new_cards_per_day, 20);
        assert_eq!(settings.reviews_per_day, 200);
        assert_eq!(settings.daily_reset_hour, 0);
    }

    #[test]
    fn test_db_global_settings_to_api() {
        let settings = DbGlobalSettings {
            device_id: Uuid::new_v4(),
            algorithm: "fsrs".to_string(),
            rating_scale: "2point".to_string(),
            matching_mode: "exact".to_string(),
            fuzzy_threshold: 0.9,
            new_cards_per_day: 30,
            reviews_per_day: 150,
            daily_reset_hour: 4,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let api = settings.to_api_settings();

        assert_eq!(api.algorithm, Algorithm::Fsrs);
        assert_eq!(api.rating_scale, RatingScale::TwoPoint);
        assert_eq!(api.matching_mode, MatchingMode::Exact);
        assert_eq!(api.fuzzy_threshold, 0.9);
        assert_eq!(api.new_cards_per_day, 30);
    }

    #[test]
    fn test_db_global_settings_matching_mode_case_insensitive() {
        let settings = DbGlobalSettings {
            matching_mode: "case_insensitive".to_string(),
            ..DbGlobalSettings::default_for_device(Uuid::new_v4())
        };
        assert_eq!(
            settings.to_api_settings().matching_mode,
            MatchingMode::CaseInsensitive
        );
    }

    // === DbDeckSettings tests ===

    #[test]
    fn test_db_deck_settings_to_api_all_none() {
        let settings = DbDeckSettings {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            deck_path: "test/deck".to_string(),
            algorithm: None,
            rating_scale: None,
            matching_mode: None,
            fuzzy_threshold: None,
            new_cards_per_day: None,
            reviews_per_day: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let api = settings.to_api_settings();

        assert_eq!(api.deck_path, "test/deck");
        assert!(api.algorithm.is_none());
        assert!(api.rating_scale.is_none());
    }

    #[test]
    fn test_db_deck_settings_to_api_with_overrides() {
        let settings = DbDeckSettings {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            deck_path: "test".to_string(),
            algorithm: Some("fsrs".to_string()),
            rating_scale: Some("2point".to_string()),
            matching_mode: Some("exact".to_string()),
            fuzzy_threshold: Some(0.95),
            new_cards_per_day: Some(50),
            reviews_per_day: Some(100),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let api = settings.to_api_settings();

        assert_eq!(api.algorithm, Some(Algorithm::Fsrs));
        assert_eq!(api.rating_scale, Some(RatingScale::TwoPoint));
        assert_eq!(api.fuzzy_threshold, Some(0.95));
    }

    // === EffectiveSettings tests ===

    #[test]
    fn test_effective_settings_merge_no_deck() {
        let global = DbGlobalSettings::default_for_device(Uuid::new_v4());
        let effective = EffectiveSettings::merge(&global, None);

        assert_eq!(effective.algorithm, global.algorithm);
        assert_eq!(effective.rating_scale, global.rating_scale);
        assert_eq!(effective.new_cards_per_day, global.new_cards_per_day);
    }

    #[test]
    fn test_effective_settings_merge_with_deck_overrides() {
        let global = DbGlobalSettings::default_for_device(Uuid::new_v4());
        let deck = DbDeckSettings {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            deck_path: "test".to_string(),
            algorithm: Some("fsrs".to_string()),
            rating_scale: None,
            matching_mode: Some("exact".to_string()),
            fuzzy_threshold: Some(0.99),
            new_cards_per_day: Some(5),
            reviews_per_day: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let effective = EffectiveSettings::merge(&global, Some(&deck));

        assert_eq!(effective.algorithm, "fsrs");
        assert_eq!(effective.rating_scale, "4point"); // From global
        assert_eq!(effective.matching_mode, "exact");
        assert_eq!(effective.fuzzy_threshold, 0.99);
        assert_eq!(effective.new_cards_per_day, 5);
        assert_eq!(effective.reviews_per_day, 200); // From global
    }

    #[test]
    fn test_effective_settings_daily_reset_always_from_global() {
        let mut global = DbGlobalSettings::default_for_device(Uuid::new_v4());
        global.daily_reset_hour = 6;

        let deck = DbDeckSettings {
            id: Uuid::new_v4(),
            device_id: Uuid::new_v4(),
            deck_path: "test".to_string(),
            algorithm: Some("sm2".to_string()),
            rating_scale: None,
            matching_mode: None,
            fuzzy_threshold: None,
            new_cards_per_day: None,
            reviews_per_day: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let effective = EffectiveSettings::merge(&global, Some(&deck));
        assert_eq!(effective.daily_reset_hour, 6);
    }
}
