# Jireh's Flashcards - Technical Specification

## 1. Overview

### 1.1 Vision

A desktop flashcard application that uses markdown files as the source of truth for card content, with cloud sync for cross-device access and spaced repetition for optimized learning.

### 1.2 Goals

- Markdown-first: Users author flashcards in plain markdown files
- Offline-first: Full functionality without internet, sync when online
- Cross-device: Study on any device with synced progress
- Extensible algorithms: Support multiple spaced repetition algorithms
- Local ownership: Users control their data (markdown files are theirs)

### 1.3 Non-Goals (MVP)

- User authentication (device token only)
- Multi-user/collaboration
- Web application
- Mobile application
- Test coverage
- Observability/monitoring
- Production hardening

---

## 2. Tech Stack

### 2.1 Desktop Application

| Component | Technology |
|-----------|------------|
| Framework | Tauri v2 |
| Frontend | React 18 + TypeScript |
| Build Tool | Vite |
| Client State | Zustand |
| Server State | TanStack Query |
| Local Database | SQLite (via Tauri) |

### 2.2 Cloud Backend

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Framework | Axum |
| Database | PostgreSQL |
| File Storage | S3/R2 |

### 2.3 Development

| Component | Technology |
|-----------|------------|
| Monorepo | Nx Workspaces |
| Package Manager | pnpm |
| Rust Toolchain | cargo (workspace) |

---

## 3. Architecture

### 3.1 Storage Tiers

```
┌─────────────────────────────────────────────────────────────────────┐
│                           TAURI APP                                 │
│                                                                     │
│  ┌──────────────┐    ┌──────────────┐    ┌───────────────────────┐ │
│  │  Local MD    │───▶│ Local SQLite │───▶│     Sync Engine       │ │
│  │  Directory   │    │  (offline)   │    │ (queue + reconcile)   │ │
│  └──────────────┘    └──────────────┘    └───────────┬───────────┘ │
│         │                                            │             │
└─────────│────────────────────────────────────────────│─────────────┘
          │ user edits                                 │ online
          ▼                                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         CLOUD BACKEND                               │
│                                                                     │
│  ┌──────────────┐              ┌──────────────┐                     │
│  │  PostgreSQL  │◀────────────▶│    S3/R2     │                     │
│  │  (canonical) │   references │  (MD files)  │                     │
│  └──────────────┘              └──────────────┘                     │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 Storage Responsibilities

| Tier | Contents | Purpose |
|------|----------|---------|
| Local MD Files | Flashcard content (Q&A) | User's source of truth, version controllable |
| Local SQLite | Cards, learning state, sync queue | Offline functionality, pending sync |
| Cloud Postgres | Cards, learning state, deck settings, device registry | Canonical database, cross-device sync |
| Cloud S3/R2 | MD file backups | Cross-device MD access, backup |

### 3.3 Sync Flow

#### MD File Sync (Local to Cloud)

1. User edits local MD files
2. User triggers sync from frontend
3. Tauri reads changed MD files
4. Tauri uploads to backend API
5. Backend parses MD, generates IDs for new cards
6. Backend stores MD in S3/R2
7. Backend updates Postgres (card registry)
8. Backend returns updated MD (with generated IDs)
9. Tauri writes updated MD back to local disk

#### Learning State Sync

1. User studies offline, reviews stored in SQLite with `synced = false`
2. User comes online
3. Sync engine pushes pending reviews to cloud Postgres
4. Cloud Postgres applies reviews (last-write-wins)
5. Sync engine pulls latest state from cloud
6. SQLite updated to match cloud state

### 3.4 Offline Mode

When offline, the app:

- Reads card content from local SQLite (cached from last sync)
- Writes reviews to local SQLite with pending sync flag
- Queues MD file changes for upload
- Fully functional for studying

When back online:

- Pending reviews pushed to cloud
- MD changes uploaded
- Local state reconciled with cloud

### 3.5 Identity

- Device token generated on first launch
- Stored locally in secure storage
- Sent with all API requests
- User data isolated by device token
- Future: proper auth will link multiple devices to one account

---

## 4. Markdown Format

### 4.1 Syntax

Cards are defined using delimiters. `ID:` marks the start of a new card.

```markdown
ID: 1
Q: What is Rust's ownership model?
A: Each value has exactly one owner. When the owner goes out of scope, the value is dropped.

ID: 2
Q: What are the three rules of ownership?
A: 1. Each value has exactly one owner
2. There can only be one owner at a time
3. When the owner goes out of scope, the value is dropped

ID: 3
Q: Explain borrowing in Rust
A: Borrowing allows you to reference a value without taking ownership.

There are two types:
- Immutable borrows (`&T`): Multiple allowed
- Mutable borrows (`&mut T`): Only one allowed

Example:
```rust
fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);
    println!("{} has length {}", s, len);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}
```
```

### 4.2 Parsing Rules

1. `ID:` on a new line starts a new card
2. `Q:` marks the question (required)
3. `A:` marks the answer (required)
4. Content continues until the next `ID:` or end of file
5. Multi-line content is preserved (including code blocks)
6. Leading/trailing whitespace is trimmed from Q and A content

### 4.3 ID Generation

- IDs are global numeric auto-increment
- Generated by the backend on first sync
- Once assigned, IDs are immutable
- Cards can be moved between decks without ID conflicts
- Sequence stored in Postgres

### 4.4 Cards Without IDs

When a card has no ID (new card):

```markdown
Q: What is a closure?
A: A function that captures its environment.

ID: 5
Q: What is a trait?
A: A collection of methods defined for an unknown type.
```

On sync, the backend assigns an ID:

```markdown
ID: 6
Q: What is a closure?
A: A function that captures its environment.

ID: 5
Q: What is a trait?
A: A collection of methods defined for an unknown type.
```

---

## 5. Data Models

### 5.1 Cloud Postgres Schema

```sql
-- Device registration
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT UNIQUE NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ DEFAULT NOW()
);

-- Global ID sequence
CREATE SEQUENCE card_id_seq START 1;

-- Cards (parsed from MD files)
CREATE TABLE cards (
    id BIGINT PRIMARY KEY DEFAULT nextval('card_id_seq'),
    device_id UUID REFERENCES devices(id),
    deck_path TEXT NOT NULL,
    question_text TEXT NOT NULL,
    answer_text TEXT NOT NULL,
    question_hash TEXT NOT NULL,
    answer_hash TEXT NOT NULL,
    source_file TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

-- Deck settings (overrides global defaults)
CREATE TABLE deck_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID REFERENCES devices(id),
    deck_path TEXT NOT NULL,
    algorithm TEXT,
    rating_scale TEXT,
    matching_mode TEXT,
    fuzzy_threshold REAL,
    new_cards_per_day INT,
    reviews_per_day INT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(device_id, deck_path)
);

-- Learning state per card
CREATE TABLE card_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id BIGINT REFERENCES cards(id),
    device_id UUID REFERENCES devices(id),
    status TEXT NOT NULL DEFAULT 'new',
    interval_days REAL NOT NULL DEFAULT 0,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    due_date DATE,
    stability REAL,
    difficulty REAL,
    lapses INT NOT NULL DEFAULT 0,
    reviews_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(card_id, device_id)
);

-- Review history (append-only)
CREATE TABLE reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id BIGINT REFERENCES cards(id),
    device_id UUID REFERENCES devices(id),
    reviewed_at TIMESTAMPTZ NOT NULL,
    rating INT NOT NULL,
    rating_scale TEXT NOT NULL,
    answer_mode TEXT NOT NULL,
    typed_answer TEXT,
    was_correct BOOLEAN,
    time_taken_ms INT,
    interval_before REAL,
    interval_after REAL,
    ease_before REAL,
    ease_after REAL,
    algorithm TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Global settings per device
CREATE TABLE global_settings (
    device_id UUID PRIMARY KEY REFERENCES devices(id),
    algorithm TEXT NOT NULL DEFAULT 'sm2',
    rating_scale TEXT NOT NULL DEFAULT '4point',
    matching_mode TEXT NOT NULL DEFAULT 'fuzzy',
    fuzzy_threshold REAL NOT NULL DEFAULT 0.8,
    new_cards_per_day INT NOT NULL DEFAULT 20,
    reviews_per_day INT NOT NULL DEFAULT 200,
    daily_reset_hour INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- MD file storage references
CREATE TABLE md_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID REFERENCES devices(id),
    file_path TEXT NOT NULL,
    s3_key TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(device_id, file_path)
);

-- Indexes
CREATE INDEX idx_cards_device_deck ON cards(device_id, deck_path);
CREATE INDEX idx_cards_deleted ON cards(deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX idx_card_states_due ON card_states(device_id, due_date);
CREATE INDEX idx_reviews_card ON reviews(card_id);
CREATE INDEX idx_reviews_device_time ON reviews(device_id, reviewed_at);
```

### 5.2 Local SQLite Schema

Mirrors Postgres but with sync metadata:

```sql
-- Local device info
CREATE TABLE local_device (
    token TEXT PRIMARY KEY,
    device_id TEXT
);

-- Cards (cached from cloud)
CREATE TABLE cards (
    id INTEGER PRIMARY KEY,
    deck_path TEXT NOT NULL,
    question_text TEXT NOT NULL,
    answer_text TEXT NOT NULL,
    source_file TEXT NOT NULL,
    deleted_at TEXT,
    synced_at TEXT
);

-- Card learning state
CREATE TABLE card_states (
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
CREATE TABLE pending_reviews (
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
CREATE TABLE deck_settings (
    deck_path TEXT PRIMARY KEY,
    algorithm TEXT,
    rating_scale TEXT,
    matching_mode TEXT,
    fuzzy_threshold REAL,
    new_cards_per_day INTEGER,
    reviews_per_day INTEGER,
    synced INTEGER NOT NULL DEFAULT 1
);

-- Global settings (cached)
CREATE TABLE global_settings (
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
CREATE TABLE md_files (
    file_path TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    last_modified TEXT NOT NULL,
    pending_upload INTEGER NOT NULL DEFAULT 0
);

-- Sync metadata
CREATE TABLE sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_sync_at TEXT,
    pending_changes INTEGER NOT NULL DEFAULT 0
);
```

### 5.3 Enums / Constants

```typescript
// Card status
type CardStatus = 'new' | 'learning' | 'review' | 'relearning';

// Algorithms
type Algorithm = 'sm2' | 'fsrs';

// Rating scales
type RatingScale = '4point' | '2point';

// 4-point ratings
type Rating4 = 1 | 2 | 3 | 4; // again, hard, good, easy

// 2-point ratings
type Rating2 = 1 | 2; // wrong, correct

// Answer modes
type AnswerMode = 'flip' | 'typed';

// Matching modes (for typed)
type MatchingMode = 'exact' | 'case_insensitive' | 'fuzzy';
```

---

## 6. Spaced Repetition Algorithms

### 6.1 Algorithm Trait

```rust
pub trait SpacedRepetitionAlgorithm: Send + Sync {
    /// Returns algorithm identifier
    fn name(&self) -> &'static str;

    /// Calculate next review state after a review
    fn schedule(
        &self,
        state: &CardState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> SchedulingResult;

    /// Initial state for a new card
    fn initial_state(&self) -> CardState;
}

pub struct SchedulingResult {
    pub new_state: CardState,
    pub next_due: DateTime<Utc>,
}

pub struct CardState {
    pub status: CardStatus,
    pub interval_days: f64,
    pub ease_factor: f64,
    pub stability: Option<f64>,
    pub difficulty: Option<f64>,
    pub lapses: u32,
    pub reviews_count: u32,
}
```

### 6.2 SM-2 Algorithm

The default algorithm, based on SuperMemo 2.

**Parameters:**
- Initial ease factor: 2.5
- Minimum ease: 1.3
- Easy bonus: 1.3
- Hard interval: 1.2
- Learning steps: [1min, 10min]
- Graduating interval: 1 day
- Easy interval: 4 days

**Rating Effects (4-point):**

| Rating | Name | Effect |
|--------|------|--------|
| 1 | Again | Reset to learning, ease -0.2 |
| 2 | Hard | Interval * 1.2, ease -0.15 |
| 3 | Good | Interval * ease |
| 4 | Easy | Interval * ease * easy_bonus, ease +0.15 |

**Interval Calculation:**

```
if status == 'new' or status == 'learning':
    if rating >= 3:
        graduate to 'review' with graduating_interval (or easy_interval if rating == 4)
    else:
        stay in learning, next step

if status == 'review':
    if rating == 1:
        lapse, move to 'relearning', interval = 1 day
    else:
        new_interval = current_interval * modifier * ease
        new_ease = ease + ease_adjustment[rating]
```

### 6.3 FSRS Algorithm

Modern algorithm based on memory research. Uses DSR model (Difficulty, Stability, Retrievability).

**Parameters:**
- w: [0.4, 0.6, 2.4, 5.8, 4.93, 0.94, 0.86, 0.01, 1.49, 0.14, 0.94, 2.18, 0.05, 0.34, 1.26, 0.29, 2.61]
- Request retention: 0.9 (target 90% retention)
- Maximum interval: 36500 days

**State Variables:**
- Stability (S): Days until retention drops to 90%
- Difficulty (D): Card difficulty 0-1

**Core Formulas:**

```
Retrievability: R = (1 + t / (9 * S))^-1
Next Stability: S' = S * (e^w[8] * (11 - D) * S^-w[9] * (e^(w[10] * (1 - R)) - 1) * w[15 or 16 or 17])
Next Difficulty: D' = w[7] * D_0(G) + (1 - w[7]) * D
```

### 6.4 Algorithm Selection

- Global default stored in `global_settings`
- Per-deck override stored in `deck_settings`
- Algorithm used is recorded with each review for accurate replay

---

## 7. Features

### 7.1 Study Modes

#### Flip Mode
1. Show question
2. User mentally recalls answer
3. User clicks to reveal answer
4. User rates recall quality

#### Typed Mode
1. Show question
2. User types answer
3. System compares to correct answer
4. Show correct answer and match result
5. User rates (or auto-rate based on match)

### 7.2 Answer Matching (Typed Mode)

| Mode | Behavior |
|------|----------|
| Exact | Must match character-for-character |
| Case Insensitive | Ignore case differences |
| Fuzzy | Levenshtein similarity >= threshold (default 80%) |

Fuzzy matching uses normalized Levenshtein distance:

```
similarity = 1 - (levenshtein_distance / max(len(a), len(b)))
match = similarity >= threshold
```

### 7.3 Rating Scales

**4-Point Scale (default):**
| Rating | Label | Meaning |
|--------|-------|---------|
| 1 | Again | Complete failure, need to relearn |
| 2 | Hard | Recalled with significant difficulty |
| 3 | Good | Recalled with some effort |
| 4 | Easy | Recalled instantly |

**2-Point Scale:**
| Rating | Label | Meaning |
|--------|-------|---------|
| 1 | Wrong | Did not recall |
| 2 | Correct | Recalled |

Mapping 2-point to algorithm:
- Wrong = Rating 1 (Again)
- Correct = Rating 3 (Good)

### 7.4 Deck Management

**Hierarchy:**
- Folder = Deck
- Nested folder = Subdeck
- Path determines hierarchy: `programming/rust/ownership`

**Operations:**
- View all decks
- View deck statistics
- Configure deck settings (override globals)
- Sync deck from local files

### 7.5 Study Limits

| Setting | Default | Description |
|---------|---------|-------------|
| new_cards_per_day | 20 | Max new cards introduced daily |
| reviews_per_day | 200 | Max reviews per day |
| daily_reset_hour | 0 | Hour (0-23) when daily counts reset (local time) |

Configurable globally and per-deck.

### 7.6 Orphan Detection

When syncing, if a card ID exists in the database but not in the MD files:

1. Mark card as orphan
2. Prompt user: "X cards were removed from your files. Delete from database?"
3. User confirms → soft delete (set `deleted_at`)
4. User cancels → cards remain (will be flagged again next sync)

Soft-deleted cards:
- Excluded from study sessions
- Review history preserved
- Can be restored by re-adding to MD files with same ID

---

## 8. API Design

### 8.1 Authentication

All requests include device token in header:

```
Authorization: Bearer <device_token>
```

First request with unknown token creates new device.

### 8.2 Endpoints

#### Device

```
POST /api/device/register
  Response: { device_id, token }

GET /api/device/status
  Response: { device_id, last_sync_at, pending_changes }
```

#### Sync

```
POST /api/sync/upload
  Body: { files: [{ path, content, hash }] }
  Response: {
    updated_files: [{ path, content }],
    new_ids: [{ path, line, id }],
    orphaned_cards: [{ id, question_preview }]
  }

POST /api/sync/confirm-delete
  Body: { card_ids: [1, 2, 3] }
  Response: { deleted_count }

POST /api/sync/pull
  Body: { last_sync_at }
  Response: {
    cards: [...],
    card_states: [...],
    settings: {...}
  }

POST /api/sync/push-reviews
  Body: { reviews: [...] }
  Response: { synced_count }
```

#### Study

```
GET /api/study/queue?deck_path=<path>
  Response: {
    new_cards: [...],
    review_cards: [...],
    limits: { new_remaining, review_remaining }
  }

POST /api/study/review
  Body: {
    card_id,
    rating,
    rating_scale,
    answer_mode,
    typed_answer?,
    time_taken_ms
  }
  Response: {
    next_state: {...},
    next_due
  }
```

#### Settings

```
GET /api/settings
  Response: { global: {...}, decks: {...} }

PUT /api/settings/global
  Body: { algorithm?, rating_scale?, ... }
  Response: { updated: {...} }

PUT /api/settings/deck/:path
  Body: { algorithm?, rating_scale?, ... }
  Response: { updated: {...} }
```

#### Decks

```
GET /api/decks
  Response: {
    decks: [{
      path,
      name,
      card_count,
      new_count,
      due_count,
      subdecks: [...]
    }]
  }

GET /api/decks/:path/stats
  Response: {
    total_cards,
    new_cards,
    learning_cards,
    review_cards,
    average_ease,
    retention_rate,
    reviews_today
  }
```

---

## 9. Development Setup

### 9.1 Project Structure

```
jirehs-flashcards/
├── apps/
│   ├── desktop/                 # Tauri app
│   │   ├── src/                 # Rust Tauri code
│   │   ├── src-tauri/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── main.rs
│   │   │       ├── commands/    # Tauri commands
│   │   │       ├── db/          # SQLite operations
│   │   │       ├── sync/        # Sync engine
│   │   │       └── parser/      # MD parser
│   │   └── index.html
│   │
│   ├── frontend/                # React app (shared)
│   │   ├── src/
│   │   │   ├── components/
│   │   │   ├── pages/
│   │   │   ├── stores/          # Zustand stores
│   │   │   ├── hooks/           # TanStack Query hooks
│   │   │   └── lib/
│   │   ├── package.json
│   │   └── vite.config.ts
│   │
│   └── backend/                 # Cloud API
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── routes/
│           ├── models/
│           ├── services/
│           │   ├── algorithm/
│           │   │   ├── mod.rs
│           │   │   ├── sm2.rs
│           │   │   └── fsrs.rs
│           │   ├── sync.rs
│           │   └── storage.rs
│           └── db/
│
├── libs/
│   └── shared-types/            # Shared TypeScript types
│       ├── src/
│       └── package.json
│
├── nx.json
├── package.json
├── pnpm-workspace.yaml
├── Cargo.toml                   # Rust workspace
└── spec.md
```

### 9.2 Nx Configuration

```json
// nx.json
{
  "targetDefaults": {
    "build": {
      "dependsOn": ["^build"],
      "cache": true
    },
    "dev": {
      "cache": false
    }
  },
  "namedInputs": {
    "default": ["{projectRoot}/**/*"],
    "production": ["default", "!{projectRoot}/**/*.spec.ts"]
  }
}
```

### 9.3 Workspace Commands

```bash
# Install dependencies
pnpm install

# Development (hot reload)
pnpm nx run desktop:dev        # Tauri + Vite dev mode
pnpm nx run backend:dev        # Backend with cargo-watch
pnpm nx run-many -t dev        # All in parallel

# Build
pnpm nx run desktop:build
pnpm nx run backend:build

# Individual targets
pnpm nx run frontend:dev       # Just the React app
pnpm nx run frontend:build
```

### 9.4 Hot Reload Setup

**Frontend (Vite):**
- Built-in HMR
- Configured in `vite.config.ts`

**Tauri:**
- Uses Vite dev server for frontend
- Rust changes require restart (cargo-watch optional)

**Backend:**
- cargo-watch for auto-reload:
  ```toml
  # .cargo/config.toml
  [alias]
  dev = "watch -x run"
  ```

### 9.5 Environment Variables

```bash
# apps/backend/.env
DATABASE_URL=postgres://user:pass@host:5432/flashcards
S3_BUCKET=flashcards-md
S3_REGION=auto
S3_ENDPOINT=https://xxx.r2.cloudflarestorage.com
S3_ACCESS_KEY=xxx
S3_SECRET_KEY=xxx

# apps/desktop/.env
VITE_API_URL=http://localhost:3000
```

---

## 10. Future Considerations

### 10.1 Planned Features (Post-MVP)

- **Web Application**: Share frontend code, browser-based access
- **Mobile Application**: React Native or Tauri Mobile
- **User Authentication**: Email/password, OAuth
- **Multi-device Linking**: One account, many devices
- **Optional Card Fields**: Tags, difficulty hints, notes
- **Import/Export**: Anki deck import, CSV export
- **Statistics Dashboard**: Retention graphs, study heatmap
- **Collaboration**: Shared decks, deck publishing

### 10.2 Schema Migrations

When adding optional fields to MD format:

```markdown
ID: 1
Q: What is Rust?
A: A systems programming language.
tags: rust, programming
difficulty: easy
notes: Review chapter 1
```

Parser should ignore unknown fields for forward compatibility.

### 10.3 Algorithm Additions

New algorithms implement the `SpacedRepetitionAlgorithm` trait:

- Leitner System
- Custom user-defined intervals
- AI-powered scheduling

### 10.4 Performance Considerations

- Card content indexed for search (future)
- Batch sync for large collections
- Incremental MD parsing (only changed files)
- SQLite WAL mode for concurrent access

---

## Appendix A: SM-2 Reference Implementation

```rust
pub struct Sm2 {
    pub initial_ease: f64,
    pub minimum_ease: f64,
    pub easy_bonus: f64,
    pub hard_multiplier: f64,
    pub graduating_interval: f64,
    pub easy_interval: f64,
}

impl Default for Sm2 {
    fn default() -> Self {
        Self {
            initial_ease: 2.5,
            minimum_ease: 1.3,
            easy_bonus: 1.3,
            hard_multiplier: 1.2,
            graduating_interval: 1.0,
            easy_interval: 4.0,
        }
    }
}

impl SpacedRepetitionAlgorithm for Sm2 {
    fn name(&self) -> &'static str {
        "sm2"
    }

    fn initial_state(&self) -> CardState {
        CardState {
            status: CardStatus::New,
            interval_days: 0.0,
            ease_factor: self.initial_ease,
            stability: None,
            difficulty: None,
            lapses: 0,
            reviews_count: 0,
        }
    }

    fn schedule(
        &self,
        state: &CardState,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> SchedulingResult {
        let rating_value = rating.to_4point_value();

        let (new_status, new_interval, new_ease) = match state.status {
            CardStatus::New | CardStatus::Learning => {
                if rating_value >= 3 {
                    let interval = if rating_value == 4 {
                        self.easy_interval
                    } else {
                        self.graduating_interval
                    };
                    (CardStatus::Review, interval, state.ease_factor)
                } else {
                    (CardStatus::Learning, 0.0, state.ease_factor)
                }
            }
            CardStatus::Review | CardStatus::Relearning => {
                if rating_value == 1 {
                    (
                        CardStatus::Relearning,
                        1.0,
                        (state.ease_factor - 0.2).max(self.minimum_ease),
                    )
                } else {
                    let ease_adj = match rating_value {
                        2 => -0.15,
                        3 => 0.0,
                        4 => 0.15,
                        _ => 0.0,
                    };
                    let multiplier = match rating_value {
                        2 => self.hard_multiplier,
                        4 => state.ease_factor * self.easy_bonus,
                        _ => state.ease_factor,
                    };
                    let new_interval = (state.interval_days * multiplier).max(1.0);
                    let new_ease = (state.ease_factor + ease_adj).max(self.minimum_ease);
                    (CardStatus::Review, new_interval, new_ease)
                }
            }
        };

        let next_due = now + chrono::Duration::days(new_interval.ceil() as i64);

        SchedulingResult {
            new_state: CardState {
                status: new_status,
                interval_days: new_interval,
                ease_factor: new_ease,
                stability: None,
                difficulty: None,
                lapses: if rating_value == 1 { state.lapses + 1 } else { state.lapses },
                reviews_count: state.reviews_count + 1,
            },
            next_due,
        }
    }
}
```

---

## Appendix B: MD Parser Pseudocode

```
function parse_md_file(content: string): Card[]
    cards = []
    current_card = null
    current_field = null
    buffer = []

    for line in content.lines():
        if line.starts_with("ID:"):
            if current_card != null:
                flush_buffer(current_card, current_field, buffer)
                cards.push(current_card)
            current_card = new Card()
            current_card.id = parse_id(line)
            current_field = null
            buffer = []

        else if line.starts_with("Q:"):
            flush_buffer(current_card, current_field, buffer)
            current_field = "question"
            buffer = [line.after("Q:").trim()]

        else if line.starts_with("A:"):
            flush_buffer(current_card, current_field, buffer)
            current_field = "answer"
            buffer = [line.after("A:").trim()]

        else:
            buffer.push(line)

    if current_card != null:
        flush_buffer(current_card, current_field, buffer)
        cards.push(current_card)

    return cards

function flush_buffer(card, field, buffer):
    if field == "question":
        card.question = buffer.join("\n").trim()
    else if field == "answer":
        card.answer = buffer.join("\n").trim()
```
