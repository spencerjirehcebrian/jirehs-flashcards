//! SQLite schema definitions.

/// Current schema version for migrations.
pub const SCHEMA_VERSION: i32 = 1;

/// Complete schema for local SQLite database.
pub const SCHEMA: &str = r#"
-- Local device info
CREATE TABLE IF NOT EXISTS local_device (
    token TEXT PRIMARY KEY,
    device_id TEXT
);

-- Cards (cached from cloud or parsed from local files)
CREATE TABLE IF NOT EXISTS cards (
    id INTEGER PRIMARY KEY,
    deck_path TEXT NOT NULL,
    question_text TEXT NOT NULL,
    answer_text TEXT NOT NULL,
    source_file TEXT NOT NULL,
    deleted_at TEXT,
    synced_at TEXT
);

-- Card learning state
CREATE TABLE IF NOT EXISTS card_states (
    card_id INTEGER PRIMARY KEY REFERENCES cards(id),
    status TEXT NOT NULL DEFAULT 'new',
    interval_days REAL NOT NULL DEFAULT 0,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    due_date TEXT,
    stability REAL,
    difficulty REAL,
    lapses INTEGER NOT NULL DEFAULT 0,
    reviews_count INTEGER NOT NULL DEFAULT 0,
    synced INTEGER NOT NULL DEFAULT 1
);

-- Pending reviews (to sync)
CREATE TABLE IF NOT EXISTS pending_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id INTEGER REFERENCES cards(id),
    reviewed_at TEXT NOT NULL,
    rating INTEGER NOT NULL,
    rating_scale TEXT NOT NULL,
    answer_mode TEXT NOT NULL,
    typed_answer TEXT,
    was_correct INTEGER,
    time_taken_ms INTEGER,
    interval_before REAL,
    interval_after REAL,
    ease_before REAL,
    ease_after REAL,
    algorithm TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0
);

-- Deck settings (cached)
CREATE TABLE IF NOT EXISTS deck_settings (
    deck_path TEXT PRIMARY KEY,
    algorithm TEXT,
    rating_scale TEXT,
    matching_mode TEXT,
    fuzzy_threshold REAL,
    new_cards_per_day INTEGER,
    reviews_per_day INTEGER,
    synced INTEGER NOT NULL DEFAULT 1
);

-- Global settings
CREATE TABLE IF NOT EXISTS global_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    algorithm TEXT NOT NULL DEFAULT 'sm2',
    rating_scale TEXT NOT NULL DEFAULT '4point',
    matching_mode TEXT NOT NULL DEFAULT 'fuzzy',
    fuzzy_threshold REAL NOT NULL DEFAULT 0.8,
    new_cards_per_day INTEGER NOT NULL DEFAULT 20,
    reviews_per_day INTEGER NOT NULL DEFAULT 200,
    daily_reset_hour INTEGER NOT NULL DEFAULT 0,
    synced INTEGER NOT NULL DEFAULT 1
);

-- MD file sync state
CREATE TABLE IF NOT EXISTS md_files (
    file_path TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    last_modified TEXT NOT NULL,
    pending_upload INTEGER NOT NULL DEFAULT 0
);

-- Sync metadata
CREATE TABLE IF NOT EXISTS sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_sync_at TEXT,
    pending_changes INTEGER NOT NULL DEFAULT 0
);

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_cards_deck ON cards(deck_path);
CREATE INDEX IF NOT EXISTS idx_cards_deleted ON cards(deleted_at);
CREATE INDEX IF NOT EXISTS idx_card_states_due ON card_states(due_date);
CREATE INDEX IF NOT EXISTS idx_pending_reviews_synced ON pending_reviews(synced);
"#;

/// Initialize global settings if not exists.
pub const INIT_GLOBAL_SETTINGS: &str = r#"
INSERT OR IGNORE INTO global_settings (id) VALUES (1);
"#;

/// Initialize sync state if not exists.
pub const INIT_SYNC_STATE: &str = r#"
INSERT OR IGNORE INTO sync_state (id, pending_changes) VALUES (1, 0);
"#;
