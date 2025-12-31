//! Settings-related Tauri commands.

use crate::db::SettingsRepository;
use crate::state::AppState;
use flashcard_core::types::{DeckSettings, EffectiveSettings, GlobalSettings};
use tauri::State;

use super::deck::CommandError;

/// Get global settings.
#[tauri::command]
pub async fn get_global_settings(
    state: State<'_, AppState>,
) -> Result<GlobalSettings, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_global_settings().map_err(Into::into)
}

/// Save global settings.
#[tauri::command]
pub async fn save_global_settings(
    settings: GlobalSettings,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.save_global_settings(&settings).map_err(Into::into)
}

/// Get deck-specific settings.
#[tauri::command]
pub async fn get_deck_settings(
    deck_path: String,
    state: State<'_, AppState>,
) -> Result<Option<DeckSettings>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_deck_settings(&deck_path).map_err(Into::into)
}

/// Save deck-specific settings.
#[tauri::command]
pub async fn save_deck_settings(
    settings: DeckSettings,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.save_deck_settings(&settings).map_err(Into::into)
}

/// Delete deck-specific settings (revert to global).
#[tauri::command]
pub async fn delete_deck_settings(
    deck_path: String,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.delete_deck_settings(&deck_path).map_err(Into::into)
}

/// Get effective settings for a deck (global merged with deck overrides).
#[tauri::command]
pub async fn get_effective_settings(
    deck_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<EffectiveSettings, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_effective_settings(deck_path.as_deref())
        .map_err(Into::into)
}
