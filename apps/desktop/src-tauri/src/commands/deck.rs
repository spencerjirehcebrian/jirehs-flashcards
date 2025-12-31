//! Deck-related Tauri commands.

use crate::db::{CardRepository, DeckRepository, SettingsRepository, SqliteRepository};
use crate::state::AppState;
use flashcard_core::types::Deck;
use flashcard_core::parser;
use std::fs;
use std::path::Path;
use tauri::State;

#[derive(Debug, serde::Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub deck_path: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CommandError {
    pub message: String,
}

impl From<crate::db::DbError> for CommandError {
    fn from(e: crate::db::DbError) -> Self {
        Self { message: e.to_string() }
    }
}

impl From<flashcard_core::ParseError> for CommandError {
    fn from(e: flashcard_core::ParseError) -> Self {
        Self { message: e.to_string() }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self { message: e.to_string() }
    }
}

/// List all decks.
#[tauri::command]
pub async fn list_decks(state: State<'_, AppState>) -> Result<Vec<Deck>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    let settings = repo.get_global_settings()?;
    repo.get_all_decks(settings.daily_reset_hour)
        .map_err(Into::into)
}

/// Import a markdown file as a deck.
#[tauri::command]
pub async fn import_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, CommandError> {
    let path = Path::new(&file_path);
    let content = fs::read_to_string(path)?;
    let raw_cards = parser::parse(&content)?;

    // Derive deck path from file path
    let deck_path = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default")
        .to_string();

    let repo = state.repository.lock().expect("repository lock");
    let ids = repo.import_cards(&deck_path, &file_path, &raw_cards)?;

    Ok(ImportResult {
        imported: ids.len(),
        deck_path,
    })
}

/// Import all markdown files from a directory.
#[tauri::command]
pub async fn import_directory(
    dir_path: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, CommandError> {
    let dir = Path::new(&dir_path);
    let deck_path = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("default")
        .to_string();

    let mut total_imported = 0;
    let repo = state.repository.lock().expect("repository lock");

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "md") {
            let content = fs::read_to_string(&path)?;
            let raw_cards = parser::parse(&content)?;
            let file_path = path.to_string_lossy().to_string();
            let ids = repo.import_cards(&deck_path, &file_path, &raw_cards)?;
            total_imported += ids.len();
        }
    }

    Ok(ImportResult {
        imported: total_imported,
        deck_path,
    })
}

/// Get deck details.
#[tauri::command]
pub async fn get_deck(
    deck_path: String,
    state: State<'_, AppState>,
) -> Result<Option<Deck>, CommandError> {
    let repo = state.repository.lock().expect("repository lock");
    let settings = repo.get_global_settings()?;
    repo.get_deck(&deck_path, settings.daily_reset_hour)
        .map_err(Into::into)
}
