//! Application state.

use crate::db::SqliteRepository;
use crate::watcher::FileWatcher;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Global application state.
pub struct AppState {
    pub repository: Arc<Mutex<SqliteRepository>>,
    pub watcher: AsyncMutex<FileWatcher>,
}

impl AppState {
    pub fn new(repository: SqliteRepository) -> Self {
        Self {
            repository: Arc::new(Mutex::new(repository)),
            watcher: AsyncMutex::new(FileWatcher::new()),
        }
    }
}
