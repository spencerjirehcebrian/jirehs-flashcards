# Jireh's Flashcards - Implementation Progress

## Scaffolding

### Root Configuration
- [x] package.json (pnpm workspace root)
- [x] pnpm-workspace.yaml
- [x] nx.json
- [x] Cargo.toml (Rust workspace)
- [x] .gitignore
- [x] tsconfig.base.json
- [x] progress.md

### Frontend App (apps/frontend)
- [x] package.json
- [x] vite.config.ts
- [x] tsconfig.json
- [x] index.html
- [x] .env.example
- [x] src/main.tsx
- [x] src/App.tsx
- [x] src/vite-env.d.ts
- [x] Folder structure (components, pages, stores, hooks, lib)

### Desktop App (apps/desktop)
- [x] package.json
- [x] src-tauri/Cargo.toml
- [x] src-tauri/tauri.conf.json
- [x] src-tauri/build.rs
- [x] src-tauri/src/main.rs
- [x] src-tauri/src/lib.rs
- [x] src-tauri/src/commands/mod.rs
- [x] src-tauri/src/db/mod.rs
- [x] src-tauri/src/sync/mod.rs
- [x] src-tauri/src/parser/mod.rs

### Backend App (apps/backend)
- [x] Cargo.toml
- [x] .env.example
- [x] src/main.rs
- [x] src/lib.rs
- [x] src/routes/mod.rs
- [x] src/models/mod.rs
- [x] src/services/mod.rs
- [x] src/services/algorithm/mod.rs (re-exports from flashcard-core)
- [x] src/services/sync.rs
- [x] src/services/storage.rs
- [x] src/db/mod.rs

### Shared Types Library (libs/shared-types)
- [x] package.json
- [x] tsconfig.json
- [x] src/index.ts

### Shared Core Library (libs/flashcard-core) - NEW
- [x] Cargo.toml
- [x] src/lib.rs
- [x] src/types.rs
- [x] src/error.rs
- [x] src/parser.rs
- [x] src/algorithm/mod.rs
- [x] src/algorithm/sm2.rs
- [x] src/algorithm/fsrs.rs
- [x] src/matching.rs (answer comparison with Levenshtein distance)

---

## Implementation (Post-Scaffolding)

### Core Features
- [x] Markdown parser (libs/flashcard-core)
- [x] SQLite local database (apps/desktop/src-tauri/src/db/)
- [x] Device token generation (backend: POST /api/device/register)
- [x] Basic study session UI

### Spaced Repetition Algorithms
- [x] SM-2 algorithm (libs/flashcard-core)
- [x] FSRS algorithm (full DSR model implementation with 17 weight parameters)
- [x] Algorithm trait/interface

### Sync Engine
- [x] MD file sync (local to cloud)
- [x] Learning state sync (pull applies cards and states to local SQLite)
- [x] Offline queue (pending_reviews table)
- [x] Conflict resolution (last-write-wins, orphan confirmation)
- [x] Settings sync (global and deck settings applied from cloud to local SQLite)

### API Endpoints
- [x] Device registration (POST /api/device/register, GET /api/device/status)
- [x] Sync pull (POST /api/sync/pull, POST /api/sync/push-reviews, POST /api/sync/confirm-delete)
- [x] Study queue (GET /api/study/queue)
- [x] Review submission (POST /api/study/review)
- [x] Settings management (GET /api/settings, PUT /api/settings/global, PUT/DELETE /api/settings/deck/:path)
- [x] Deck management (GET /api/decks, GET /api/decks/:path/stats)
- [x] Sync upload (POST /api/sync/upload with S3/R2 storage)

### Frontend Features
- [x] Deck list view
- [x] Study session (flip mode)
- [x] Study session (typed mode)
- [x] Settings page (full implementation)
- [x] Statistics view
- [x] React Router navigation
- [x] Toast notifications
- [x] Answer comparison UI with diff highlighting
- [x] Cloud sync UI in Settings page (device registration, sync status, orphan confirmation)

### Desktop Features
- [x] File watcher for MD changes (with auto-import to SQLite)
- [x] Local SQLite integration
- [x] Tauri commands (list_decks, import_file, import_directory, get_study_queue, submit_review)
- [x] Settings commands (get_global_settings, save_global_settings, get_deck_settings, save_deck_settings, delete_deck_settings, get_effective_settings)
- [x] Stats commands (get_deck_stats, get_study_stats, get_calendar_data)
- [x] Watcher commands (start_watching, stop_watching, get_watched_directories)
- [x] Typed answer comparison command (compare_typed_answer)
- [x] Sync commands (start_sync, get_sync_status, cancel_sync, confirm_orphan_deletion, skip_orphan_deletion)
- [x] Device commands (register_device, get_device_status, check_connectivity, get_local_sync_state)
- [x] Daily reset hour enforcement (date_utils module, adjusted "today" calculation for study limits)

---

## Legend
- [x] Completed
- [ ] To Do
- [~] In Progress
