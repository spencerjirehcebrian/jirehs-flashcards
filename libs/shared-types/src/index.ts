// Card status (matches Rust enum)
export type CardStatus = 'new' | 'learning' | 'review' | 'relearning';

// Algorithms
export type Algorithm = 'sm2' | 'fsrs';

// Rating scales
export type RatingScale = '4point' | '2point';

// Ratings (1-4 for 4-point scale)
export type Rating = 1 | 2 | 3 | 4;

// Answer modes
export type AnswerMode = 'flip' | 'typed';

// Matching modes
export type MatchingMode = 'exact' | 'case_insensitive' | 'fuzzy';

// Card (matches Rust Card struct - snake_case from serde)
export interface Card {
  id: number;
  deck_path: string;
  question: string;
  answer: string;
  source_file: string;
  deleted_at?: string;
}

// Card learning state (matches Rust CardState)
export interface CardState {
  status: CardStatus;
  interval_days: number;
  ease_factor: number;
  due_date?: string;
  stability?: number;
  difficulty?: number;
  lapses: number;
  reviews_count: number;
}

// Deck (matches Rust Deck)
export interface Deck {
  path: string;
  name: string;
  card_count: number;
  new_count: number;
  due_count: number;
}

// Study queue (matches Rust StudyQueue)
export interface StudyQueue {
  new_cards: Card[];
  review_cards: Card[];
  new_remaining: number;
  review_remaining: number;
}

// Review request (sent to Tauri)
export interface ReviewRequest {
  card_id: number;
  rating: Rating;
  rating_scale: RatingScale;
  answer_mode: AnswerMode;
  typed_answer?: string;
  time_taken_ms?: number;
}

// Review response (from Tauri)
export interface ReviewResponse {
  new_state: CardState;
  next_due: string;
}

// Import result (from Tauri)
export interface ImportResult {
  imported: number;
  deck_path: string;
}

// Command error (from Tauri)
export interface CommandError {
  message: string;
}

// Settings
export interface GlobalSettings {
  algorithm: Algorithm;
  rating_scale: RatingScale;
  matching_mode: MatchingMode;
  fuzzy_threshold: number;
  new_cards_per_day: number;
  reviews_per_day: number;
  daily_reset_hour: number;
}

export interface DeckSettings {
  deck_path: string;
  algorithm?: Algorithm;
  rating_scale?: RatingScale;
  matching_mode?: MatchingMode;
  fuzzy_threshold?: number;
  new_cards_per_day?: number;
  reviews_per_day?: number;
}

// Effective settings (global merged with deck overrides)
export interface EffectiveSettings {
  algorithm: Algorithm;
  rating_scale: RatingScale;
  matching_mode: MatchingMode;
  fuzzy_threshold: number;
  new_cards_per_day: number;
  reviews_per_day: number;
  daily_reset_hour: number;
}

// Diff types for answer comparison
export type DiffType = 'Same' | 'Added' | 'Removed';

export interface DiffSegment {
  text: string;
  diff_type: DiffType;
}

// Compare answer response (from Tauri)
export interface CompareAnswerResponse {
  is_correct: boolean;
  similarity: number;
  matching_mode: MatchingMode;
  typed_normalized: string;
  correct_normalized: string;
  diff: DiffSegment[];
}

// Statistics types
export interface DeckStats {
  total_cards: number;
  new_cards: number;
  learning_cards: number;
  review_cards: number;
  average_ease: number;
  average_interval: number;
}

export interface StudyStats {
  reviews_today: number;
  new_today: number;
  streak_days: number;
  retention_rate: number;
  total_reviews: number;
}

export interface CalendarData {
  date: string;
  reviews: number;
}
