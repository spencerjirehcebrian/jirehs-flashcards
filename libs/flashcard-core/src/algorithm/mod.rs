//! Spaced repetition algorithm implementations.

pub mod fsrs;
pub mod sm2;

use crate::types::{CardState, Rating};
use chrono::{DateTime, Utc};

/// Result of scheduling a card after review.
#[derive(Debug, Clone)]
pub struct SchedulingResult {
    pub new_state: CardState,
    pub next_due: DateTime<Utc>,
}

/// Trait for spaced repetition algorithms.
pub trait SpacedRepetitionAlgorithm: Send + Sync {
    /// Algorithm identifier.
    fn name(&self) -> &'static str;

    /// Calculate next review state after a review.
    fn schedule(&self, state: &CardState, rating: Rating, now: DateTime<Utc>) -> SchedulingResult;

    /// Initial state for a new card.
    fn initial_state(&self) -> CardState;
}

/// Get algorithm by name.
pub fn get_algorithm(name: &str) -> Option<Box<dyn SpacedRepetitionAlgorithm>> {
    match name {
        "sm2" => Some(Box::new(sm2::Sm2::default())),
        "fsrs" => Some(Box::new(fsrs::Fsrs::default())),
        _ => None,
    }
}
