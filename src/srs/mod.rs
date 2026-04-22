/// Phase 5 SRS scheduling policy.
/// This module provides a lightweight interval strategy for next due dates.

use chrono::{DateTime, Duration, Utc};

/// Compute next due timestamp (RFC3339) based on correctness and review count.
///
/// Policy (MVP):
/// - Wrong answer: due in 10 minutes
/// - Correct answer: due interval grows with review_count tier
///   - 0..=1 -> 1 day
///   - 2..=3 -> 3 days
///   - 4..=6 -> 7 days
///   - >=7   -> 14 days
pub fn compute_next_due(now: DateTime<Utc>, was_correct: bool, review_count: u32) -> String {
    let next = if was_correct {
        let interval = match review_count {
            0..=1 => Duration::days(1),
            2..=3 => Duration::days(3),
            4..=6 => Duration::days(7),
            _ => Duration::days(14),
        };
        now + interval
    } else {
        now + Duration::minutes(10)
    };

    next.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_answer_due_is_soon() {
        let now = Utc::now();
        let due = compute_next_due(now, false, 0);
        let parsed = DateTime::parse_from_rfc3339(&due).unwrap().with_timezone(&Utc);
        assert!(parsed >= now + Duration::minutes(9));
    }

    #[test]
    fn correct_answer_interval_grows_with_count() {
        let now = Utc::now();
        let low = DateTime::parse_from_rfc3339(&compute_next_due(now, true, 0))
            .unwrap()
            .with_timezone(&Utc);
        let high = DateTime::parse_from_rfc3339(&compute_next_due(now, true, 8))
            .unwrap()
            .with_timezone(&Utc);

        assert!(high > low);
    }
}

