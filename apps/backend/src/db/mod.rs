//! PostgreSQL database operations

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::error::{ApiError, Result};
use crate::models::*;

/// Database wrapper with connection pool
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Connect to PostgreSQL and create connection pool
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| ApiError::Database(e.into()))?;
        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // === Device Repository ===

    /// Create a new device with generated token
    pub async fn create_device(&self, name: Option<&str>) -> Result<Device> {
        let token = Uuid::new_v4().to_string();
        let device = sqlx::query_as::<_, Device>(
            r#"
            INSERT INTO devices (token, name)
            VALUES ($1, $2)
            RETURNING id, token, name, created_at, last_seen_at
            "#,
        )
        .bind(&token)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        // Create default global settings for the device
        sqlx::query(
            r#"
            INSERT INTO global_settings (device_id)
            VALUES ($1)
            "#,
        )
        .bind(device.id)
        .execute(&self.pool)
        .await?;

        Ok(device)
    }

    /// Get device by token
    pub async fn get_device_by_token(&self, token: &str) -> Result<Option<Device>> {
        let device = sqlx::query_as::<_, Device>(
            r#"
            SELECT id, token, name, created_at, last_seen_at
            FROM devices
            WHERE token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(device)
    }

    /// Update device last_seen_at timestamp
    pub async fn update_last_seen(&self, device_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE devices
            SET last_seen_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(device_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // === Card Repository ===

    /// Get next card ID from sequence
    pub async fn get_next_card_id(&self) -> Result<i64> {
        let row = sqlx::query("SELECT nextval('card_id_seq') as id")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("id"))
    }

    /// Get card by ID
    pub async fn get_card(&self, card_id: i64) -> Result<Option<DbCard>> {
        let card = sqlx::query_as::<_, DbCard>(
            r#"
            SELECT id, device_id, deck_path, question_text, answer_text,
                   question_hash, answer_hash, source_file, created_at, updated_at, deleted_at
            FROM cards
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(card_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(card)
    }

    /// Get all cards for a device, optionally filtered by deck
    pub async fn get_cards_by_device(
        &self,
        device_id: Uuid,
        deck_path: Option<&str>,
    ) -> Result<Vec<DbCard>> {
        let cards = match deck_path {
            Some(path) => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT id, device_id, deck_path, question_text, answer_text,
                           question_hash, answer_hash, source_file, created_at, updated_at, deleted_at
                    FROM cards
                    WHERE device_id = $1 AND deck_path = $2 AND deleted_at IS NULL
                    ORDER BY id
                    "#,
                )
                .bind(device_id)
                .bind(path)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT id, device_id, deck_path, question_text, answer_text,
                           question_hash, answer_hash, source_file, created_at, updated_at, deleted_at
                    FROM cards
                    WHERE device_id = $1 AND deleted_at IS NULL
                    ORDER BY id
                    "#,
                )
                .bind(device_id)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(cards)
    }

    /// Upsert a card (insert or update)
    pub async fn upsert_card(&self, card: &DbCard) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cards (id, device_id, deck_path, question_text, answer_text,
                              question_hash, answer_hash, source_file, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                deck_path = EXCLUDED.deck_path,
                question_text = EXCLUDED.question_text,
                answer_text = EXCLUDED.answer_text,
                question_hash = EXCLUDED.question_hash,
                answer_hash = EXCLUDED.answer_hash,
                source_file = EXCLUDED.source_file,
                updated_at = NOW(),
                deleted_at = NULL
            "#,
        )
        .bind(card.id)
        .bind(card.device_id)
        .bind(&card.deck_path)
        .bind(&card.question_text)
        .bind(&card.answer_text)
        .bind(&card.question_hash)
        .bind(&card.answer_hash)
        .bind(&card.source_file)
        .bind(card.created_at)
        .bind(card.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Soft delete cards by IDs
    pub async fn soft_delete_cards(&self, card_ids: &[i64]) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE cards
            SET deleted_at = NOW()
            WHERE id = ANY($1)
            "#,
        )
        .bind(card_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    /// Get orphaned cards (cards in DB not in the provided list)
    pub async fn get_orphaned_cards(
        &self,
        device_id: Uuid,
        current_card_ids: &[i64],
    ) -> Result<Vec<OrphanedCard>> {
        let orphans = sqlx::query_as::<_, OrphanedCard>(
            r#"
            SELECT id, LEFT(question_text, 50) as question_preview
            FROM cards
            WHERE device_id = $1 AND deleted_at IS NULL AND id != ALL($2)
            "#,
        )
        .bind(device_id)
        .bind(current_card_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(orphans)
    }

    // === Card State Repository ===

    /// Get card state
    pub async fn get_card_state(&self, card_id: i64, device_id: Uuid) -> Result<Option<DbCardState>> {
        let state = sqlx::query_as::<_, DbCardState>(
            r#"
            SELECT id, card_id, device_id, status, interval_days, ease_factor,
                   due_date, stability, difficulty, lapses, reviews_count,
                   created_at, updated_at
            FROM card_states
            WHERE card_id = $1 AND device_id = $2
            "#,
        )
        .bind(card_id)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(state)
    }

    /// Upsert card state
    pub async fn upsert_card_state(
        &self,
        card_id: i64,
        device_id: Uuid,
        state: &DbCardState,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO card_states (card_id, device_id, status, interval_days, ease_factor,
                                    due_date, stability, difficulty, lapses, reviews_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (card_id, device_id) DO UPDATE SET
                status = EXCLUDED.status,
                interval_days = EXCLUDED.interval_days,
                ease_factor = EXCLUDED.ease_factor,
                due_date = EXCLUDED.due_date,
                stability = EXCLUDED.stability,
                difficulty = EXCLUDED.difficulty,
                lapses = EXCLUDED.lapses,
                reviews_count = EXCLUDED.reviews_count,
                updated_at = NOW()
            "#,
        )
        .bind(card_id)
        .bind(device_id)
        .bind(&state.status)
        .bind(state.interval_days)
        .bind(state.ease_factor)
        .bind(state.due_date)
        .bind(state.stability)
        .bind(state.difficulty)
        .bind(state.lapses)
        .bind(state.reviews_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get new cards for study
    pub async fn get_new_cards(
        &self,
        device_id: Uuid,
        deck_path: Option<&str>,
        limit: i32,
    ) -> Result<Vec<DbCard>> {
        let cards = match deck_path {
            Some(path) => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT c.id, c.device_id, c.deck_path, c.question_text, c.answer_text,
                           c.question_hash, c.answer_hash, c.source_file, c.created_at, c.updated_at, c.deleted_at
                    FROM cards c
                    LEFT JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
                    WHERE c.device_id = $1 AND c.deck_path = $2 AND c.deleted_at IS NULL
                      AND (cs.status IS NULL OR cs.status = 'new')
                    ORDER BY c.id
                    LIMIT $3
                    "#,
                )
                .bind(device_id)
                .bind(path)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT c.id, c.device_id, c.deck_path, c.question_text, c.answer_text,
                           c.question_hash, c.answer_hash, c.source_file, c.created_at, c.updated_at, c.deleted_at
                    FROM cards c
                    LEFT JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
                    WHERE c.device_id = $1 AND c.deleted_at IS NULL
                      AND (cs.status IS NULL OR cs.status = 'new')
                    ORDER BY c.id
                    LIMIT $2
                    "#,
                )
                .bind(device_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(cards)
    }

    /// Get due cards for review
    pub async fn get_due_cards(
        &self,
        device_id: Uuid,
        deck_path: Option<&str>,
        limit: i32,
    ) -> Result<Vec<DbCard>> {
        let today = Utc::now().date_naive();

        let cards = match deck_path {
            Some(path) => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT c.id, c.device_id, c.deck_path, c.question_text, c.answer_text,
                           c.question_hash, c.answer_hash, c.source_file, c.created_at, c.updated_at, c.deleted_at
                    FROM cards c
                    JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
                    WHERE c.device_id = $1 AND c.deck_path = $2 AND c.deleted_at IS NULL
                      AND cs.status IN ('review', 'learning', 'relearning')
                      AND cs.due_date <= $3
                    ORDER BY cs.due_date
                    LIMIT $4
                    "#,
                )
                .bind(device_id)
                .bind(path)
                .bind(today)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT c.id, c.device_id, c.deck_path, c.question_text, c.answer_text,
                           c.question_hash, c.answer_hash, c.source_file, c.created_at, c.updated_at, c.deleted_at
                    FROM cards c
                    JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
                    WHERE c.device_id = $1 AND c.deleted_at IS NULL
                      AND cs.status IN ('review', 'learning', 'relearning')
                      AND cs.due_date <= $2
                    ORDER BY cs.due_date
                    LIMIT $3
                    "#,
                )
                .bind(device_id)
                .bind(today)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(cards)
    }

    // === Review Repository ===

    /// Insert a review record
    pub async fn insert_review(&self, review: &DbReview) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO reviews (id, card_id, device_id, reviewed_at, rating, rating_scale,
                                answer_mode, typed_answer, was_correct, time_taken_ms,
                                interval_before, interval_after, ease_before, ease_after, algorithm)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
        )
        .bind(review.id)
        .bind(review.card_id)
        .bind(review.device_id)
        .bind(review.reviewed_at)
        .bind(review.rating)
        .bind(&review.rating_scale)
        .bind(&review.answer_mode)
        .bind(&review.typed_answer)
        .bind(review.was_correct)
        .bind(review.time_taken_ms)
        .bind(review.interval_before)
        .bind(review.interval_after)
        .bind(review.ease_before)
        .bind(review.ease_after)
        .bind(&review.algorithm)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert multiple reviews (for sync)
    pub async fn insert_reviews(&self, reviews: &[DbReview]) -> Result<usize> {
        let mut count = 0;
        for review in reviews {
            self.insert_review(review).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Get reviews since a timestamp
    pub async fn get_reviews_since(
        &self,
        device_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<DbReview>> {
        let reviews = match since {
            Some(ts) => {
                sqlx::query_as::<_, DbReview>(
                    r#"
                    SELECT id, card_id, device_id, reviewed_at, rating, rating_scale,
                           answer_mode, typed_answer, was_correct, time_taken_ms,
                           interval_before, interval_after, ease_before, ease_after,
                           algorithm, created_at
                    FROM reviews
                    WHERE device_id = $1 AND created_at > $2
                    ORDER BY reviewed_at
                    "#,
                )
                .bind(device_id)
                .bind(ts)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbReview>(
                    r#"
                    SELECT id, card_id, device_id, reviewed_at, rating, rating_scale,
                           answer_mode, typed_answer, was_correct, time_taken_ms,
                           interval_before, interval_after, ease_before, ease_after,
                           algorithm, created_at
                    FROM reviews
                    WHERE device_id = $1
                    ORDER BY reviewed_at
                    "#,
                )
                .bind(device_id)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(reviews)
    }

    // === Settings Repository ===

    /// Get global settings for a device
    pub async fn get_global_settings(&self, device_id: Uuid) -> Result<DbGlobalSettings> {
        let settings = sqlx::query_as::<_, DbGlobalSettings>(
            r#"
            SELECT device_id, algorithm, rating_scale, matching_mode, fuzzy_threshold,
                   new_cards_per_day, reviews_per_day, daily_reset_hour, created_at, updated_at
            FROM global_settings
            WHERE device_id = $1
            "#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| DbGlobalSettings::default_for_device(device_id));

        Ok(settings)
    }

    /// Upsert global settings
    pub async fn upsert_global_settings(&self, device_id: Uuid, settings: &DbGlobalSettings) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO global_settings (device_id, algorithm, rating_scale, matching_mode,
                                        fuzzy_threshold, new_cards_per_day, reviews_per_day, daily_reset_hour)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (device_id) DO UPDATE SET
                algorithm = EXCLUDED.algorithm,
                rating_scale = EXCLUDED.rating_scale,
                matching_mode = EXCLUDED.matching_mode,
                fuzzy_threshold = EXCLUDED.fuzzy_threshold,
                new_cards_per_day = EXCLUDED.new_cards_per_day,
                reviews_per_day = EXCLUDED.reviews_per_day,
                daily_reset_hour = EXCLUDED.daily_reset_hour,
                updated_at = NOW()
            "#,
        )
        .bind(device_id)
        .bind(&settings.algorithm)
        .bind(&settings.rating_scale)
        .bind(&settings.matching_mode)
        .bind(settings.fuzzy_threshold)
        .bind(settings.new_cards_per_day)
        .bind(settings.reviews_per_day)
        .bind(settings.daily_reset_hour)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get deck settings
    pub async fn get_deck_settings(&self, device_id: Uuid, deck_path: &str) -> Result<Option<DbDeckSettings>> {
        let settings = sqlx::query_as::<_, DbDeckSettings>(
            r#"
            SELECT id, device_id, deck_path, algorithm, rating_scale, matching_mode,
                   fuzzy_threshold, new_cards_per_day, reviews_per_day, created_at, updated_at
            FROM deck_settings
            WHERE device_id = $1 AND deck_path = $2
            "#,
        )
        .bind(device_id)
        .bind(deck_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(settings)
    }

    /// Upsert deck settings
    pub async fn upsert_deck_settings(&self, device_id: Uuid, settings: &DbDeckSettings) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO deck_settings (device_id, deck_path, algorithm, rating_scale, matching_mode,
                                      fuzzy_threshold, new_cards_per_day, reviews_per_day)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (device_id, deck_path) DO UPDATE SET
                algorithm = EXCLUDED.algorithm,
                rating_scale = EXCLUDED.rating_scale,
                matching_mode = EXCLUDED.matching_mode,
                fuzzy_threshold = EXCLUDED.fuzzy_threshold,
                new_cards_per_day = EXCLUDED.new_cards_per_day,
                reviews_per_day = EXCLUDED.reviews_per_day,
                updated_at = NOW()
            "#,
        )
        .bind(device_id)
        .bind(&settings.deck_path)
        .bind(&settings.algorithm)
        .bind(&settings.rating_scale)
        .bind(&settings.matching_mode)
        .bind(settings.fuzzy_threshold)
        .bind(settings.new_cards_per_day)
        .bind(settings.reviews_per_day)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all deck settings for a device
    pub async fn get_all_deck_settings(&self, device_id: Uuid) -> Result<Vec<DbDeckSettings>> {
        let settings = sqlx::query_as::<_, DbDeckSettings>(
            r#"
            SELECT id, device_id, deck_path, algorithm, rating_scale, matching_mode,
                   fuzzy_threshold, new_cards_per_day, reviews_per_day, created_at, updated_at
            FROM deck_settings
            WHERE device_id = $1
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(settings)
    }

    /// Delete deck settings
    pub async fn delete_deck_settings(&self, device_id: Uuid, deck_path: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM deck_settings
            WHERE device_id = $1 AND deck_path = $2
            "#,
        )
        .bind(device_id)
        .bind(deck_path)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get effective settings (global merged with deck overrides)
    pub async fn get_effective_settings(
        &self,
        device_id: Uuid,
        deck_path: Option<&str>,
    ) -> Result<EffectiveSettings> {
        let global = self.get_global_settings(device_id).await?;

        let deck = match deck_path {
            Some(path) => self.get_deck_settings(device_id, path).await?,
            None => None,
        };

        Ok(EffectiveSettings::merge(&global, deck.as_ref()))
    }

    // === Deck Repository ===

    /// Get all decks for a device
    pub async fn get_all_decks(&self, device_id: Uuid) -> Result<Vec<DeckInfo>> {
        let decks = sqlx::query_as::<_, DeckInfo>(
            r#"
            SELECT
                c.deck_path as path,
                c.deck_path as name,
                COUNT(c.id)::INT as card_count,
                COUNT(CASE WHEN cs.status IS NULL OR cs.status = 'new' THEN 1 END)::INT as new_count,
                COUNT(CASE WHEN cs.status IN ('review', 'learning', 'relearning')
                           AND cs.due_date <= CURRENT_DATE THEN 1 END)::INT as due_count
            FROM cards c
            LEFT JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
            WHERE c.device_id = $1 AND c.deleted_at IS NULL
            GROUP BY c.deck_path
            ORDER BY c.deck_path
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(decks)
    }

    /// Get deck statistics
    pub async fn get_deck_stats(&self, device_id: Uuid, deck_path: &str) -> Result<DeckStatsResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(c.id)::INT as total_cards,
                COUNT(CASE WHEN cs.status IS NULL OR cs.status = 'new' THEN 1 END)::INT as new_cards,
                COUNT(CASE WHEN cs.status = 'learning' THEN 1 END)::INT as learning_cards,
                COUNT(CASE WHEN cs.status = 'review' THEN 1 END)::INT as review_cards,
                COALESCE(AVG(cs.ease_factor), 2.5)::FLOAT8 as average_ease,
                COALESCE(AVG(cs.interval_days), 0)::FLOAT8 as average_interval
            FROM cards c
            LEFT JOIN card_states cs ON c.id = cs.card_id AND cs.device_id = $1
            WHERE c.device_id = $1 AND c.deck_path = $2 AND c.deleted_at IS NULL
            "#,
        )
        .bind(device_id)
        .bind(deck_path)
        .fetch_one(&self.pool)
        .await?;

        let reviews_today: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM reviews r
            JOIN cards c ON r.card_id = c.id
            WHERE r.device_id = $1 AND c.deck_path = $2
              AND r.reviewed_at >= CURRENT_DATE
            "#,
        )
        .bind(device_id)
        .bind(deck_path)
        .fetch_one(&self.pool)
        .await?;

        // Calculate retention rate from recent reviews
        let retention: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(CASE WHEN rating >= 3 THEN 1.0 ELSE 0.0 END)::FLOAT8
            FROM reviews r
            JOIN cards c ON r.card_id = c.id
            WHERE r.device_id = $1 AND c.deck_path = $2
              AND r.reviewed_at >= CURRENT_DATE - INTERVAL '30 days'
            "#,
        )
        .bind(device_id)
        .bind(deck_path)
        .fetch_one(&self.pool)
        .await?;

        Ok(DeckStatsResponse {
            total_cards: row.get::<i32, _>("total_cards") as usize,
            new_cards: row.get::<i32, _>("new_cards") as usize,
            learning_cards: row.get::<i32, _>("learning_cards") as usize,
            review_cards: row.get::<i32, _>("review_cards") as usize,
            average_ease: row.get("average_ease"),
            average_interval: row.get("average_interval"),
            retention_rate: retention.unwrap_or(0.0),
            reviews_today: reviews_today as usize,
        })
    }

    /// Get cards updated since a timestamp (for sync)
    pub async fn get_cards_since(
        &self,
        device_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<DbCard>> {
        let cards = match since {
            Some(ts) => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT id, device_id, deck_path, question_text, answer_text,
                           question_hash, answer_hash, source_file, created_at, updated_at, deleted_at
                    FROM cards
                    WHERE device_id = $1 AND updated_at > $2
                    ORDER BY id
                    "#,
                )
                .bind(device_id)
                .bind(ts)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbCard>(
                    r#"
                    SELECT id, device_id, deck_path, question_text, answer_text,
                           question_hash, answer_hash, source_file, created_at, updated_at, deleted_at
                    FROM cards
                    WHERE device_id = $1 AND deleted_at IS NULL
                    ORDER BY id
                    "#,
                )
                .bind(device_id)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(cards)
    }

    /// Get card states updated since a timestamp (for sync)
    pub async fn get_card_states_since(
        &self,
        device_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<DbCardState>> {
        let states = match since {
            Some(ts) => {
                sqlx::query_as::<_, DbCardState>(
                    r#"
                    SELECT id, card_id, device_id, status, interval_days, ease_factor,
                           due_date, stability, difficulty, lapses, reviews_count,
                           created_at, updated_at
                    FROM card_states
                    WHERE device_id = $1 AND updated_at > $2
                    ORDER BY card_id
                    "#,
                )
                .bind(device_id)
                .bind(ts)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, DbCardState>(
                    r#"
                    SELECT id, card_id, device_id, status, interval_days, ease_factor,
                           due_date, stability, difficulty, lapses, reviews_count,
                           created_at, updated_at
                    FROM card_states
                    WHERE device_id = $1
                    ORDER BY card_id
                    "#,
                )
                .bind(device_id)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(states)
    }

    // === MD File Repository ===

    /// Upsert MD file tracking record
    pub async fn upsert_md_file(
        &self,
        device_id: Uuid,
        file_path: &str,
        s3_key: &str,
        content_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO md_files (device_id, file_path, s3_key, content_hash, uploaded_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (device_id, file_path) DO UPDATE SET
                s3_key = EXCLUDED.s3_key,
                content_hash = EXCLUDED.content_hash,
                uploaded_at = NOW()
            "#,
        )
        .bind(device_id)
        .bind(file_path)
        .bind(s3_key)
        .bind(content_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all MD files for a device
    pub async fn get_md_files(&self, device_id: Uuid) -> Result<Vec<MdFile>> {
        let files = sqlx::query_as::<_, MdFile>(
            r#"
            SELECT id, device_id, file_path, s3_key, content_hash, uploaded_at
            FROM md_files
            WHERE device_id = $1
            ORDER BY file_path
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(files)
    }

    /// Get MD file by path
    pub async fn get_md_file(&self, device_id: Uuid, file_path: &str) -> Result<Option<MdFile>> {
        let file = sqlx::query_as::<_, MdFile>(
            r#"
            SELECT id, device_id, file_path, s3_key, content_hash, uploaded_at
            FROM md_files
            WHERE device_id = $1 AND file_path = $2
            "#,
        )
        .bind(device_id)
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(file)
    }

    /// Delete MD file record
    pub async fn delete_md_file(&self, device_id: Uuid, file_path: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM md_files
            WHERE device_id = $1 AND file_path = $2
            "#,
        )
        .bind(device_id)
        .bind(file_path)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
