//! Spaced repetition algorithm implementations.
//!
//! Re-exports from flashcard-core for backward compatibility.

pub use flashcard_core::algorithm::{
    fsrs, get_algorithm, sm2, SchedulingResult, SpacedRepetitionAlgorithm,
};
pub use flashcard_core::types::{CardState, CardStatus, Rating};
