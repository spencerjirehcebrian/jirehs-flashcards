-- Device registration
CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT UNIQUE NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ DEFAULT NOW()
);

-- Global ID sequence
CREATE SEQUENCE IF NOT EXISTS card_id_seq START 1;

-- Cards (parsed from MD files)
CREATE TABLE IF NOT EXISTS cards (
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
CREATE TABLE IF NOT EXISTS deck_settings (
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
CREATE TABLE IF NOT EXISTS card_states (
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
CREATE TABLE IF NOT EXISTS reviews (
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
CREATE TABLE IF NOT EXISTS global_settings (
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
CREATE TABLE IF NOT EXISTS md_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID REFERENCES devices(id),
    file_path TEXT NOT NULL,
    s3_key TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(device_id, file_path)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_cards_device_deck ON cards(device_id, deck_path);
CREATE INDEX IF NOT EXISTS idx_cards_deleted ON cards(deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_card_states_due ON card_states(device_id, due_date);
CREATE INDEX IF NOT EXISTS idx_reviews_card ON reviews(card_id);
CREATE INDEX IF NOT EXISTS idx_reviews_device_time ON reviews(device_id, reviewed_at);
