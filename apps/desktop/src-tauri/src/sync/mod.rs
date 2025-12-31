//! Sync engine for cloud synchronization.

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::{LocalSyncState, PendingReview};
use flashcard_core::types::{Card, CardState, CardStatus};

/// Sync errors.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Backend error: {status} - {message}")]
    Backend { status: u16, message: String },

    #[error("Database error: {0}")]
    Database(String),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Not authenticated - please register device first")]
    NotAuthenticated,

    #[error("Sync already in progress")]
    AlreadyInProgress,

    #[error("Sync cancelled by user")]
    Cancelled,

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Sync status for UI.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SyncStatus {
    Idle,
    Syncing { stage: SyncStage, progress: f32 },
    AwaitingOrphanConfirmation { orphans: Vec<OrphanInfo> },
    Completed { synced_at: String, stats: SyncStats },
    Failed { error: String },
}

/// Current sync stage.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "name")]
pub enum SyncStage {
    Connecting,
    UploadingFiles { current: usize, total: usize },
    ParsingCards,
    ReceivingUpdates,
    PushingReviews { count: usize },
    PullingState,
    ApplyingChanges,
    WritingFiles { current: usize, total: usize },
}

/// Sync statistics.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SyncStats {
    pub files_uploaded: usize,
    pub cards_created: usize,
    pub cards_updated: usize,
    pub orphans_deleted: usize,
    pub reviews_synced: usize,
    pub states_pulled: usize,
}

/// Orphan card info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanInfo {
    pub card_id: i64,
    pub question_preview: String,
}

// === API Request/Response Types ===

#[derive(Debug, Serialize)]
struct SyncUploadRequest {
    files: Vec<SyncFile>,
}

#[derive(Debug, Serialize)]
struct SyncFile {
    path: String,
    content: String,
    hash: String,
}

#[derive(Debug, Deserialize)]
struct SyncUploadResponse {
    updated_files: Vec<UpdatedFile>,
    new_ids: Vec<NewIdAssignment>,
    orphaned_cards: Vec<OrphanedCard>,
}

#[derive(Debug, Clone, Deserialize)]
struct UpdatedFile {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct NewIdAssignment {
    #[allow(dead_code)]
    path: String,
    #[allow(dead_code)]
    line: usize,
    #[allow(dead_code)]
    id: i64,
}

#[derive(Debug, Deserialize)]
struct OrphanedCard {
    id: i64,
    question_preview: String,
}

#[derive(Debug, Serialize)]
struct PushReviewsRequest {
    reviews: Vec<ReviewSubmission>,
}

#[derive(Debug, Serialize)]
struct ReviewSubmission {
    card_id: i64,
    reviewed_at: DateTime<Utc>,
    rating: i32,
    rating_scale: String,
    answer_mode: String,
    typed_answer: Option<String>,
    was_correct: Option<bool>,
    time_taken_ms: Option<i32>,
    interval_before: f64,
    interval_after: f64,
    ease_before: f64,
    ease_after: f64,
    algorithm: String,
}

#[derive(Debug, Deserialize)]
struct PushReviewsResponse {
    synced_count: usize,
}

#[derive(Debug, Serialize)]
struct SyncPullRequest {
    last_sync_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct SyncPullResponse {
    cards: Vec<ApiCard>,
    card_states: Vec<ApiCardState>,
    settings: SyncedSettings,
}

#[derive(Debug, Deserialize)]
struct ApiCard {
    id: i64,
    deck_path: String,
    question: String,
    answer: String,
    source_file: String,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct ApiCardState {
    card_id: i64,
    status: String,
    interval_days: f64,
    ease_factor: f64,
    stability: Option<f64>,
    difficulty: Option<f64>,
    lapses: u32,
    reviews_count: u32,
    due_date: Option<DateTime<Utc>>,
}

/// Global settings from API.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiGlobalSettings {
    pub algorithm: String,
    pub rating_scale: String,
    pub matching_mode: String,
    pub fuzzy_threshold: f64,
    pub new_cards_per_day: u32,
    pub reviews_per_day: u32,
    pub daily_reset_hour: u32,
}

/// Deck settings from API.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiDeckSettings {
    pub deck_path: String,
    pub algorithm: Option<String>,
    pub rating_scale: Option<String>,
    pub matching_mode: Option<String>,
    pub fuzzy_threshold: Option<f64>,
    pub new_cards_per_day: Option<u32>,
    pub reviews_per_day: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SyncedSettings {
    global: ApiGlobalSettings,
    decks: Vec<ApiDeckSettings>,
}

#[derive(Debug, Serialize)]
struct ConfirmDeleteRequest {
    card_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct ConfirmDeleteResponse {
    deleted_count: usize,
}

#[derive(Debug, Serialize)]
struct DeviceRegisterRequest {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceRegisterResponse {
    device_id: String,
    token: String,
}

/// Inner state shared across clones.
struct SyncEngineInner {
    client: Client,
    backend_url: String,
    status: Mutex<SyncStatus>,
    stats: Mutex<SyncStats>,
    pending_updated_files: Mutex<Vec<UpdatedFile>>,
}

/// Sync engine for managing cloud synchronization.
///
/// This struct is Clone-able because it wraps all state in Arc.
/// This allows it to be used across async boundaries without holding locks.
#[derive(Clone)]
pub struct SyncEngine {
    inner: Arc<SyncEngineInner>,
}

impl SyncEngine {
    /// Create a new sync engine.
    pub fn new(backend_url: String) -> Self {
        Self {
            inner: Arc::new(SyncEngineInner {
                client: Client::new(),
                backend_url: backend_url.trim_end_matches('/').to_string(),
                status: Mutex::new(SyncStatus::Idle),
                stats: Mutex::new(SyncStats::default()),
                pending_updated_files: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Get current sync status.
    pub async fn status(&self) -> SyncStatus {
        self.inner.status.lock().await.clone()
    }

    /// Check if backend is reachable.
    pub async fn check_connectivity(&self) -> Result<bool, SyncError> {
        let url = format!("{}/health", self.inner.backend_url);
        match self.inner.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(SyncError::Network(e.to_string())),
        }
    }

    /// Register a new device with the backend.
    pub async fn register_device(
        &self,
        name: Option<String>,
    ) -> Result<(String, String), SyncError> {
        let url = format!("{}/api/device/register", self.inner.backend_url);
        let request = DeviceRegisterRequest { name };

        let resp = self
            .inner
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SyncError::Backend { status, message });
        }

        let response: DeviceRegisterResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Parse(e.to_string()))?;

        Ok((response.token, response.device_id))
    }

    /// Run full sync operation.
    ///
    /// Uses callbacks for database operations to avoid holding MutexGuard across await points.
    pub async fn sync<F1, F2, F3, F4, F5, F6, F7, F8>(
        &self,
        token: &str,
        md_files: Vec<(String, String)>,
        get_pending_reviews: F1,
        mark_reviews_synced: F2,
        get_sync_state: F3,
        update_sync_state: F4,
        apply_cards_from_sync: F5,
        apply_states_from_sync: F6,
        apply_global_settings: F7,
        apply_deck_settings: F8,
    ) -> Result<SyncStats, SyncError>
    where
        F1: Fn() -> Vec<PendingReview> + Send + Sync,
        F2: Fn(&[i64]) + Send + Sync,
        F3: Fn() -> Option<LocalSyncState> + Send + Sync,
        F4: Fn(&str) + Send + Sync,
        F5: Fn(&[Card], &str) -> usize + Send + Sync,
        F6: Fn(&[(i64, CardState)]) -> usize + Send + Sync,
        F7: Fn(&ApiGlobalSettings) + Send + Sync,
        F8: Fn(&[ApiDeckSettings]) + Send + Sync,
    {
        // Check if sync already in progress
        {
            let current = self.inner.status.lock().await;
            if matches!(*current, SyncStatus::Syncing { .. }) {
                return Err(SyncError::AlreadyInProgress);
            }
        }

        // Reset stats
        *self.inner.stats.lock().await = SyncStats::default();

        // Update status: Connecting
        self.set_status(SyncStatus::Syncing {
            stage: SyncStage::Connecting,
            progress: 0.0,
        })
        .await;

        // 1. Check connectivity
        if !self.check_connectivity().await? {
            self.set_status(SyncStatus::Failed {
                error: "Backend not reachable".to_string(),
            })
            .await;
            return Err(SyncError::Network("Backend not reachable".to_string()));
        }

        // 2. Upload files
        self.set_status(SyncStatus::Syncing {
            stage: SyncStage::UploadingFiles {
                current: 0,
                total: md_files.len(),
            },
            progress: 0.1,
        })
        .await;

        let upload_result = self.upload_files(token, &md_files).await?;

        {
            let mut stats = self.inner.stats.lock().await;
            stats.files_uploaded = md_files.len();
            stats.cards_created = upload_result.new_ids.len();
        }

        // 3. Check for orphans - if any, pause for user confirmation
        if !upload_result.orphaned_cards.is_empty() {
            let orphans: Vec<OrphanInfo> = upload_result
                .orphaned_cards
                .iter()
                .map(|o| OrphanInfo {
                    card_id: o.id,
                    question_preview: o.question_preview.clone(),
                })
                .collect();

            // Store updated files for later
            *self.inner.pending_updated_files.lock().await = upload_result.updated_files;

            self.set_status(SyncStatus::AwaitingOrphanConfirmation { orphans })
                .await;

            // Sync will be resumed by confirm_orphan_deletion or skip_orphan_deletion
            return Ok(self.inner.stats.lock().await.clone());
        }

        // 4. Continue with sync
        self.continue_sync_internal(
            token,
            &upload_result.updated_files,
            get_pending_reviews,
            mark_reviews_synced,
            get_sync_state,
            update_sync_state,
            apply_cards_from_sync,
            apply_states_from_sync,
            apply_global_settings,
            apply_deck_settings,
        )
        .await
    }

    /// Continue sync after orphan confirmation (without orphan deletion).
    pub async fn continue_sync_without_orphans<F1, F2, F3, F4, F5, F6, F7, F8>(
        &self,
        token: &str,
        get_pending_reviews: F1,
        mark_reviews_synced: F2,
        get_sync_state: F3,
        update_sync_state: F4,
        apply_cards_from_sync: F5,
        apply_states_from_sync: F6,
        apply_global_settings: F7,
        apply_deck_settings: F8,
    ) -> Result<SyncStats, SyncError>
    where
        F1: Fn() -> Vec<PendingReview> + Send + Sync,
        F2: Fn(&[i64]) + Send + Sync,
        F3: Fn() -> Option<LocalSyncState> + Send + Sync,
        F4: Fn(&str) + Send + Sync,
        F5: Fn(&[Card], &str) -> usize + Send + Sync,
        F6: Fn(&[(i64, CardState)]) -> usize + Send + Sync,
        F7: Fn(&ApiGlobalSettings) + Send + Sync,
        F8: Fn(&[ApiDeckSettings]) + Send + Sync,
    {
        let updated_files = self.inner.pending_updated_files.lock().await.clone();
        self.continue_sync_internal(
            token,
            &updated_files,
            get_pending_reviews,
            mark_reviews_synced,
            get_sync_state,
            update_sync_state,
            apply_cards_from_sync,
            apply_states_from_sync,
            apply_global_settings,
            apply_deck_settings,
        )
        .await
    }

    /// Internal continue sync implementation.
    async fn continue_sync_internal<F1, F2, F3, F4, F5, F6, F7, F8>(
        &self,
        token: &str,
        updated_files: &[UpdatedFile],
        get_pending_reviews: F1,
        mark_reviews_synced: F2,
        get_sync_state: F3,
        update_sync_state: F4,
        apply_cards_from_sync: F5,
        apply_states_from_sync: F6,
        apply_global_settings: F7,
        apply_deck_settings: F8,
    ) -> Result<SyncStats, SyncError>
    where
        F1: Fn() -> Vec<PendingReview> + Send + Sync,
        F2: Fn(&[i64]) + Send + Sync,
        F3: Fn() -> Option<LocalSyncState> + Send + Sync,
        F4: Fn(&str) + Send + Sync,
        F5: Fn(&[Card], &str) -> usize + Send + Sync,
        F6: Fn(&[(i64, CardState)]) -> usize + Send + Sync,
        F7: Fn(&ApiGlobalSettings) + Send + Sync,
        F8: Fn(&[ApiDeckSettings]) + Send + Sync,
    {
        // 3. Push pending reviews
        self.set_status(SyncStatus::Syncing {
            stage: SyncStage::PushingReviews { count: 0 },
            progress: 0.4,
        })
        .await;

        let pending_reviews = get_pending_reviews();

        if !pending_reviews.is_empty() {
            let synced_count = self.push_reviews(token, &pending_reviews).await?;

            // Mark reviews as synced
            let ids: Vec<i64> = pending_reviews.iter().map(|r| r.id).collect();
            mark_reviews_synced(&ids);

            self.inner.stats.lock().await.reviews_synced = synced_count;
        }

        // 4. Pull state
        self.set_status(SyncStatus::Syncing {
            stage: SyncStage::PullingState,
            progress: 0.6,
        })
        .await;

        let sync_state = get_sync_state();

        let last_sync = sync_state.and_then(|s| {
            s.last_sync_at.and_then(|ts| {
                DateTime::parse_from_rfc3339(&ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            })
        });

        let pull_response = self.pull_state(token, last_sync).await?;

        self.inner.stats.lock().await.states_pulled = pull_response.card_states.len();

        // 5. Apply pulled state to local DB
        self.set_status(SyncStatus::Syncing {
            stage: SyncStage::ApplyingChanges,
            progress: 0.8,
        })
        .await;

        // Convert and apply pulled cards
        let now = Utc::now().to_rfc3339();
        if !pull_response.cards.is_empty() {
            let cards: Vec<Card> = pull_response
                .cards
                .iter()
                .map(|c| Card {
                    id: c.id,
                    deck_path: c.deck_path.clone(),
                    question: c.question.clone(),
                    answer: c.answer.clone(),
                    source_file: c.source_file.clone(),
                    deleted_at: c.deleted_at,
                })
                .collect();
            let applied = apply_cards_from_sync(&cards, &now);
            self.inner.stats.lock().await.cards_updated += applied;
        }

        // Convert and apply pulled card states
        if !pull_response.card_states.is_empty() {
            let states: Vec<(i64, CardState)> = pull_response
                .card_states
                .iter()
                .map(|s| {
                    let status = match s.status.as_str() {
                        "learning" => CardStatus::Learning,
                        "review" => CardStatus::Review,
                        "relearning" => CardStatus::Relearning,
                        _ => CardStatus::New,
                    };
                    (
                        s.card_id,
                        CardState {
                            status,
                            interval_days: s.interval_days,
                            ease_factor: s.ease_factor,
                            stability: s.stability,
                            difficulty: s.difficulty,
                            lapses: s.lapses,
                            reviews_count: s.reviews_count,
                            due_date: s.due_date,
                        },
                    )
                })
                .collect();
            apply_states_from_sync(&states);
        }

        // Apply pulled settings
        apply_global_settings(&pull_response.settings.global);
        if !pull_response.settings.decks.is_empty() {
            apply_deck_settings(&pull_response.settings.decks);
        }

        // 6. Write updated files to disk
        if !updated_files.is_empty() {
            self.set_status(SyncStatus::Syncing {
                stage: SyncStage::WritingFiles {
                    current: 0,
                    total: updated_files.len(),
                },
                progress: 0.9,
            })
            .await;

            // Note: File writing should be handled by the caller
            // since we need to know the base directory
        }

        // 7. Update sync state
        let now = Utc::now().to_rfc3339();
        update_sync_state(&now);

        let stats = self.inner.stats.lock().await.clone();

        self.set_status(SyncStatus::Completed {
            synced_at: now,
            stats: stats.clone(),
        })
        .await;

        Ok(stats)
    }

    /// Confirm orphan deletion.
    pub async fn confirm_orphan_deletion(
        &self,
        token: &str,
        card_ids: Vec<i64>,
    ) -> Result<usize, SyncError> {
        let url = format!("{}/api/sync/confirm-delete", self.inner.backend_url);
        let request = ConfirmDeleteRequest { card_ids };

        let resp = self
            .inner
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SyncError::Backend { status, message });
        }

        let response: ConfirmDeleteResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Parse(e.to_string()))?;

        self.inner.stats.lock().await.orphans_deleted = response.deleted_count;

        Ok(response.deleted_count)
    }

    // === Private methods ===

    async fn set_status(&self, status: SyncStatus) {
        *self.inner.status.lock().await = status;
    }

    async fn upload_files(
        &self,
        token: &str,
        files: &[(String, String)],
    ) -> Result<SyncUploadResponse, SyncError> {
        let url = format!("{}/api/sync/upload", self.inner.backend_url);

        let sync_files: Vec<SyncFile> = files
            .iter()
            .map(|(path, content)| SyncFile {
                path: path.clone(),
                content: content.clone(),
                hash: hash_content(content),
            })
            .collect();

        let request = SyncUploadRequest { files: sync_files };

        let resp = self
            .inner
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SyncError::Backend { status, message });
        }

        resp.json()
            .await
            .map_err(|e| SyncError::Parse(e.to_string()))
    }

    async fn push_reviews(
        &self,
        token: &str,
        reviews: &[PendingReview],
    ) -> Result<usize, SyncError> {
        let url = format!("{}/api/sync/push-reviews", self.inner.backend_url);

        let submissions: Vec<ReviewSubmission> = reviews
            .iter()
            .map(|r| ReviewSubmission {
                card_id: r.card_id,
                reviewed_at: DateTime::parse_from_rfc3339(&r.reviewed_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                rating: r.rating,
                rating_scale: r.rating_scale.clone(),
                answer_mode: r.answer_mode.clone(),
                typed_answer: r.typed_answer.clone(),
                was_correct: r.was_correct,
                time_taken_ms: r.time_taken_ms,
                interval_before: r.interval_before,
                interval_after: r.interval_after,
                ease_before: r.ease_before,
                ease_after: r.ease_after,
                algorithm: r.algorithm.clone(),
            })
            .collect();

        let request = PushReviewsRequest { reviews: submissions };

        let resp = self
            .inner
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SyncError::Backend { status, message });
        }

        let response: PushReviewsResponse = resp
            .json()
            .await
            .map_err(|e| SyncError::Parse(e.to_string()))?;

        Ok(response.synced_count)
    }

    async fn pull_state(
        &self,
        token: &str,
        last_sync_at: Option<DateTime<Utc>>,
    ) -> Result<SyncPullResponse, SyncError> {
        let url = format!("{}/api/sync/pull", self.inner.backend_url);
        let request = SyncPullRequest { last_sync_at };

        let resp = self
            .inner
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&request)
            .send()
            .await
            .map_err(|e| SyncError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(SyncError::Backend { status, message });
        }

        resp.json()
            .await
            .map_err(|e| SyncError::Parse(e.to_string()))
    }
}

/// Calculate SHA256 hash of content.
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
