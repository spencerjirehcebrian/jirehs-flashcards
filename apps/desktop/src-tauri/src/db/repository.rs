//! Repository pattern for database access.

use crate::db::date_utils::{get_adjusted_today, get_adjusted_today_string};
use crate::db::error::DbError;
use chrono::{DateTime, NaiveDate, Utc};
use flashcard_core::types::{
    Algorithm, Card, CardState, CardStatus, Deck, DeckSettings, EffectiveSettings, GlobalSettings,
    MatchingMode, RatingScale, RawCard, StudyQueue,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

type Result<T> = std::result::Result<T, DbError>;

/// Repository for card operations.
pub trait CardRepository {
    fn get_card(&self, id: i64) -> Result<Option<Card>>;
    fn get_cards_by_deck(&self, deck_path: &str) -> Result<Vec<Card>>;
    fn upsert_cards(&self, cards: &[Card]) -> Result<()>;
    fn upsert_cards_from_sync(&self, cards: &[Card], synced_at: &str) -> Result<usize>;
    fn delete_cards(&self, ids: &[i64]) -> Result<()>;
    fn get_new_cards(&self, deck_path: Option<&str>, limit: usize) -> Result<Vec<Card>>;
    fn get_due_cards(
        &self,
        deck_path: Option<&str>,
        limit: usize,
        daily_reset_hour: u32,
    ) -> Result<Vec<Card>>;
}

/// Repository for card state operations.
pub trait StateRepository {
    fn get_card_state(&self, card_id: i64) -> Result<Option<CardState>>;
    fn save_card_state(&self, card_id: i64, state: &CardState) -> Result<()>;
    fn save_card_states_synced(&self, states: &[(i64, CardState)]) -> Result<usize>;
}

/// Repository for deck operations.
pub trait DeckRepository {
    fn get_all_decks(&self, daily_reset_hour: u32) -> Result<Vec<Deck>>;
    fn get_deck(&self, path: &str, daily_reset_hour: u32) -> Result<Option<Deck>>;
}

/// Repository for settings operations.
pub trait SettingsRepository {
    fn get_global_settings(&self) -> Result<GlobalSettings>;
    fn save_global_settings(&self, settings: &GlobalSettings) -> Result<()>;
    fn get_deck_settings(&self, deck_path: &str) -> Result<Option<DeckSettings>>;
    fn save_deck_settings(&self, settings: &DeckSettings) -> Result<()>;
    fn delete_deck_settings(&self, deck_path: &str) -> Result<()>;
    fn get_effective_settings(&self, deck_path: Option<&str>) -> Result<EffectiveSettings>;
}

/// Deck statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeckStats {
    pub total_cards: usize,
    pub new_cards: usize,
    pub learning_cards: usize,
    pub review_cards: usize,
    pub average_ease: f64,
    pub average_interval: f64,
}

/// Overall study statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StudyStats {
    pub reviews_today: usize,
    pub new_today: usize,
    pub streak_days: usize,
    pub retention_rate: f64,
    pub total_reviews: usize,
}

/// Calendar data point.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CalendarData {
    pub date: String,
    pub reviews: usize,
}

/// Pending review record for sync.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingReview {
    pub id: i64,
    pub card_id: i64,
    pub reviewed_at: String,
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

/// MD file sync info.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MdFileInfo {
    pub file_path: String,
    pub content_hash: String,
    pub last_modified: String,
}

/// Local sync state.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalSyncState {
    pub last_sync_at: Option<String>,
    pub pending_changes: i32,
}

/// Device info.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalDeviceInfo {
    pub token: String,
    pub device_id: Option<String>,
}

/// Repository for sync operations.
pub trait SyncRepository {
    fn get_pending_reviews(&self) -> Result<Vec<PendingReview>>;
    fn mark_reviews_synced(&self, ids: &[i64]) -> Result<()>;
    fn insert_pending_review(&self, review: &PendingReview) -> Result<i64>;
    fn get_pending_files(&self) -> Result<Vec<MdFileInfo>>;
    fn update_file_hash(&self, path: &str, hash: &str, last_modified: &str) -> Result<()>;
    fn mark_file_pending(&self, path: &str) -> Result<()>;
    fn clear_pending_upload(&self, path: &str) -> Result<()>;
    fn get_sync_state(&self) -> Result<LocalSyncState>;
    fn update_sync_state(&self, last_sync_at: &str) -> Result<()>;
    fn increment_pending_changes(&self) -> Result<()>;
    fn reset_pending_changes(&self) -> Result<()>;
    fn get_device_token(&self) -> Result<Option<LocalDeviceInfo>>;
    fn save_device_token(&self, token: &str, device_id: &str) -> Result<()>;
}

/// Repository for statistics operations.
pub trait StatsRepository {
    fn get_deck_stats(&self, deck_path: Option<&str>) -> Result<DeckStats>;
    fn get_study_stats(&self, daily_reset_hour: u32) -> Result<StudyStats>;
    fn get_calendar_data(&self, days: usize, daily_reset_hour: u32) -> Result<Vec<CalendarData>>;
}

/// SQLite implementation of repositories.
pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    /// Open database at path, creating if necessary.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let repo = Self { conn };
        repo.initialize()?;
        Ok(repo)
    }

    /// Open in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let repo = Self { conn };
        repo.initialize()?;
        Ok(repo)
    }

    fn initialize(&self) -> Result<()> {
        self.conn.execute_batch(super::schema::SCHEMA)?;
        self.conn.execute_batch(super::schema::INIT_GLOBAL_SETTINGS)?;
        self.conn.execute_batch(super::schema::INIT_SYNC_STATE)?;
        Ok(())
    }

    /// Import cards from parsed markdown.
    pub fn import_cards(&self, deck_path: &str, source_file: &str, raw_cards: &[RawCard]) -> Result<Vec<i64>> {
        let mut ids = Vec::with_capacity(raw_cards.len());

        for raw in raw_cards {
            let id = if let Some(id) = raw.id {
                self.conn.execute(
                    "INSERT OR REPLACE INTO cards (id, deck_path, question_text, answer_text, source_file) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, deck_path, raw.question, raw.answer, source_file],
                )?;
                id
            } else {
                self.conn.execute(
                    "INSERT INTO cards (deck_path, question_text, answer_text, source_file) VALUES (?1, ?2, ?3, ?4)",
                    params![deck_path, raw.question, raw.answer, source_file],
                )?;
                self.conn.last_insert_rowid()
            };
            ids.push(id);

            // Initialize card state if not exists
            self.conn.execute(
                "INSERT OR IGNORE INTO card_states (card_id) VALUES (?1)",
                params![id],
            )?;
        }

        Ok(ids)
    }

    /// Soft-delete all cards from a specific source file.
    pub fn delete_cards_by_source_file(&self, source_file: &str) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        let count = self.conn.execute(
            "UPDATE cards SET deleted_at = ?1 WHERE source_file = ?2 AND deleted_at IS NULL",
            params![now, source_file],
        )?;
        Ok(count)
    }
}

impl CardRepository for SqliteRepository {
    fn get_card(&self, id: i64) -> Result<Option<Card>> {
        self.conn
            .query_row(
                "SELECT id, deck_path, question_text, answer_text, source_file, deleted_at FROM cards WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Card {
                        id: row.get(0)?,
                        deck_path: row.get(1)?,
                        question: row.get(2)?,
                        answer: row.get(3)?,
                        source_file: row.get(4)?,
                        deleted_at: row.get::<_, Option<String>>(5)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn get_cards_by_deck(&self, deck_path: &str) -> Result<Vec<Card>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, deck_path, question_text, answer_text, source_file, deleted_at FROM cards WHERE deck_path = ?1 AND deleted_at IS NULL",
        )?;

        let cards = stmt
            .query_map(params![deck_path], |row| {
                Ok(Card {
                    id: row.get(0)?,
                    deck_path: row.get(1)?,
                    question: row.get(2)?,
                    answer: row.get(3)?,
                    source_file: row.get(4)?,
                    deleted_at: None,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(cards)
    }

    fn upsert_cards(&self, cards: &[Card]) -> Result<()> {
        for card in cards {
            self.conn.execute(
                "INSERT OR REPLACE INTO cards (id, deck_path, question_text, answer_text, source_file) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![card.id, card.deck_path, card.question, card.answer, card.source_file],
            )?;
        }
        Ok(())
    }

    fn delete_cards(&self, ids: &[i64]) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        for id in ids {
            self.conn.execute(
                "UPDATE cards SET deleted_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        }
        Ok(())
    }

    fn upsert_cards_from_sync(&self, cards: &[Card], synced_at: &str) -> Result<usize> {
        let mut count = 0;
        for card in cards {
            let deleted_at_str = card.deleted_at.map(|d| d.to_rfc3339());
            self.conn.execute(
                "INSERT OR REPLACE INTO cards (id, deck_path, question_text, answer_text, source_file, deleted_at, synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![card.id, card.deck_path, card.question, card.answer, card.source_file, deleted_at_str, synced_at],
            )?;

            // Initialize card state if not exists
            self.conn.execute(
                "INSERT OR IGNORE INTO card_states (card_id) VALUES (?1)",
                params![card.id],
            )?;

            count += 1;
        }
        Ok(count)
    }

    fn get_new_cards(&self, deck_path: Option<&str>, limit: usize) -> Result<Vec<Card>> {
        let sql = match deck_path {
            Some(_) => "SELECT c.id, c.deck_path, c.question_text, c.answer_text, c.source_file
                FROM cards c
                JOIN card_states cs ON c.id = cs.card_id
                WHERE c.deck_path = ?1 AND c.deleted_at IS NULL AND cs.status = 'new'
                LIMIT ?2",
            None => "SELECT c.id, c.deck_path, c.question_text, c.answer_text, c.source_file
                FROM cards c
                JOIN card_states cs ON c.id = cs.card_id
                WHERE c.deleted_at IS NULL AND cs.status = 'new'
                LIMIT ?1",
        };

        let mut stmt = self.conn.prepare(sql)?;
        let cards = if let Some(path) = deck_path {
            stmt.query_map(params![path, limit], Self::row_to_card)?
        } else {
            stmt.query_map(params![limit], Self::row_to_card)?
        };

        cards.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn get_due_cards(
        &self,
        deck_path: Option<&str>,
        limit: usize,
        daily_reset_hour: u32,
    ) -> Result<Vec<Card>> {
        let today = get_adjusted_today_string(daily_reset_hour);
        let sql = match deck_path {
            Some(_) => "SELECT c.id, c.deck_path, c.question_text, c.answer_text, c.source_file
                FROM cards c
                JOIN card_states cs ON c.id = cs.card_id
                WHERE c.deck_path = ?1 AND c.deleted_at IS NULL AND cs.status != 'new' AND cs.due_date <= ?2
                ORDER BY cs.due_date
                LIMIT ?3",
            None => "SELECT c.id, c.deck_path, c.question_text, c.answer_text, c.source_file
                FROM cards c
                JOIN card_states cs ON c.id = cs.card_id
                WHERE c.deleted_at IS NULL AND cs.status != 'new' AND cs.due_date <= ?1
                ORDER BY cs.due_date
                LIMIT ?2",
        };

        let mut stmt = self.conn.prepare(sql)?;
        let cards = if let Some(path) = deck_path {
            stmt.query_map(params![path, today, limit], Self::row_to_card)?
        } else {
            stmt.query_map(params![today, limit], Self::row_to_card)?
        };

        cards.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }
}

impl SqliteRepository {
    fn row_to_card(row: &rusqlite::Row) -> rusqlite::Result<Card> {
        Ok(Card {
            id: row.get(0)?,
            deck_path: row.get(1)?,
            question: row.get(2)?,
            answer: row.get(3)?,
            source_file: row.get(4)?,
            deleted_at: None,
        })
    }
}

impl StateRepository for SqliteRepository {
    fn get_card_state(&self, card_id: i64) -> Result<Option<CardState>> {
        self.conn
            .query_row(
                "SELECT status, interval_days, ease_factor, due_date, stability, difficulty, lapses, reviews_count FROM card_states WHERE card_id = ?1",
                params![card_id],
                |row| {
                    let status_str: String = row.get(0)?;
                    let status = match status_str.as_str() {
                        "new" => CardStatus::New,
                        "learning" => CardStatus::Learning,
                        "review" => CardStatus::Review,
                        "relearning" => CardStatus::Relearning,
                        _ => CardStatus::New,
                    };
                    let due_str: Option<String> = row.get(3)?;
                    let due_date = due_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc)));

                    Ok(CardState {
                        status,
                        interval_days: row.get(1)?,
                        ease_factor: row.get(2)?,
                        due_date,
                        stability: row.get(4)?,
                        difficulty: row.get(5)?,
                        lapses: row.get(6)?,
                        reviews_count: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn save_card_state(&self, card_id: i64, state: &CardState) -> Result<()> {
        let status_str = match state.status {
            CardStatus::New => "new",
            CardStatus::Learning => "learning",
            CardStatus::Review => "review",
            CardStatus::Relearning => "relearning",
        };
        let due_str = state.due_date.map(|d| d.to_rfc3339());

        self.conn.execute(
            "INSERT OR REPLACE INTO card_states (card_id, status, interval_days, ease_factor, due_date, stability, difficulty, lapses, reviews_count, synced) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)",
            params![card_id, status_str, state.interval_days, state.ease_factor, due_str, state.stability, state.difficulty, state.lapses, state.reviews_count],
        )?;
        Ok(())
    }

    fn save_card_states_synced(&self, states: &[(i64, CardState)]) -> Result<usize> {
        let mut count = 0;
        for (card_id, state) in states {
            let status_str = match state.status {
                CardStatus::New => "new",
                CardStatus::Learning => "learning",
                CardStatus::Review => "review",
                CardStatus::Relearning => "relearning",
            };
            let due_str = state.due_date.map(|d| d.to_rfc3339());

            self.conn.execute(
                "INSERT OR REPLACE INTO card_states (card_id, status, interval_days, ease_factor, due_date, stability, difficulty, lapses, reviews_count, synced)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
                params![card_id, status_str, state.interval_days, state.ease_factor, due_str, state.stability, state.difficulty, state.lapses, state.reviews_count],
            )?;
            count += 1;
        }
        Ok(count)
    }
}

impl DeckRepository for SqliteRepository {
    fn get_all_decks(&self, daily_reset_hour: u32) -> Result<Vec<Deck>> {
        let today = get_adjusted_today_string(daily_reset_hour);
        let mut stmt = self.conn.prepare(
            "SELECT deck_path, COUNT(*) as total,
                SUM(CASE WHEN cs.status = 'new' THEN 1 ELSE 0 END) as new_count,
                SUM(CASE WHEN cs.status != 'new' AND cs.due_date <= ?1 THEN 1 ELSE 0 END) as due_count
            FROM cards c
            LEFT JOIN card_states cs ON c.id = cs.card_id
            WHERE c.deleted_at IS NULL
            GROUP BY deck_path",
        )?;

        let decks = stmt
            .query_map(params![today], |row| {
                let path: String = row.get(0)?;
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                Ok(Deck {
                    path: path.clone(),
                    name,
                    card_count: row.get(1)?,
                    new_count: row.get(2)?,
                    due_count: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(decks)
    }

    fn get_deck(&self, path: &str, daily_reset_hour: u32) -> Result<Option<Deck>> {
        let today = get_adjusted_today_string(daily_reset_hour);
        self.conn
            .query_row(
                "SELECT deck_path, COUNT(*) as total,
                    SUM(CASE WHEN cs.status = 'new' THEN 1 ELSE 0 END) as new_count,
                    SUM(CASE WHEN cs.status != 'new' AND cs.due_date <= ?1 THEN 1 ELSE 0 END) as due_count
                FROM cards c
                LEFT JOIN card_states cs ON c.id = cs.card_id
                WHERE c.deleted_at IS NULL AND c.deck_path = ?2
                GROUP BY deck_path",
                params![today, path],
                |row| {
                    let path: String = row.get(0)?;
                    let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                    Ok(Deck {
                        path: path.clone(),
                        name,
                        card_count: row.get(1)?,
                        new_count: row.get(2)?,
                        due_count: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }
}

impl SettingsRepository for SqliteRepository {
    fn get_global_settings(&self) -> Result<GlobalSettings> {
        self.conn
            .query_row(
                "SELECT algorithm, rating_scale, matching_mode, fuzzy_threshold, new_cards_per_day, reviews_per_day, daily_reset_hour FROM global_settings WHERE id = 1",
                [],
                |row| {
                    let algorithm_str: String = row.get(0)?;
                    let rating_scale_str: String = row.get(1)?;
                    let matching_mode_str: String = row.get(2)?;

                    Ok(GlobalSettings {
                        algorithm: Algorithm::from_str(&algorithm_str).unwrap_or_default(),
                        rating_scale: match rating_scale_str.as_str() {
                            "2point" => RatingScale::TwoPoint,
                            _ => RatingScale::FourPoint,
                        },
                        matching_mode: match matching_mode_str.as_str() {
                            "exact" => MatchingMode::Exact,
                            "case_insensitive" => MatchingMode::CaseInsensitive,
                            _ => MatchingMode::Fuzzy,
                        },
                        fuzzy_threshold: row.get(3)?,
                        new_cards_per_day: row.get(4)?,
                        reviews_per_day: row.get(5)?,
                        daily_reset_hour: row.get(6)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    fn save_global_settings(&self, settings: &GlobalSettings) -> Result<()> {
        let algorithm_str = settings.algorithm.as_str();
        let rating_scale_str = match settings.rating_scale {
            RatingScale::FourPoint => "4point",
            RatingScale::TwoPoint => "2point",
        };
        let matching_mode_str = match settings.matching_mode {
            MatchingMode::Exact => "exact",
            MatchingMode::CaseInsensitive => "case_insensitive",
            MatchingMode::Fuzzy => "fuzzy",
        };

        self.conn.execute(
            "UPDATE global_settings SET algorithm = ?1, rating_scale = ?2, matching_mode = ?3, fuzzy_threshold = ?4, new_cards_per_day = ?5, reviews_per_day = ?6, daily_reset_hour = ?7, synced = 0 WHERE id = 1",
            params![
                algorithm_str,
                rating_scale_str,
                matching_mode_str,
                settings.fuzzy_threshold,
                settings.new_cards_per_day,
                settings.reviews_per_day,
                settings.daily_reset_hour,
            ],
        )?;

        Ok(())
    }

    fn get_deck_settings(&self, deck_path: &str) -> Result<Option<DeckSettings>> {
        self.conn
            .query_row(
                "SELECT deck_path, algorithm, rating_scale, matching_mode, fuzzy_threshold, new_cards_per_day, reviews_per_day FROM deck_settings WHERE deck_path = ?1",
                params![deck_path],
                |row| {
                    let deck_path: String = row.get(0)?;
                    let algorithm_str: Option<String> = row.get(1)?;
                    let rating_scale_str: Option<String> = row.get(2)?;
                    let matching_mode_str: Option<String> = row.get(3)?;

                    Ok(DeckSettings {
                        deck_path,
                        algorithm: algorithm_str.and_then(|s| Algorithm::from_str(&s)),
                        rating_scale: rating_scale_str.map(|s| match s.as_str() {
                            "2point" => RatingScale::TwoPoint,
                            _ => RatingScale::FourPoint,
                        }),
                        matching_mode: matching_mode_str.map(|s| match s.as_str() {
                            "exact" => MatchingMode::Exact,
                            "case_insensitive" => MatchingMode::CaseInsensitive,
                            _ => MatchingMode::Fuzzy,
                        }),
                        fuzzy_threshold: row.get(4)?,
                        new_cards_per_day: row.get(5)?,
                        reviews_per_day: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn save_deck_settings(&self, settings: &DeckSettings) -> Result<()> {
        let algorithm_str = settings.algorithm.map(|a| a.as_str().to_string());
        let rating_scale_str = settings.rating_scale.map(|rs| match rs {
            RatingScale::FourPoint => "4point".to_string(),
            RatingScale::TwoPoint => "2point".to_string(),
        });
        let matching_mode_str = settings.matching_mode.map(|mm| match mm {
            MatchingMode::Exact => "exact".to_string(),
            MatchingMode::CaseInsensitive => "case_insensitive".to_string(),
            MatchingMode::Fuzzy => "fuzzy".to_string(),
        });

        self.conn.execute(
            "INSERT OR REPLACE INTO deck_settings (deck_path, algorithm, rating_scale, matching_mode, fuzzy_threshold, new_cards_per_day, reviews_per_day, synced) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                settings.deck_path,
                algorithm_str,
                rating_scale_str,
                matching_mode_str,
                settings.fuzzy_threshold,
                settings.new_cards_per_day,
                settings.reviews_per_day,
            ],
        )?;

        Ok(())
    }

    fn delete_deck_settings(&self, deck_path: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM deck_settings WHERE deck_path = ?1",
            params![deck_path],
        )?;
        Ok(())
    }

    fn get_effective_settings(&self, deck_path: Option<&str>) -> Result<EffectiveSettings> {
        let global = self.get_global_settings()?;
        let deck = match deck_path {
            Some(path) => self.get_deck_settings(path)?,
            None => None,
        };
        Ok(EffectiveSettings::merge(&global, deck.as_ref()))
    }
}

use crate::sync::{ApiDeckSettings, ApiGlobalSettings};

impl SqliteRepository {
    /// Save global settings from cloud sync (marks as synced).
    pub fn save_global_settings_synced(&self, settings: &ApiGlobalSettings) -> Result<()> {
        self.conn.execute(
            "UPDATE global_settings SET algorithm = ?1, rating_scale = ?2, matching_mode = ?3, fuzzy_threshold = ?4, new_cards_per_day = ?5, reviews_per_day = ?6, daily_reset_hour = ?7, synced = 1 WHERE id = 1",
            params![
                settings.algorithm,
                settings.rating_scale,
                settings.matching_mode,
                settings.fuzzy_threshold,
                settings.new_cards_per_day,
                settings.reviews_per_day,
                settings.daily_reset_hour,
            ],
        )?;
        Ok(())
    }

    /// Save deck settings from cloud sync (marks as synced).
    pub fn save_deck_settings_synced(&self, settings: &ApiDeckSettings) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO deck_settings (deck_path, algorithm, rating_scale, matching_mode, fuzzy_threshold, new_cards_per_day, reviews_per_day, synced) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            params![
                settings.deck_path,
                settings.algorithm,
                settings.rating_scale,
                settings.matching_mode,
                settings.fuzzy_threshold,
                settings.new_cards_per_day,
                settings.reviews_per_day,
            ],
        )?;
        Ok(())
    }
}

impl StatsRepository for SqliteRepository {
    fn get_deck_stats(&self, deck_path: Option<&str>) -> Result<DeckStats> {
        let (total, new, learning, review, avg_ease, avg_interval) = match deck_path {
            Some(path) => {
                self.conn.query_row(
                    "SELECT
                        COUNT(*) as total,
                        SUM(CASE WHEN cs.status = 'new' THEN 1 ELSE 0 END) as new_count,
                        SUM(CASE WHEN cs.status = 'learning' OR cs.status = 'relearning' THEN 1 ELSE 0 END) as learning_count,
                        SUM(CASE WHEN cs.status = 'review' THEN 1 ELSE 0 END) as review_count,
                        COALESCE(AVG(cs.ease_factor), 2.5) as avg_ease,
                        COALESCE(AVG(CASE WHEN cs.interval_days > 0 THEN cs.interval_days END), 0) as avg_interval
                    FROM cards c
                    LEFT JOIN card_states cs ON c.id = cs.card_id
                    WHERE c.deleted_at IS NULL AND c.deck_path = ?1",
                    params![path],
                    |row| Ok((
                        row.get::<_, usize>(0)?,
                        row.get::<_, usize>(1)?,
                        row.get::<_, usize>(2)?,
                        row.get::<_, usize>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                    )),
                )?
            }
            None => {
                self.conn.query_row(
                    "SELECT
                        COUNT(*) as total,
                        SUM(CASE WHEN cs.status = 'new' THEN 1 ELSE 0 END) as new_count,
                        SUM(CASE WHEN cs.status = 'learning' OR cs.status = 'relearning' THEN 1 ELSE 0 END) as learning_count,
                        SUM(CASE WHEN cs.status = 'review' THEN 1 ELSE 0 END) as review_count,
                        COALESCE(AVG(cs.ease_factor), 2.5) as avg_ease,
                        COALESCE(AVG(CASE WHEN cs.interval_days > 0 THEN cs.interval_days END), 0) as avg_interval
                    FROM cards c
                    LEFT JOIN card_states cs ON c.id = cs.card_id
                    WHERE c.deleted_at IS NULL",
                    [],
                    |row| Ok((
                        row.get::<_, usize>(0)?,
                        row.get::<_, usize>(1)?,
                        row.get::<_, usize>(2)?,
                        row.get::<_, usize>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, f64>(5)?,
                    )),
                )?
            }
        };

        Ok(DeckStats {
            total_cards: total,
            new_cards: new,
            learning_cards: learning,
            review_cards: review,
            average_ease: avg_ease,
            average_interval: avg_interval,
        })
    }

    fn get_study_stats(&self, daily_reset_hour: u32) -> Result<StudyStats> {
        let today = get_adjusted_today_string(daily_reset_hour);
        let today_date = get_adjusted_today(daily_reset_hour);

        // Get today's review count
        let reviews_today: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM pending_reviews WHERE date(reviewed_at) = ?1",
            params![today],
            |row| row.get(0),
        ).unwrap_or(0);

        // Get today's new cards seen (cards that were 'new' status and got reviewed today)
        let new_today: usize = self.conn.query_row(
            "SELECT COUNT(DISTINCT card_id) FROM pending_reviews
             WHERE date(reviewed_at) = ?1",
            params![today],
            |row| row.get(0),
        ).unwrap_or(0);

        // Get total reviews
        let total_reviews: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM pending_reviews",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Calculate streak (consecutive days with reviews)
        let mut streak_days = 0usize;
        let mut current_date = today_date;

        loop {
            let date_str = current_date.format("%Y-%m-%d").to_string();
            let count: usize = self.conn.query_row(
                "SELECT COUNT(*) FROM pending_reviews WHERE date(reviewed_at) = ?1",
                params![date_str],
                |row| row.get(0),
            ).unwrap_or(0);

            if count > 0 {
                streak_days += 1;
                current_date = current_date.pred_opt().unwrap_or(current_date);
            } else if streak_days == 0 && current_date == today_date {
                // Allow for today not having reviews yet
                current_date = current_date.pred_opt().unwrap_or(current_date);
            } else {
                break;
            }

            // Safety limit
            if streak_days > 365 {
                break;
            }
        }

        // Calculate retention rate (correct reviews / total reviews)
        let retention_rate: f64 = self.conn.query_row(
            "SELECT COALESCE(
                CAST(SUM(CASE WHEN rating >= 3 THEN 1 ELSE 0 END) AS REAL) /
                NULLIF(COUNT(*), 0),
                0.0
            ) FROM pending_reviews",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);

        Ok(StudyStats {
            reviews_today,
            new_today,
            streak_days,
            retention_rate,
            total_reviews,
        })
    }

    fn get_calendar_data(&self, days: usize, daily_reset_hour: u32) -> Result<Vec<CalendarData>> {
        let mut data = Vec::new();
        let today = get_adjusted_today(daily_reset_hour);

        for i in 0..days {
            let date = today - chrono::Duration::days(i as i64);
            let date_str = date.format("%Y-%m-%d").to_string();

            let reviews: usize = self.conn.query_row(
                "SELECT COUNT(*) FROM pending_reviews WHERE date(reviewed_at) = ?1",
                params![date_str],
                |row| row.get(0),
            ).unwrap_or(0);

            data.push(CalendarData {
                date: date_str,
                reviews,
            });
        }

        // Reverse so oldest is first
        data.reverse();
        Ok(data)
    }
}

impl SyncRepository for SqliteRepository {
    fn get_pending_reviews(&self) -> Result<Vec<PendingReview>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, card_id, reviewed_at, rating, rating_scale, answer_mode, typed_answer,
                    was_correct, time_taken_ms, interval_before, interval_after,
                    ease_before, ease_after, algorithm
             FROM pending_reviews WHERE synced = 0",
        )?;

        let reviews = stmt
            .query_map([], |row| {
                Ok(PendingReview {
                    id: row.get(0)?,
                    card_id: row.get(1)?,
                    reviewed_at: row.get(2)?,
                    rating: row.get(3)?,
                    rating_scale: row.get(4)?,
                    answer_mode: row.get(5)?,
                    typed_answer: row.get(6)?,
                    was_correct: row.get::<_, Option<i32>>(7)?.map(|v| v != 0),
                    time_taken_ms: row.get(8)?,
                    interval_before: row.get(9)?,
                    interval_after: row.get(10)?,
                    ease_before: row.get(11)?,
                    ease_after: row.get(12)?,
                    algorithm: row.get(13)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(reviews)
    }

    fn mark_reviews_synced(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE pending_reviews SET synced = 1 WHERE id IN ({})", placeholders);

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        stmt.execute(params.as_slice())?;

        Ok(())
    }

    fn insert_pending_review(&self, review: &PendingReview) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO pending_reviews (card_id, reviewed_at, rating, rating_scale, answer_mode,
                typed_answer, was_correct, time_taken_ms, interval_before, interval_after,
                ease_before, ease_after, algorithm, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0)",
            params![
                review.card_id,
                review.reviewed_at,
                review.rating,
                review.rating_scale,
                review.answer_mode,
                review.typed_answer,
                review.was_correct.map(|b| if b { 1 } else { 0 }),
                review.time_taken_ms,
                review.interval_before,
                review.interval_after,
                review.ease_before,
                review.ease_after,
                review.algorithm,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn get_pending_files(&self) -> Result<Vec<MdFileInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, content_hash, last_modified FROM md_files WHERE pending_upload = 1",
        )?;

        let files = stmt
            .query_map([], |row| {
                Ok(MdFileInfo {
                    file_path: row.get(0)?,
                    content_hash: row.get(1)?,
                    last_modified: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(files)
    }

    fn update_file_hash(&self, path: &str, hash: &str, last_modified: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO md_files (file_path, content_hash, last_modified, pending_upload)
             VALUES (?1, ?2, ?3, 0)",
            params![path, hash, last_modified],
        )?;
        Ok(())
    }

    fn mark_file_pending(&self, path: &str) -> Result<()> {
        // First check if file exists, update if so, otherwise insert
        let existing: Option<String> = self.conn
            .query_row(
                "SELECT file_path FROM md_files WHERE file_path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?;

        if existing.is_some() {
            self.conn.execute(
                "UPDATE md_files SET pending_upload = 1 WHERE file_path = ?1",
                params![path],
            )?;
        } else {
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "INSERT INTO md_files (file_path, content_hash, last_modified, pending_upload)
                 VALUES (?1, '', ?2, 1)",
                params![path, now],
            )?;
        }
        Ok(())
    }

    fn clear_pending_upload(&self, path: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE md_files SET pending_upload = 0 WHERE file_path = ?1",
            params![path],
        )?;
        Ok(())
    }

    fn get_sync_state(&self) -> Result<LocalSyncState> {
        self.conn
            .query_row(
                "SELECT last_sync_at, pending_changes FROM sync_state WHERE id = 1",
                [],
                |row| {
                    Ok(LocalSyncState {
                        last_sync_at: row.get(0)?,
                        pending_changes: row.get(1)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    fn update_sync_state(&self, last_sync_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_state SET last_sync_at = ?1, pending_changes = 0 WHERE id = 1",
            params![last_sync_at],
        )?;
        Ok(())
    }

    fn increment_pending_changes(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_state SET pending_changes = pending_changes + 1 WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    fn reset_pending_changes(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE sync_state SET pending_changes = 0 WHERE id = 1",
            [],
        )?;
        Ok(())
    }

    fn get_device_token(&self) -> Result<Option<LocalDeviceInfo>> {
        self.conn
            .query_row(
                "SELECT token, device_id FROM local_device LIMIT 1",
                [],
                |row| {
                    Ok(LocalDeviceInfo {
                        token: row.get(0)?,
                        device_id: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn save_device_token(&self, token: &str, device_id: &str) -> Result<()> {
        // Clear existing and insert new
        self.conn.execute("DELETE FROM local_device", [])?;
        self.conn.execute(
            "INSERT INTO local_device (token, device_id) VALUES (?1, ?2)",
            params![token, device_id],
        )?;
        Ok(())
    }
}
