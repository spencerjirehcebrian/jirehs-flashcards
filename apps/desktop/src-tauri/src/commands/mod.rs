//! Tauri commands exposed to the frontend.

pub mod deck;
pub mod settings;
pub mod stats;
pub mod study;
pub mod sync;
pub mod watcher;

pub use deck::{get_deck, import_directory, import_file, list_decks};
pub use settings::{
    delete_deck_settings, get_deck_settings, get_effective_settings, get_global_settings,
    save_deck_settings, save_global_settings,
};
pub use stats::{get_calendar_data, get_deck_stats, get_study_stats};
pub use study::{compare_typed_answer, get_card, get_card_state, get_study_queue, submit_review};
pub use sync::{
    cancel_sync, check_connectivity, confirm_orphan_deletion, get_device_status,
    get_local_sync_state, get_sync_status, register_device, skip_orphan_deletion, start_sync,
    SyncEngineState,
};
pub use watcher::{get_watched_directories, start_watching, stop_watching};
