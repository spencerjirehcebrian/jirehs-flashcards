//! FSRS (Free Spaced Repetition Scheduler) algorithm.
//!
//! Modern algorithm based on memory research using DSR model:
//! - Difficulty (D): Card difficulty 1-10
//! - Stability (S): Days until retention drops to target
//! - Retrievability (R): Probability of recall

use super::{SchedulingResult, SpacedRepetitionAlgorithm};
use crate::types::{CardState, CardStatus, Rating};
use chrono::{DateTime, Duration, Utc};

/// FSRS algorithm with configurable parameters.
#[derive(Debug, Clone)]
pub struct Fsrs {
    pub request_retention: f64,
    pub maximum_interval: f64,
    /// FSRS-4.5 parameters (17 weights).
    pub w: [f64; 17],
}

impl Default for Fsrs {
    fn default() -> Self {
        Self {
            request_retention: 0.9,
            maximum_interval: 36500.0,
            w: [
                0.4, 0.6, 2.4, 5.8, // w[0-3]: initial stability for Again, Hard, Good, Easy
                4.93,  // w[4]: initial difficulty base
                0.94,  // w[5]: initial difficulty modifier
                0.86,  // w[6]: difficulty decay
                0.01,  // w[7]: mean reversion weight
                1.49,  // w[8]: stability exp base
                0.14,  // w[9]: stability decay
                0.94,  // w[10]: retrievability effect
                2.18,  // w[11]: forget stability base
                0.05,  // w[12]: difficulty on forget
                0.34,  // w[13]: stability on forget
                1.26,  // w[14]: retrievability on forget
                0.29,  // w[15]: hard penalty
                2.61,  // w[16]: easy bonus
            ],
        }
    }
}

impl SpacedRepetitionAlgorithm for Fsrs {
    fn name(&self) -> &'static str {
        "fsrs"
    }

    fn initial_state(&self) -> CardState {
        CardState {
            status: CardStatus::New,
            interval_days: 0.0,
            ease_factor: 2.5,
            stability: None,
            difficulty: None,
            lapses: 0,
            reviews_count: 0,
            due_date: None,
        }
    }

    fn schedule(&self, state: &CardState, rating: Rating, now: DateTime<Utc>) -> SchedulingResult {
        let rating_value = rating.to_value();
        let is_first_review = state.reviews_count == 0
            || state.stability.is_none()
            || state.difficulty.is_none();

        let (new_stability, new_difficulty, new_status, new_lapses) = if is_first_review {
            self.schedule_first_review(state.status, rating_value, state.lapses)
        } else {
            self.schedule_subsequent_review(state, rating_value, now)
        };

        let new_interval = if rating_value == 1 {
            self.short_term_interval(new_stability)
        } else {
            self.interval_from_stability(new_stability)
        };

        let next_due = now + Duration::seconds((new_interval * 86400.0) as i64);

        SchedulingResult {
            new_state: CardState {
                status: new_status,
                interval_days: new_interval,
                ease_factor: state.ease_factor,
                stability: Some(new_stability),
                difficulty: Some(new_difficulty),
                lapses: new_lapses,
                reviews_count: state.reviews_count + 1,
                due_date: Some(next_due),
            },
            next_due,
        }
    }
}

impl Fsrs {
    /// Calculate initial stability for a new card based on first rating.
    /// S0(G) = w[G-1] where G is rating 1-4
    fn initial_stability(&self, rating: u8) -> f64 {
        let index = (rating.saturating_sub(1)) as usize;
        self.w[index.min(3)].max(0.1)
    }

    /// Calculate initial difficulty for a new card based on first rating.
    /// D0(G) = w[4] - w[5] * (G - 3)
    fn initial_difficulty(&self, rating: u8) -> f64 {
        let d0 = self.w[4] - self.w[5] * (rating as f64 - 3.0);
        d0.clamp(1.0, 10.0)
    }

    /// Calculate next difficulty using mean reversion.
    /// D' = w[7] * D0(G) + (1 - w[7]) * D
    /// Apply decay: D'' = D' - w[6] * (G - 3)
    fn next_difficulty(&self, current_d: f64, rating: u8) -> f64 {
        let d0 = self.initial_difficulty(rating);
        let d_new = self.w[7] * d0 + (1.0 - self.w[7]) * current_d;
        let d_decayed = d_new - self.w[6] * (rating as f64 - 3.0);
        d_decayed.clamp(1.0, 10.0)
    }

    /// Calculate retrievability (probability of recall).
    /// R = (1 + t / (9 * S))^(-1)
    fn retrievability(&self, elapsed_days: f64, stability: f64) -> f64 {
        if stability <= 0.0 {
            return 0.0;
        }
        let factor = 1.0 + elapsed_days / (9.0 * stability);
        factor.powf(-1.0)
    }

    /// Calculate next stability after successful recall.
    /// S' = S * (e^(w[8]) * (11 - D) * S^(-w[9]) * (e^(w[10]*(1-R)) - 1) + 1) * modifier
    fn next_stability_recall(
        &self,
        stability: f64,
        difficulty: f64,
        retrievability: f64,
        rating: u8,
    ) -> f64 {
        let exp_w8 = self.w[8].exp();
        let d_factor = (11.0 - difficulty).max(0.1);
        let s_decay = stability.powf(-self.w[9]);
        let r_factor = (self.w[10] * (1.0 - retrievability)).exp() - 1.0;

        let growth = exp_w8 * d_factor * s_decay * r_factor + 1.0;

        let modifier = match rating {
            2 => self.w[15], // Hard penalty
            4 => self.w[16], // Easy bonus
            _ => 1.0,        // Good (3) or default
        };

        let new_s = stability * growth * modifier;
        new_s.max(0.1).min(self.maximum_interval)
    }

    /// Calculate next stability after forgetting (lapse).
    /// S' = w[11] * D^(-w[12]) * ((S+1)^w[13] - 1) * e^(w[14]*(1-R))
    fn next_stability_forget(
        &self,
        stability: f64,
        difficulty: f64,
        retrievability: f64,
    ) -> f64 {
        let d_factor = difficulty.max(1.0).powf(-self.w[12]);
        let s_factor = (stability + 1.0).powf(self.w[13]) - 1.0;
        let r_factor = (self.w[14] * (1.0 - retrievability)).exp();

        let new_s = self.w[11] * d_factor * s_factor * r_factor;
        // Never exceed previous stability on lapse
        new_s.max(0.1).min(stability)
    }

    /// Calculate optimal interval from stability.
    /// I = 9 * S * (1/R - 1) where R = request_retention
    fn interval_from_stability(&self, stability: f64) -> f64 {
        if self.request_retention <= 0.0 || self.request_retention >= 1.0 {
            return stability;
        }
        let interval = 9.0 * stability * (1.0 / self.request_retention - 1.0);
        interval.max(1.0).min(self.maximum_interval)
    }

    /// Calculate short-term interval for learning/relearning states.
    fn short_term_interval(&self, stability: f64) -> f64 {
        // Use 10 minutes to 1 day based on stability
        let minutes = (stability * 60.0).max(10.0).min(1440.0);
        minutes / 1440.0
    }

    /// Calculate elapsed days since last review.
    fn elapsed_days(state: &CardState, now: DateTime<Utc>) -> f64 {
        match state.due_date {
            Some(due) => {
                // Due date = last review + interval, so last review = due - interval
                let interval_secs = (state.interval_days * 86400.0) as i64;
                let last_review = due - Duration::seconds(interval_secs);
                let elapsed = now.signed_duration_since(last_review);
                (elapsed.num_seconds() as f64 / 86400.0).max(0.0)
            }
            None => state.interval_days.max(0.0),
        }
    }

    /// Determine new status based on current status and rating.
    fn determine_status(current: CardStatus, rating: u8) -> CardStatus {
        match (current, rating) {
            (CardStatus::New, 1) => CardStatus::Learning,
            (CardStatus::New, _) => CardStatus::Review,
            (CardStatus::Learning, 1) => CardStatus::Learning,
            (CardStatus::Learning, _) => CardStatus::Review,
            (CardStatus::Review, 1) => CardStatus::Relearning,
            (CardStatus::Review, _) => CardStatus::Review,
            (CardStatus::Relearning, 1) => CardStatus::Relearning,
            (CardStatus::Relearning, _) => CardStatus::Review,
        }
    }

    /// Schedule first review - initialize stability and difficulty.
    fn schedule_first_review(
        &self,
        current_status: CardStatus,
        rating: u8,
        current_lapses: u32,
    ) -> (f64, f64, CardStatus, u32) {
        let stability = self.initial_stability(rating);
        let difficulty = self.initial_difficulty(rating);
        let status = Self::determine_status(current_status, rating);
        (stability, difficulty, status, current_lapses)
    }

    /// Schedule subsequent review - update stability and difficulty.
    fn schedule_subsequent_review(
        &self,
        state: &CardState,
        rating: u8,
        now: DateTime<Utc>,
    ) -> (f64, f64, CardStatus, u32) {
        let current_s = state.stability.unwrap_or(1.0);
        let current_d = state.difficulty.unwrap_or(5.0);

        let elapsed = Self::elapsed_days(state, now);
        let r = self.retrievability(elapsed, current_s);

        // Update difficulty
        let new_d = self.next_difficulty(current_d, rating);

        // Update stability and status based on rating
        let (new_s, lapses) = if rating == 1 {
            // Lapse: forgot the card
            let s = self.next_stability_forget(current_s, current_d, r);
            (s, state.lapses + 1)
        } else {
            // Recall: remembered the card
            let s = self.next_stability_recall(current_s, current_d, r, rating);
            (s, state.lapses)
        };

        let status = Self::determine_status(state.status, rating);
        (new_s, new_d, status, lapses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn new_card_first_review_good() {
        let fsrs = Fsrs::default();
        let state = fsrs.initial_state();
        let result = fsrs.schedule(&state, Rating::Good, now());

        assert_eq!(result.new_state.status, CardStatus::Review);
        assert!(result.new_state.stability.unwrap() > 0.0);
        assert!(result.new_state.difficulty.is_some());
        assert_eq!(result.new_state.reviews_count, 1);
    }

    #[test]
    fn new_card_first_review_again() {
        let fsrs = Fsrs::default();
        let state = fsrs.initial_state();
        let result = fsrs.schedule(&state, Rating::Again, now());

        assert_eq!(result.new_state.status, CardStatus::Learning);
        assert!(result.new_state.stability.unwrap() > 0.0);
    }

    #[test]
    fn new_card_first_review_easy_higher_stability() {
        let fsrs = Fsrs::default();
        let state = fsrs.initial_state();

        let good_result = fsrs.schedule(&state, Rating::Good, now());
        let easy_result = fsrs.schedule(&state, Rating::Easy, now());

        assert!(
            easy_result.new_state.stability.unwrap() > good_result.new_state.stability.unwrap()
        );
    }

    #[test]
    fn stability_increases_on_successful_recall() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 5.0,
            ease_factor: 2.5,
            stability: Some(5.0),
            difficulty: Some(5.0),
            lapses: 0,
            reviews_count: 5,
            due_date: Some(current_time),
        };

        let result = fsrs.schedule(&state, Rating::Good, current_time);
        assert!(result.new_state.stability.unwrap() > 5.0);
    }

    #[test]
    fn stability_decreases_on_lapse() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            ease_factor: 2.5,
            stability: Some(10.0),
            difficulty: Some(5.0),
            lapses: 0,
            reviews_count: 5,
            due_date: Some(current_time),
        };

        let result = fsrs.schedule(&state, Rating::Again, current_time);
        assert!(result.new_state.stability.unwrap() < 10.0);
        assert_eq!(result.new_state.lapses, 1);
    }

    #[test]
    fn difficulty_decreases_on_easy() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            stability: Some(5.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            interval_days: 5.0,
            reviews_count: 1,
            ..Default::default()
        };

        let result = fsrs.schedule(&state, Rating::Easy, current_time);
        assert!(result.new_state.difficulty.unwrap() < 5.0);
    }

    #[test]
    fn difficulty_increases_on_again() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            stability: Some(5.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            interval_days: 5.0,
            reviews_count: 1,
            ..Default::default()
        };

        let result = fsrs.schedule(&state, Rating::Again, current_time);
        assert!(result.new_state.difficulty.unwrap() > 5.0);
    }

    #[test]
    fn difficulty_clamped_to_bounds() {
        let fsrs = Fsrs::default();
        let current_time = now();

        // Test upper bound
        let state = CardState {
            status: CardStatus::Review,
            stability: Some(5.0),
            difficulty: Some(10.0),
            due_date: Some(current_time),
            interval_days: 5.0,
            reviews_count: 1,
            ..Default::default()
        };
        let result = fsrs.schedule(&state, Rating::Again, current_time);
        assert!(result.new_state.difficulty.unwrap() <= 10.0);

        // Test lower bound
        let state = CardState {
            difficulty: Some(1.0),
            ..state
        };
        let result = fsrs.schedule(&state, Rating::Easy, current_time);
        assert!(result.new_state.difficulty.unwrap() >= 1.0);
    }

    #[test]
    fn interval_respects_maximum() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            stability: Some(50000.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            interval_days: 1000.0,
            reviews_count: 10,
            ..Default::default()
        };

        let result = fsrs.schedule(&state, Rating::Good, current_time);
        assert!(result.new_state.interval_days <= fsrs.maximum_interval);
    }

    #[test]
    fn retrievability_formula() {
        let fsrs = Fsrs::default();

        // At t=0, R should be 1.0
        let r = fsrs.retrievability(0.0, 10.0);
        assert!((r - 1.0).abs() < 0.001);

        // At t = 9*S, R = 0.5
        let r = fsrs.retrievability(90.0, 10.0);
        assert!((r - 0.5).abs() < 0.001);
    }

    #[test]
    fn learning_card_graduates_on_good() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Learning,
            stability: Some(1.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            interval_days: 0.01,
            reviews_count: 1,
            ..Default::default()
        };

        let result = fsrs.schedule(&state, Rating::Good, current_time);
        assert_eq!(result.new_state.status, CardStatus::Review);
    }

    #[test]
    fn review_card_lapses_on_again() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            stability: Some(10.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            interval_days: 10.0,
            reviews_count: 5,
            ..Default::default()
        };

        let result = fsrs.schedule(&state, Rating::Again, current_time);
        assert_eq!(result.new_state.status, CardStatus::Relearning);
    }

    #[test]
    fn hard_penalty_reduces_stability_growth() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            stability: Some(10.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            reviews_count: 5,
            ..Default::default()
        };

        let good_result = fsrs.schedule(&state, Rating::Good, current_time);
        let hard_result = fsrs.schedule(&state, Rating::Hard, current_time);

        assert!(
            hard_result.new_state.stability.unwrap() < good_result.new_state.stability.unwrap()
        );
    }

    #[test]
    fn easy_bonus_increases_stability_growth() {
        let fsrs = Fsrs::default();
        let current_time = now();
        let state = CardState {
            status: CardStatus::Review,
            interval_days: 10.0,
            stability: Some(10.0),
            difficulty: Some(5.0),
            due_date: Some(current_time),
            reviews_count: 5,
            ..Default::default()
        };

        let good_result = fsrs.schedule(&state, Rating::Good, current_time);
        let easy_result = fsrs.schedule(&state, Rating::Easy, current_time);

        assert!(
            easy_result.new_state.stability.unwrap() > good_result.new_state.stability.unwrap()
        );
    }

    #[test]
    fn initial_stability_values() {
        let fsrs = Fsrs::default();

        // Initial stability should increase with rating
        let s_again = fsrs.initial_stability(1);
        let s_hard = fsrs.initial_stability(2);
        let s_good = fsrs.initial_stability(3);
        let s_easy = fsrs.initial_stability(4);

        assert!(s_again < s_hard);
        assert!(s_hard < s_good);
        assert!(s_good < s_easy);
    }

    #[test]
    fn initial_difficulty_values() {
        let fsrs = Fsrs::default();

        // Initial difficulty should decrease with rating
        let d_again = fsrs.initial_difficulty(1);
        let d_hard = fsrs.initial_difficulty(2);
        let d_good = fsrs.initial_difficulty(3);
        let d_easy = fsrs.initial_difficulty(4);

        assert!(d_again > d_hard);
        assert!(d_hard > d_good);
        assert!(d_good > d_easy);
    }
}
