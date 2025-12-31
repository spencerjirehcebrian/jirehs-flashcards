//! Statistics Tauri commands.

use crate::db::{CalendarData, DeckStats, SettingsRepository, StatsRepository, StudyStats};
use crate::state::AppState;
use tauri::State;

use super::deck::CommandError;

/// Get deck statistics.
#[tauri::command]
pub async fn get_deck_stats(
    deck_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<DeckStats, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    repo.get_deck_stats(deck_path.as_deref()).map_err(Into::into)
}

/// Get overall study statistics.
#[tauri::command]
pub async fn get_study_stats(state: State<'_, AppState>) -> Result<StudyStats, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    let settings = repo.get_global_settings()?;
    repo.get_study_stats(settings.daily_reset_hour)
        .map_err(Into::into)
}

/// Get calendar data for heatmap.
#[tauri::command]
pub async fn get_calendar_data(
    days: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<CalendarData>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    let settings = repo.get_global_settings()?;
    let days = days.unwrap_or(90);
    repo.get_calendar_data(days, settings.daily_reset_hour)
        .map_err(Into::into)
}
