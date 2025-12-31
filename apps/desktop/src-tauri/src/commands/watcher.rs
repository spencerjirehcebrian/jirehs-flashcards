//! File watcher commands.

use crate::state::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, State};

/// Start watching a directory for file changes.
#[tauri::command]
pub async fn start_watching(
    dir_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = PathBuf::from(&dir_path);

    if !path.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path));
    }

    let mut watcher = state.watcher.lock().await;

    // Initialize watcher if not already started
    if !watcher.is_started() {
        watcher.start(app_handle, state.repository.clone())?;
    }

    watcher.watch(path)
}

/// Stop watching a directory.
#[tauri::command]
pub async fn stop_watching(dir_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = PathBuf::from(&dir_path);
    let mut watcher = state.watcher.lock().await;
    watcher.unwatch(&path)
}

/// Get the list of currently watched directories.
#[tauri::command]
pub async fn get_watched_directories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let watcher = state.watcher.lock().await;
    Ok(watcher.get_watched_directories())
}
