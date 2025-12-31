import { invoke } from '@tauri-apps/api/core';
import type {
  CalendarData,
  Card,
  CardState,
  CompareAnswerResponse,
  Deck,
  DeckSettings,
  DeckStats,
  EffectiveSettings,
  GlobalSettings,
  ImportResult,
  ReviewRequest,
  ReviewResponse,
  StudyQueue,
  StudyStats,
} from '@jirehs-flashcards/shared-types';

export const tauri = {
  // Deck commands
  listDecks: () => invoke<Deck[]>('list_decks'),
  getDeck: (deckPath: string) => invoke<Deck | null>('get_deck', { deckPath }),
  importFile: (filePath: string) => invoke<ImportResult>('import_file', { filePath }),
  importDirectory: (dirPath: string) => invoke<ImportResult>('import_directory', { dirPath }),

  // Study commands
  getStudyQueue: (deckPath?: string) => invoke<StudyQueue>('get_study_queue', { deckPath }),
  submitReview: (request: ReviewRequest) => invoke<ReviewResponse>('submit_review', { request }),
  getCard: (cardId: number) => invoke<Card | null>('get_card', { cardId }),
  getCardState: (cardId: number) => invoke<CardState | null>('get_card_state', { cardId }),
  compareTypedAnswer: (typedAnswer: string, correctAnswer: string, deckPath?: string) =>
    invoke<CompareAnswerResponse>('compare_typed_answer', { typedAnswer, correctAnswer, deckPath }),

  // Settings commands
  getGlobalSettings: () => invoke<GlobalSettings>('get_global_settings'),
  saveGlobalSettings: (settings: GlobalSettings) =>
    invoke<void>('save_global_settings', { settings }),
  getDeckSettings: (deckPath: string) =>
    invoke<DeckSettings | null>('get_deck_settings', { deckPath }),
  saveDeckSettings: (settings: DeckSettings) =>
    invoke<void>('save_deck_settings', { settings }),
  deleteDeckSettings: (deckPath: string) =>
    invoke<void>('delete_deck_settings', { deckPath }),
  getEffectiveSettings: (deckPath?: string) =>
    invoke<EffectiveSettings>('get_effective_settings', { deckPath }),

  // Stats commands
  getDeckStats: (deckPath?: string) =>
    invoke<DeckStats>('get_deck_stats', { deckPath }),
  getStudyStats: () => invoke<StudyStats>('get_study_stats'),
  getCalendarData: (days?: number) =>
    invoke<CalendarData[]>('get_calendar_data', { days }),

  // File watcher commands
  startWatching: (dirPath: string) => invoke<void>('start_watching', { dirPath }),
  stopWatching: (dirPath: string) => invoke<void>('stop_watching', { dirPath }),
  getWatchedDirectories: () => invoke<string[]>('get_watched_directories'),

  // Sync commands
  startSync: (backendUrl: string, watchedDirs: string[]) =>
    invoke<SyncStatus>('start_sync', { backendUrl, watchedDirs }),
  getSyncStatus: () => invoke<SyncStatus>('get_sync_status'),
  cancelSync: () => invoke<void>('cancel_sync'),
  confirmOrphanDeletion: (cardIds: number[]) =>
    invoke<number>('confirm_orphan_deletion', { cardIds }),
  skipOrphanDeletion: () => invoke<SyncStats>('skip_orphan_deletion'),
  registerDevice: (backendUrl: string, deviceName?: string) =>
    invoke<DeviceInfo>('register_device', { backendUrl, deviceName }),
  getDeviceStatus: () => invoke<DeviceInfo | null>('get_device_status'),
  checkConnectivity: (backendUrl: string) =>
    invoke<boolean>('check_connectivity', { backendUrl }),
  getLocalSyncState: () => invoke<LocalSyncState>('get_local_sync_state'),
};

// Sync types
export interface SyncStatus {
  type: 'Idle' | 'Syncing' | 'AwaitingOrphanConfirmation' | 'Completed' | 'Failed';
  stage?: SyncStage;
  progress?: number;
  orphans?: OrphanInfo[];
  synced_at?: string;
  stats?: SyncStats;
  error?: string;
}

export interface SyncStage {
  name: 'Connecting' | 'UploadingFiles' | 'ParsingCards' | 'ReceivingUpdates' |
        'PushingReviews' | 'PullingState' | 'ApplyingChanges' | 'WritingFiles';
  current?: number;
  total?: number;
  count?: number;
}

export interface SyncStats {
  files_uploaded: number;
  cards_created: number;
  cards_updated: number;
  orphans_deleted: number;
  reviews_synced: number;
  states_pulled: number;
}

export interface OrphanInfo {
  card_id: number;
  question_preview: string;
}

export interface DeviceInfo {
  token: string;
  device_id: string | null;
}

export interface LocalSyncState {
  last_sync_at: string | null;
  pending_changes: number;
}
