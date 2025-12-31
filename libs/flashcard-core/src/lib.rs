//! Core flashcard library shared by desktop and backend applications.
//!
//! Provides:
//! - Markdown parser for flashcard files
//! - Spaced repetition algorithm implementations (SM-2, FSRS)
//! - Answer matching for typed mode (Levenshtein distance)
//! - Shared types (Card, CardState, Rating, etc.)

pub mod algorithm;
pub mod error;
pub mod matching;
pub mod parser;
pub mod types;

pub use algorithm::{SchedulingResult, SpacedRepetitionAlgorithm};
pub use error::{ParseError, Result};
pub use matching::{compare_answers, levenshtein_distance, normalized_similarity, word_diff, DiffSegment, DiffType, MatchResult};
pub use parser::parse;
pub use types::{
    Algorithm, Card, CardState, CardStatus, DeckSettings, EffectiveSettings, GlobalSettings,
    MatchingMode, Rating, RatingScale, RawCard,
};
