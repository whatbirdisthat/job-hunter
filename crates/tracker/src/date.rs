//! `Date` — a small calendar-day value type for the tracker cores (R-SCH-1).
//!
//! `{ year, month, day }` with `Ord` + serde. Day arithmetic is whole-calendar-day
//! counting with NO timezones (the cores never read the wall clock — `today` is always a
//! parameter). The ordering is the natural lexicographic (year, month, day) order, which is
//! the correct chronological order for proper calendar dates.

use serde::{Deserialize, Serialize};

/// A calendar date — year/month/day, no time-of-day, no timezone. `Ord` is chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    /// Construct a date. The cores treat the fields as already-valid calendar coordinates
    /// (the fixtures and command boundary supply real dates); arithmetic is day-count based.
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Date { year, month, day }
    }

    /// Days from the proleptic-Gregorian epoch (0000-03-01) — a monotonic day index used
    /// ONLY internally to subtract two dates into a whole-day delta. Uses the standard
    /// civil-from-days algorithm (Howard Hinnant's `days_from_civil`) so month/year
    /// boundaries are handled exactly without a date library.
    fn day_number(&self) -> i64 {
        let y = self.year as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        // shift so March is month 0 (Feb's leap day lands at year-end)
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400; // [0, 399]
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
        era * 146097 + doe - 719468
    }

    /// Whole calendar days from `self` to `other` (`other - self`). Negative when `other`
    /// precedes `self`. Callers that need a non-negative "days since" clamp at the call site
    /// (R-SCH-2). Counts across month and year boundaries exactly (R-SCH-1).
    pub fn days_until(&self, other: &Date) -> i64 {
        other.day_number() - self.day_number()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_chronological() {
        assert!(Date::new(2026, 1, 1) < Date::new(2026, 1, 2));
        assert!(Date::new(2025, 12, 31) < Date::new(2026, 1, 1));
        assert!(Date::new(2026, 2, 1) > Date::new(2026, 1, 28));
    }

    #[test]
    fn days_until_basic_and_signed() {
        assert_eq!(Date::new(2026, 1, 1).days_until(&Date::new(2026, 1, 1)), 0);
        assert_eq!(
            Date::new(2026, 1, 1).days_until(&Date::new(2026, 1, 11)),
            10
        );
        // negative when other precedes self (the scheduler clamps at its call site)
        assert_eq!(
            Date::new(2026, 1, 11).days_until(&Date::new(2026, 1, 1)),
            -10
        );
    }

    #[test]
    fn days_until_leap_day() {
        // 2028 is a leap year — Feb has 29 days; Feb 28 -> Mar 1 is 2 days.
        assert_eq!(Date::new(2028, 2, 28).days_until(&Date::new(2028, 3, 1)), 2);
        // 2026 is not a leap year — Feb 28 -> Mar 1 is 1 day.
        assert_eq!(Date::new(2026, 2, 28).days_until(&Date::new(2026, 3, 1)), 1);
    }

    #[test]
    fn serde_round_trip() {
        let d = Date::new(2026, 6, 13);
        let j = serde_json::to_string(&d).unwrap();
        assert_eq!(j, r#"{"year":2026,"month":6,"day":13}"#);
        assert_eq!(serde_json::from_str::<Date>(&j).unwrap(), d);
    }

    #[test]
    fn negative_year_day_number_exercises_negative_era_branch() {
        // Negative years exercise the y < 0 branch in the era calculation.
        // year == 0, month <= 2 → y = 0 - 1 = -1 → y < 0 branch triggers.
        // We compute days from proleptic-Gregorian epoch (0000-03-01).
        // year 0, month 1 (January), day 1 is well before the epoch.
        // This test ensures the else-branch of `if y >= 0 { y } else { y - 399 }`
        // is reachable and correctly computes the negative-year civil day number.
        let date_year_zero = Date::new(0, 1, 1);
        let date_year_one = Date::new(1, 1, 1);
        // year 0 is before year 1, so days_until should be negative
        assert!(date_year_zero.days_until(&date_year_one) > 0);
        // Also test that year 0 computes a valid day number and can be ordered
        let date_year_neg_one = Date::new(-1, 6, 15);
        assert!(date_year_neg_one < date_year_zero);
    }
}
