import type {
  CalendarData,
  Card,
  CardState,
  CompareAnswerResponse,
  Deck,
  DeckSettings,
  DeckStats,
  DiffSegment,
  EffectiveSettings,
  GlobalSettings,
  StudyQueue,
  StudyStats,
} from '@jirehs-flashcards/shared-types';
import type {
  DeviceInfo,
  LocalSyncState,
  OrphanInfo,
  SyncStats,
  SyncStatus,
} from '../lib/tauri';

let idCounter = 1;

// Helper to generate unique IDs
function nextId(): number {
  return idCounter++;
}

// Reset ID counter (useful for tests that need predictable IDs)
export function resetIdCounter(start = 1) {
  idCounter = start;
}

// Deck factory
export function createMockDeck(overrides: Partial<Deck> = {}): Deck {
  return {
    path: `/decks/deck-${nextId()}`,
    name: `Test Deck ${idCounter}`,
    card_count: 10,
    new_count: 5,
    due_count: 3,
    ...overrides,
  };
}

// Card factory
export function createMockCard(overrides: Partial<Card> = {}): Card {
  const id = nextId();
  return {
    id,
    deck_path: '/decks/test-deck',
    question: `Question ${id}?`,
    answer: `Answer ${id}`,
    source_file: `/decks/test-deck/card-${id}.md`,
    ...overrides,
  };
}

// Card state factory
export function createMockCardState(overrides: Partial<CardState> = {}): CardState {
  return {
    status: 'new',
    interval_days: 0,
    ease_factor: 2.5,
    lapses: 0,
    reviews_count: 0,
    ...overrides,
  };
}

// Study queue factory
export function createMockStudyQueue(overrides: Partial<StudyQueue> = {}): StudyQueue {
  return {
    new_cards: [],
    review_cards: [],
    new_remaining: 0,
    review_remaining: 0,
    ...overrides,
  };
}

// Global settings factory
export function createMockGlobalSettings(
  overrides: Partial<GlobalSettings> = {}
): GlobalSettings {
  return {
    algorithm: 'sm2',
    rating_scale: '4point',
    matching_mode: 'exact',
    fuzzy_threshold: 0.8,
    new_cards_per_day: 20,
    reviews_per_day: 200,
    daily_reset_hour: 4,
    ...overrides,
  };
}

// Deck settings factory
export function createMockDeckSettings(
  overrides: Partial<DeckSettings> = {}
): DeckSettings {
  return {
    deck_path: '/decks/test-deck',
    ...overrides,
  };
}

// Effective settings factory
export function createMockEffectiveSettings(
  overrides: Partial<EffectiveSettings> = {}
): EffectiveSettings {
  return {
    algorithm: 'sm2',
    rating_scale: '4point',
    matching_mode: 'exact',
    fuzzy_threshold: 0.8,
    new_cards_per_day: 20,
    reviews_per_day: 200,
    daily_reset_hour: 4,
    ...overrides,
  };
}

// Deck stats factory
export function createMockDeckStats(overrides: Partial<DeckStats> = {}): DeckStats {
  return {
    total_cards: 100,
    new_cards: 20,
    learning_cards: 10,
    review_cards: 70,
    average_ease: 2.5,
    average_interval: 21,
    ...overrides,
  };
}

// Study stats factory
export function createMockStudyStats(overrides: Partial<StudyStats> = {}): StudyStats {
  return {
    reviews_today: 50,
    new_today: 10,
    streak_days: 7,
    retention_rate: 0.85,
    total_reviews: 1000,
    ...overrides,
  };
}

// Calendar data factory
export function createMockCalendarData(
  days = 7,
  overrides: Partial<CalendarData>[] = []
): CalendarData[] {
  const data: CalendarData[] = [];
  const today = new Date();

  for (let i = 0; i < days; i++) {
    const date = new Date(today);
    date.setDate(date.getDate() - i);
    data.push({
      date: date.toISOString().split('T')[0],
      reviews: Math.floor(Math.random() * 50),
      ...overrides[i],
    });
  }

  return data;
}

// Compare answer response factory
export function createMockCompareAnswerResponse(
  overrides: Partial<CompareAnswerResponse> = {}
): CompareAnswerResponse {
  return {
    is_correct: true,
    similarity: 1.0,
    matching_mode: 'exact',
    typed_normalized: 'answer',
    correct_normalized: 'answer',
    diff: [{ text: 'answer', diff_type: 'Same' }],
    ...overrides,
  };
}

// Diff segment factory
export function createMockDiffSegment(
  overrides: Partial<DiffSegment> = {}
): DiffSegment {
  return {
    text: 'text',
    diff_type: 'Same',
    ...overrides,
  };
}

// Sync status factory
export function createMockSyncStatus(
  type: SyncStatus['type'] = 'Idle',
  overrides: Partial<Omit<SyncStatus, 'type'>> = {}
): SyncStatus {
  const base: SyncStatus = { type };

  switch (type) {
    case 'Syncing':
      return {
        ...base,
        stage: { name: 'Connecting' },
        progress: 0,
        ...overrides,
      };
    case 'AwaitingOrphanConfirmation':
      return {
        ...base,
        orphans: [],
        ...overrides,
      };
    case 'Completed':
      return {
        ...base,
        synced_at: new Date().toISOString(),
        stats: createMockSyncStats(),
        ...overrides,
      };
    case 'Failed':
      return {
        ...base,
        error: 'Sync failed',
        ...overrides,
      };
    default:
      return { ...base, ...overrides };
  }
}

// Sync stats factory
export function createMockSyncStats(overrides: Partial<SyncStats> = {}): SyncStats {
  return {
    files_uploaded: 0,
    cards_created: 0,
    cards_updated: 0,
    orphans_deleted: 0,
    reviews_synced: 0,
    states_pulled: 0,
    ...overrides,
  };
}

// Orphan info factory
export function createMockOrphanInfo(overrides: Partial<OrphanInfo> = {}): OrphanInfo {
  const id = nextId();
  return {
    card_id: id,
    question_preview: `Orphaned question ${id}?`,
    ...overrides,
  };
}

// Device info factory
export function createMockDeviceInfo(overrides: Partial<DeviceInfo> = {}): DeviceInfo {
  return {
    token: 'test-token-123',
    device_id: 'device-123',
    ...overrides,
  };
}

// Local sync state factory
export function createMockLocalSyncState(
  overrides: Partial<LocalSyncState> = {}
): LocalSyncState {
  return {
    last_sync_at: null,
    pending_changes: 0,
    ...overrides,
  };
}
