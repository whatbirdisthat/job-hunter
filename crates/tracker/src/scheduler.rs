//! Follow-up scheduler (R-SCH-*). Pure, date-driven, CLOCK INJECTED — `today` is always a
//! parameter; no function here reads the wall clock.
//!
//! Aging boundaries (the pinned L1 coordinates, FOUNDER resolutions §8):
//! day 0–2 → `None`; day 3–5 → `FirstFollowUp`; day 6 → `None` (inter-window gap);
//! day 7–10 → `SecondFollowUp`; day 11+ → `Archive`.

use crate::date::Date;
use serde::{Deserialize, Serialize};

/// A recommended follow-up window as inclusive day offsets from submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowUpWindow {
    pub opens_day: u32,
    pub closes_day: u32,
}

/// The aging action for an application given its submission date and `today`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgingAction {
    /// Before the first window, or in the day-6 gap (days 0–2 and day 6).
    None,
    /// Days 3–5 inclusive.
    FirstFollowUp,
    /// Days 7–10 inclusive.
    SecondFollowUp,
    /// Beyond day 10 (deprioritise / archive).
    Archive,
}

/// Whole calendar days from `submitted` to `today`, clamped to `0` when `today < submitted`
/// (R-SCH-1/2 — DISCUSS-FUTUREDATE clamp). Counts across month/year boundaries exactly.
pub fn days_since(submitted: Date, today: Date) -> i64 {
    submitted.days_until(&today).max(0)
}

/// The aging action for an application given its submission date and `today` (R-SCH-4..7).
/// Clock-injected: `today` is a parameter; NO wall clock read (R-SCH-3).
pub fn aging_action(submitted: Date, today: Date) -> AgingAction {
    match days_since(submitted, today) {
        0..=2 => AgingAction::None,
        3..=5 => AgingAction::FirstFollowUp,
        6 => AgingAction::None,
        7..=10 => AgingAction::SecondFollowUp,
        _ => AgingAction::Archive,
    }
}

/// The deterministic constant follow-up window for an aging action (R-SCH-7).
/// `None`/`Archive` have no actionable window.
pub fn follow_up_window(action: &AgingAction) -> Option<FollowUpWindow> {
    match action {
        AgingAction::FirstFollowUp => Some(FollowUpWindow {
            opens_day: 3,
            closes_day: 5,
        }),
        AgingAction::SecondFollowUp => Some(FollowUpWindow {
            opens_day: 7,
            closes_day: 10,
        }),
        AgingAction::None | AgingAction::Archive => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `submitted` fixed at 2026-01-01; `today` = day-0 + `offset` days, computed honestly
    /// through the Date arithmetic (so a fence-post bug in days_since trips these too).
    fn submitted() -> Date {
        Date::new(2026, 1, 1)
    }
    fn day(offset: u32) -> Date {
        // 2026-01-01 + offset days, staying within January for offsets <= 30.
        Date::new(2026, 1, 1 + offset)
    }

    #[test]
    fn days_since_same_day_is_zero() {
        assert_eq!(days_since(submitted(), submitted()), 0);
    }

    #[test]
    fn days_since_month_boundary() {
        // R-SCH-1 — Jan 30 -> Feb 2 is 3 calendar days.
        assert_eq!(days_since(Date::new(2026, 1, 30), Date::new(2026, 2, 2)), 3);
    }

    #[test]
    fn days_since_year_boundary() {
        // R-SCH-1 — Dec 31 -> Jan 2 next year is 2 calendar days.
        assert_eq!(
            days_since(Date::new(2025, 12, 31), Date::new(2026, 1, 2)),
            2
        );
    }

    #[test]
    fn future_today_clamps_to_zero() {
        // R-SCH-2 — today before submitted clamps to 0 (DISCUSS-FUTUREDATE), and the
        // aging action is None.
        assert_eq!(days_since(Date::new(2026, 1, 6), Date::new(2026, 1, 1)), 0);
        assert_eq!(
            aging_action(Date::new(2026, 1, 6), Date::new(2026, 1, 1)),
            AgingAction::None
        );
    }

    #[test]
    fn day_2_is_none() {
        // R-SCH-4 — day 2 is before the first window.
        assert_eq!(aging_action(submitted(), day(2)), AgingAction::None);
        assert_eq!(aging_action(submitted(), day(0)), AgingAction::None);
    }

    #[test]
    fn day_3_and_5_first_follow_up() {
        // R-SCH-5 — window opens at day 3, closes at day 5.
        assert_eq!(
            aging_action(submitted(), day(3)),
            AgingAction::FirstFollowUp
        );
        assert_eq!(
            aging_action(submitted(), day(4)),
            AgingAction::FirstFollowUp
        );
        assert_eq!(
            aging_action(submitted(), day(5)),
            AgingAction::FirstFollowUp
        );
    }

    #[test]
    fn day_6_is_none() {
        // R-SCH-6 — the inter-window gap (DISCUSS-WINDOW-GAP).
        assert_eq!(aging_action(submitted(), day(6)), AgingAction::None);
    }

    #[test]
    fn day_7_and_10_second_follow_up() {
        // R-SCH-6 — second window opens at day 7, closes at day 10.
        assert_eq!(
            aging_action(submitted(), day(7)),
            AgingAction::SecondFollowUp
        );
        assert_eq!(
            aging_action(submitted(), day(10)),
            AgingAction::SecondFollowUp
        );
    }

    #[test]
    fn day_11_is_archive() {
        // R-SCH-7 — beyond day 10.
        assert_eq!(aging_action(submitted(), day(11)), AgingAction::Archive);
        assert_eq!(aging_action(submitted(), day(30)), AgingAction::Archive);
    }

    #[test]
    fn follow_up_window_constants() {
        // R-SCH-7 — deterministic constant windows.
        assert_eq!(
            follow_up_window(&AgingAction::FirstFollowUp),
            Some(FollowUpWindow {
                opens_day: 3,
                closes_day: 5
            })
        );
        assert_eq!(
            follow_up_window(&AgingAction::SecondFollowUp),
            Some(FollowUpWindow {
                opens_day: 7,
                closes_day: 10
            })
        );
        assert_eq!(follow_up_window(&AgingAction::None), None);
        assert_eq!(follow_up_window(&AgingAction::Archive), None);
    }
}
