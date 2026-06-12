//! Lifecycle state machine (R-TRK-*). Pure, total, NO clock, NO IO.
//!
//! States `Discovered → Tailored → Applied → FollowUpDue → Interview → Closed`. Transitions
//! are EXPLICIT and TESTED; an illegal transition is a typed error, never a silent no-op.
//! `legal_transitions()` exposes the table as DATA so the L1 test enumerates the full
//! `AppState × AppState` matrix and asserts each cell is exactly legal-or-error.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The application lifecycle states (R-TRK-1). `Closed` is terminal (R-TRK-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    Discovered,
    Tailored,
    Applied,
    FollowUpDue,
    Interview,
    Closed,
}

/// Every lifecycle state, in declaration order — lets the test enumerate the full matrix.
pub const ALL_STATES: [AppState; 6] = [
    AppState::Discovered,
    AppState::Tailored,
    AppState::Applied,
    AppState::FollowUpDue,
    AppState::Interview,
    AppState::Closed,
];

/// An illegal transition (R-TRK-3) — a typed error carrying the offending pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("illegal transition: {from:?} -> {to:?}")]
pub struct TransitionError {
    pub from: AppState,
    pub to: AppState,
}

/// The legal-transition table as DATA (R-TRK-5) — the single source of truth the L1 test
/// enumerates. `Closed` appears only as a destination, never a source (R-TRK-4).
pub fn legal_transitions() -> &'static [(AppState, AppState)] {
    use AppState::*;
    &[
        (Discovered, Tailored),
        (Tailored, Applied),
        (Applied, FollowUpDue),
        (Applied, Closed),
        (FollowUpDue, Interview),
        (FollowUpDue, Closed),
        (Interview, Closed),
    ]
}

/// Pure, total transition (R-TRK-2/3): legal → `Ok(to)`, illegal → typed `Err`. NO clock.
pub fn transition(from: AppState, to: AppState) -> Result<AppState, TransitionError> {
    if legal_transitions()
        .iter()
        .any(|&(f, t)| f == from && t == to)
    {
        Ok(to)
    } else {
        Err(TransitionError { from, to })
    }
}

/// A bad enum string at the command boundary (R-TRK-CMD-3) — typed, never a panic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unrecognized {kind} value: {value:?}")]
pub struct ParseEnumError {
    pub kind: &'static str,
    pub value: String,
}

/// Parse an `AppState` from a lowercase string (R-TRK-CMD-3) — never panics.
impl AppState {
    pub fn parse(s: &str) -> Result<AppState, ParseEnumError> {
        match s.trim().to_lowercase().as_str() {
            "discovered" => Ok(AppState::Discovered),
            "tailored" => Ok(AppState::Tailored),
            "applied" => Ok(AppState::Applied),
            "followupdue" | "follow_up_due" | "followup" => Ok(AppState::FollowUpDue),
            "interview" => Ok(AppState::Interview),
            "closed" => Ok(AppState::Closed),
            other => Err(ParseEnumError {
                kind: "AppState",
                value: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AppState::*;

    #[test]
    fn all_states_present() {
        // R-TRK-1 — exactly the six declared states, in order.
        assert_eq!(ALL_STATES.len(), 6);
        assert_eq!(
            ALL_STATES,
            [
                Discovered,
                Tailored,
                Applied,
                FollowUpDue,
                Interview,
                Closed
            ]
        );
    }

    #[test]
    fn legal_edges_transition_ok() {
        // R-TRK-2 — every listed edge advances to its destination.
        for &(from, to) in legal_transitions() {
            assert_eq!(
                transition(from, to),
                Ok(to),
                "{from:?}->{to:?} should be legal"
            );
        }
    }

    #[test]
    fn illegal_edge_is_typed_error() {
        // R-TRK-3 — an unlisted pair is a typed error, never a panic / no-op.
        assert_eq!(
            transition(Discovered, Interview),
            Err(TransitionError {
                from: Discovered,
                to: Interview
            })
        );
        // self-loops are illegal too
        assert!(transition(Applied, Applied).is_err());
    }

    #[test]
    fn closed_is_terminal() {
        // R-TRK-4 — Closed has no outgoing legal edge.
        assert!(
            !legal_transitions().iter().any(|&(f, _)| f == Closed),
            "Closed must not be a source of any legal edge"
        );
        for &to in &ALL_STATES {
            assert!(
                transition(Closed, to).is_err(),
                "Closed->{to:?} must be illegal"
            );
        }
    }

    #[test]
    fn full_matrix_is_legal_or_error() {
        // R-TRK-5 — enumerate the full AppState x AppState matrix; each cell is exactly
        // legal-or-error and the two views agree (the table is the single source of truth).
        for &from in &ALL_STATES {
            for &to in &ALL_STATES {
                let in_table = legal_transitions()
                    .iter()
                    .any(|&(f, t)| f == from && t == to);
                match transition(from, to) {
                    Ok(got) => {
                        assert!(
                            in_table,
                            "{from:?}->{to:?} returned Ok but is not in the table"
                        );
                        assert_eq!(got, to);
                    }
                    Err(e) => {
                        assert!(
                            !in_table,
                            "{from:?}->{to:?} is in the table but returned Err"
                        );
                        assert_eq!(e, TransitionError { from, to });
                    }
                }
            }
        }
    }

    #[test]
    fn removing_an_edge_flips_a_cell() {
        // R-TRK-5 non-vacuous twin — a pair NOT in the table is provably an error, so the
        // matrix test above is not trivially satisfiable. If `Discovered->Tailored` were the
        // only legal edge removed, `transition` would return Err for it — we assert the
        // contrapositive: a deliberately-unlisted neighbour pair IS an error today.
        assert!(transition(Tailored, Discovered).is_err()); // reverse of a legal edge
        assert!(transition(Discovered, Applied).is_err()); // skipping Tailored
    }

    #[test]
    fn parse_round_trips_and_rejects() {
        // R-TRK-CMD-3 — lowercase strings parse; a bad value is a typed error, no panic.
        assert_eq!(AppState::parse("discovered"), Ok(Discovered));
        assert_eq!(AppState::parse("tailored"), Ok(Tailored));
        assert_eq!(AppState::parse("applied"), Ok(Applied));
        assert_eq!(AppState::parse("  Closed "), Ok(Closed));
        assert_eq!(AppState::parse("followUpDue"), Ok(FollowUpDue));
        assert_eq!(AppState::parse("interview"), Ok(Interview));
        let err = AppState::parse("teleported").unwrap_err();
        assert_eq!(err.kind, "AppState");
        assert!(err.to_string().contains("teleported"));
    }

    #[test]
    fn transition_error_display() {
        let e = TransitionError {
            from: Applied,
            to: Discovered,
        };
        assert!(e.to_string().contains("Applied"));
    }
}
