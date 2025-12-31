//! SM-2 spaced repetition algorithm.
//!
//! Based on SuperMemo 2 with configurable parameters.

use super::{SchedulingResult, SpacedRepetitionAlgorithm};
use crate::types::{CardState, CardStatus, Rating};
use chrono::{DateTime, Duration, Utc};

/// SM-2 algorithm with configurable parameters.
#[derive(Debug, Clone)]
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
            due_date: None,
        }
    }

    fn schedule(&self, state: &CardState, rating: Rating, now: DateTime<Utc>) -> SchedulingResult {
        let rating_value = rating.to_value();

        let (new_status, new_interval, new_ease, new_lapses) = match state.status {
            CardStatus::New | CardStatus::Learning => self.schedule_learning(state, rating_value),
            CardStatus::Review | CardStatus::Relearning => self.schedule_review(state, rating_value),
        };

        let next_due = now + Duration::days(new_interval.ceil() as i64);

        SchedulingResult {
            new_state: CardState {
                status: new_status,
                interval_days: new_interval,
                ease_factor: new_ease,
                stability: None,
                difficulty: None,
                lapses: new_lapses,
                reviews_count: state.reviews_count + 1,
                due_date: Some(next_due),
            },
            next_due,
        }
    }
}

impl Sm2 {
    fn schedule_learning(&self, state: &CardState, rating: u8) -> (CardStatus, f64, f64, u32) {
        if rating >= 3 {
            let interval = if rating == 4 {
                self.easy_interval
            } else {
                self.graduating_interval
            };
            (CardStatus::Review, interval, state.ease_factor, state.lapses)
        } else {
            (CardStatus::Learning, 0.0, state.ease_factor, state.lapses)
        }
    }

    fn schedule_review(&self, state: &CardState, rating: u8) -> (CardStatus, f64, f64, u32) {
        if rating == 1 {
            // Lapse: reset to relearning
            (
                CardStatus::Relearning,
                1.0,
                (state.ease_factor - 0.2).max(self.minimum_ease),
                state.lapses + 1,
            )
        } else {
            let ease_adj = match rating {
                2 => -0.15,
                4 => 0.15,
                _ => 0.0,
            };
            let multiplier = match rating {
                2 => self.hard_multiplier,
                4 => state.ease_factor * self.easy_bonus,
                _ => state.ease_factor,
            };
            let new_interval = (state.interval_days * multiplier).max(1.0);
            let new_ease = (state.ease_factor + ease_adj).max(self.minimum_ease);
            (CardStatus::Review, new_interval, new_ease, state.lapses)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn new_card_graduates_on_good() {
        let sm2 = Sm2::default();
        let state = sm2.initial_state();
        let result = sm2.schedule(&state, Rating::Good, now());
        assert_eq!(result.new_state.status, CardStatus::Review);
        assert_eq!(result.new_state.interval_days, 1.0);
    }

    #[test]
    fn new_card_easy_gets_longer_interval() {
        let sm2 = Sm2::default();
        let state = sm2.initial_state();
        let result = sm2.schedule(&state, Rating::Easy, now());
        assert_eq!(result.new_state.interval_days, 4.0);
    }

    #[test]
    fn review_card_lapse_on_again() {
        let sm2 = Sm2::default();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            ease_factor: 2.5,
            lapses: 0,
            reviews_count: 5,
            ..Default::default()
        };
        let result = sm2.schedule(&state, Rating::Again, now());
        assert_eq!(result.new_state.status, CardStatus::Relearning);
        assert_eq!(result.new_state.lapses, 1);
    }

    #[test]
    fn ease_factor_never_below_minimum() {
        let sm2 = Sm2::default();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            ease_factor: 1.4,
            ..Default::default()
        };
        let result = sm2.schedule(&state, Rating::Again, now());
        assert!(result.new_state.ease_factor >= sm2.minimum_ease);
    }
}
