//! Sync-related Tauri commands.

use std::fs;
use std::path::Path;
use tauri::State;
use tokio::sync::Mutex;

use crate::db::{CardRepository, LocalDeviceInfo, LocalSyncState, StateRepository, SyncRepository};
use crate::state::AppState;
use crate::sync::{ApiDeckSettings, ApiGlobalSettings, SyncEngine, SyncStats, SyncStatus};
use flashcard_core::types::{Card, CardState};

/// Command error type for sync operations.
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    pub message: String,
}

impl CommandError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn database(e: impl std::fmt::Display) -> Self {
        Self::new(format!("Database error: {}", e))
    }
}

/// Sync engine state wrapper.
pub struct SyncEngineState {
    engine: Mutex<Option<SyncEngine>>,
}

impl SyncEngineState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
        }
    }
}

/// Start a sync operation.
#[tauri::command]
pub async fn start_sync(
    backend_url: String,
    watched_dirs: Vec<String>,
    state: State<'_, AppState>,
    sync_state: State<'_, SyncEngineState>,
) -> Result<SyncStatus, CommandError> {
    // Collect all MD files from watched directories (sync operation, no await)
    let mut md_files: Vec<(String, String)> = Vec::new();

    for dir_path in &watched_dirs {
        let path = Path::new(dir_path);
        if path.is_dir() {
            collect_md_files(path, path, &mut md_files)?;
        }
    }

    // Create or get sync engine - hold lock only briefly
    let engine = {
        let mut engine_guard = sync_state.engine.lock().await;
        if engine_guard.is_none() {
            *engine_guard = Some(SyncEngine::new(backend_url.clone()));
        }
        // Clone the Arc-based engine internals so we can release the lock
        engine_guard.as_ref().unwrap().clone()
    };

    // Get device token from repo - hold lock only briefly (sync operation)
    let device_token = {
        let repo = state.repository.lock().expect("repository lock");
        repo.get_device_token()
            .map_err(|e| CommandError::database(e))?
    };

    let device_info = device_token.ok_or_else(|| CommandError::new("Not authenticated - please register device first"))?;

    // Run sync with the cloned engine (no MutexGuard held)
    match engine.sync(&device_info.token, md_files, || {
        // Callback to get pending reviews - locks repo briefly
        let repo = state.repository.lock().expect("repository lock");
        repo.get_pending_reviews().unwrap_or_default()
    }, |ids| {
        // Callback to mark reviews synced - locks repo briefly
        let repo = state.repository.lock().expect("repository lock");
        let _ = repo.mark_reviews_synced(ids);
    }, || {
        // Callback to get sync state
        let repo = state.repository.lock().expect("repository lock");
        repo.get_sync_state().ok()
    }, |timestamp| {
        // Callback to update sync state
        let repo = state.repository.lock().expect("repository lock");
        let _ = repo.update_sync_state(timestamp);
    }, |cards: &[Card], synced_at: &str| {
        // Callback to apply pulled cards
        let repo = state.repository.lock().expect("repository lock");
        repo.upsert_cards_from_sync(cards, synced_at).unwrap_or(0)
    }, |states: &[(i64, CardState)]| {
        // Callback to apply pulled card states
        let repo = state.repository.lock().expect("repository lock");
        repo.save_card_states_synced(states).unwrap_or(0)
    }, |global_settings: &ApiGlobalSettings| {
        // Callback to apply global settings from cloud
        let repo = state.repository.lock().expect("repository lock");
        let _ = repo.save_global_settings_synced(global_settings);
    }, |deck_settings: &[ApiDeckSettings]| {
        // Callback to apply deck settings from cloud
        let repo = state.repository.lock().expect("repository lock");
        for settings in deck_settings {
            let _ = repo.save_deck_settings_synced(settings);
        }
    }).await {
        Ok(_stats) => Ok(engine.status().await),
        Err(e) => {
            Ok(SyncStatus::Failed {
                error: e.to_string(),
            })
        }
    }
}

/// Get current sync status.
#[tauri::command]
pub async fn get_sync_status(
    sync_state: State<'_, SyncEngineState>,
) -> Result<SyncStatus, CommandError> {
    let engine_guard = sync_state.engine.lock().await;

    match engine_guard.as_ref() {
        Some(engine) => Ok(engine.status().await),
        None => Ok(SyncStatus::Idle),
    }
}

/// Cancel ongoing sync.
#[tauri::command]
pub async fn cancel_sync(
    sync_state: State<'_, SyncEngineState>,
) -> Result<(), CommandError> {
    let mut engine_guard = sync_state.engine.lock().await;
    *engine_guard = None;
    Ok(())
}

/// Confirm orphan deletion.
#[tauri::command]
pub async fn confirm_orphan_deletion(
    card_ids: Vec<i64>,
    state: State<'_, AppState>,
    sync_state: State<'_, SyncEngineState>,
) -> Result<usize, CommandError> {
    // Get engine clone
    let engine = {
        let engine_guard = sync_state.engine.lock().await;
        engine_guard
            .as_ref()
            .ok_or_else(|| CommandError::new("No sync in progress"))?
            .clone()
    };

    // Get device token - hold lock briefly
    let device_info = {
        let repo = state.repository.lock().expect("repository lock");
        repo.get_device_token()
            .map_err(|e| CommandError::database(e))?
            .ok_or_else(|| CommandError::new("Not authenticated"))?
    };

    // Now do async operation without holding any guards
    let deleted_count = engine
        .confirm_orphan_deletion(&device_info.token, card_ids)
        .await
        .map_err(|e| CommandError::new(e.to_string()))?;

    Ok(deleted_count)
}

/// Skip orphan deletion (keep orphaned cards).
#[tauri::command]
pub async fn skip_orphan_deletion(
    state: State<'_, AppState>,
    sync_state: State<'_, SyncEngineState>,
) -> Result<SyncStats, CommandError> {
    // Get engine clone
    let engine = {
        let engine_guard = sync_state.engine.lock().await;
        engine_guard
            .as_ref()
            .ok_or_else(|| CommandError::new("No sync in progress"))?
            .clone()
    };

    // Get device token - hold lock briefly
    let device_info = {
        let repo = state.repository.lock().expect("repository lock");
        repo.get_device_token()
            .map_err(|e| CommandError::database(e))?
            .ok_or_else(|| CommandError::new("Not authenticated"))?
    };

    // Continue sync without deleting orphans
    let stats = engine
        .continue_sync_without_orphans(&device_info.token, || {
            let repo = state.repository.lock().expect("repository lock");
            repo.get_pending_reviews().unwrap_or_default()
        }, |ids| {
            let repo = state.repository.lock().expect("repository lock");
            let _ = repo.mark_reviews_synced(ids);
        }, || {
            let repo = state.repository.lock().expect("repository lock");
            repo.get_sync_state().ok()
        }, |timestamp| {
            let repo = state.repository.lock().expect("repository lock");
            let _ = repo.update_sync_state(timestamp);
        }, |cards: &[Card], synced_at: &str| {
            let repo = state.repository.lock().expect("repository lock");
            repo.upsert_cards_from_sync(cards, synced_at).unwrap_or(0)
        }, |states: &[(i64, CardState)]| {
            let repo = state.repository.lock().expect("repository lock");
            repo.save_card_states_synced(states).unwrap_or(0)
        }, |global_settings: &ApiGlobalSettings| {
            let repo = state.repository.lock().expect("repository lock");
            let _ = repo.save_global_settings_synced(global_settings);
        }, |deck_settings: &[ApiDeckSettings]| {
            let repo = state.repository.lock().expect("repository lock");
            for settings in deck_settings {
                let _ = repo.save_deck_settings_synced(settings);
            }
        })
        .await
        .map_err(|e| CommandError::new(e.to_string()))?;

    Ok(stats)
}

/// Register device with backend.
#[tauri::command]
pub async fn register_device(
    backend_url: String,
    device_name: Option<String>,
    state: State<'_, AppState>,
    sync_state: State<'_, SyncEngineState>,
) -> Result<LocalDeviceInfo, CommandError> {
    // Create or get sync engine
    let engine = {
        let mut engine_guard = sync_state.engine.lock().await;
        if engine_guard.is_none() {
            *engine_guard = Some(SyncEngine::new(backend_url.clone()));
        }
        engine_guard.as_ref().unwrap().clone()
    };

    // Register device (async operation)
    let (token, device_id) = engine
        .register_device(device_name)
        .await
        .map_err(|e| CommandError::new(e.to_string()))?;

    // Save to local database - hold lock briefly
    {
        let repo = state.repository.lock().expect("repository lock");
        repo.save_device_token(&token, &device_id)
            .map_err(|e| CommandError::database(e))?;
    }

    Ok(LocalDeviceInfo {
        token,
        device_id: Some(device_id),
    })
}

/// Get device registration status.
#[tauri::command]
pub async fn get_device_status(
    state: State<'_, AppState>,
) -> Result<Option<LocalDeviceInfo>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    let device_info = repo
        .get_device_token()
        .map_err(|e| CommandError::database(e))?;
    Ok(device_info)
}

/// Check backend connectivity.
#[tauri::command]
pub async fn check_connectivity(
    backend_url: String,
) -> Result<bool, CommandError> {
    let engine = SyncEngine::new(backend_url);
    engine
        .check_connectivity()
        .await
        .map_err(|e| CommandError::new(e.to_string()))
}

/// Get local sync state.
#[tauri::command]
pub async fn get_local_sync_state(
    state: State<'_, AppState>,
) -> Result<LocalSyncState, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_sync_state()
        .map_err(|e| CommandError::database(e))
}

// === Helper functions ===

/// Collect all .md files from a directory recursively.
fn collect_md_files(
    base_path: &Path,
    current_path: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), CommandError> {
    if current_path.is_dir() {
        for entry in fs::read_dir(current_path)
            .map_err(|e| CommandError::new(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry.map_err(|e| CommandError::new(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                collect_md_files(base_path, &path, files)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let relative_path = path
                    .strip_prefix(base_path)
                    .map_err(|e| CommandError::new(e.to_string()))?
                    .to_string_lossy()
                    .to_string();

                let content = fs::read_to_string(&path)
                    .map_err(|e| CommandError::new(format!("Failed to read file: {}", e)))?;

                files.push((relative_path, content));
            }
        }
    }
    Ok(())
}
