//! Date utilities for daily reset hour handling.

use chrono::{Duration, Local, NaiveDate, Timelike};

/// Get adjusted "today" based on daily_reset_hour.
///
/// If the current hour is before the reset hour, "today" is actually "yesterday"
/// from a study perspective. This allows users to study late at night and have
/// it count towards the previous day.
///
/// # Arguments
/// * `daily_reset_hour` - Hour of day (0-23) when a new "study day" begins
///
/// # Returns
/// The adjusted date as a NaiveDate
pub fn get_adjusted_today(daily_reset_hour: u32) -> NaiveDate {
    let now = Local::now();
    let current_hour = now.hour();

    if current_hour < daily_reset_hour {
        // Before reset hour, consider it still "yesterday"
        (now - Duration::days(1)).date_naive()
    } else {
        now.date_naive()
    }
}

/// Format adjusted today as YYYY-MM-DD string for SQL queries.
pub fn get_adjusted_today_string(daily_reset_hour: u32) -> String {
    get_adjusted_today(daily_reset_hour)
        .format("%Y-%m-%d")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midnight_reset() {
        // With reset at midnight (0), any hour should return today
        let today = Local::now().date_naive();
        let result = get_adjusted_today(0);
        assert_eq!(result, today);
    }

    #[test]
    fn test_format_string() {
        let result = get_adjusted_today_string(0);
        // Should be in YYYY-MM-DD format
        assert_eq!(result.len(), 10);
        assert_eq!(&result[4..5], "-");
        assert_eq!(&result[7..8], "-");
    }
}
