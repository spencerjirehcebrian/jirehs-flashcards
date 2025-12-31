mod commands;
mod db;
mod parser;
mod state;
mod sync;
mod watcher;

use commands::deck::{get_deck, import_directory, import_file, list_decks};
use commands::settings::{
    delete_deck_settings, get_deck_settings, get_effective_settings, get_global_settings,
    save_deck_settings, save_global_settings,
};
use commands::stats::{get_calendar_data, get_deck_stats, get_study_stats};
use commands::study::{compare_typed_answer, get_card, get_card_state, get_study_queue, submit_review};
use commands::sync::{
    cancel_sync, check_connectivity, confirm_orphan_deletion, get_device_status,
    get_local_sync_state, get_sync_status, register_device, skip_orphan_deletion, start_sync,
};
use commands::watcher::{get_watched_directories, start_watching, stop_watching};
use commands::SyncEngineState;
use db::SqliteRepository;
use state::AppState;
use std::path::PathBuf;

fn get_db_path() -> PathBuf {
    // Use app data directory for production, fallback to current dir
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jirehs-flashcards")
        .join("flashcards.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ensure data directory exists
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // Open database
    let repository = SqliteRepository::open(&db_path).expect("failed to open database");
    let app_state = AppState::new(repository);

    let sync_engine_state = SyncEngineState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .manage(sync_engine_state)
        .setup(|_app| {
            // File watcher will be started when the first directory is watched
            // via the start_watching command
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Deck commands
            list_decks,
            import_file,
            import_directory,
            get_deck,
            // Study commands
            get_study_queue,
            submit_review,
            get_card,
            get_card_state,
            compare_typed_answer,
            // Settings commands
            get_global_settings,
            save_global_settings,
            get_deck_settings,
            save_deck_settings,
            delete_deck_settings,
            get_effective_settings,
            // Stats commands
            get_deck_stats,
            get_study_stats,
            get_calendar_data,
            // Watcher commands
            start_watching,
            stop_watching,
            get_watched_directories,
            // Sync commands
            start_sync,
            get_sync_status,
            cancel_sync,
            confirm_orphan_deletion,
            skip_orphan_deletion,
            register_device,
            get_device_status,
            check_connectivity,
            get_local_sync_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
