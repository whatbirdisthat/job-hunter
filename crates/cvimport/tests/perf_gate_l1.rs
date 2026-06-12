//! L1 — unit tests for the shared perf-delta gate (Finding 3). Proves the gate is
//! NON-VACUOUS: a simulated 100× regression that is still far under the absolute budget
//! MUST fail the delta arm (the old `|| elapsed < BUDGET` gate let it pass).

#[path = "perf_gate.rs"]
mod perf_gate;

use perf_gate::{enforce_gate, evaluate_gate, read_baseline, GateResult};

#[test]
fn passes_within_budget_and_delta() {
    // baseline 0.5s, 3x factor, run 1.0s → under 60s budget AND under 1.5s delta cap.
    assert_eq!(evaluate_gate(1.0, Some(0.5), 60.0, 3.0), GateResult::Pass);
}

#[test]
fn simulated_100x_regression_fails_the_delta_assert() {
    // THE non-vacuity proof: a baseline of 0.01s and a run 100× slower (1.0s) is still far
    // under the 60s budget, yet the delta arm MUST fire. (Old gate: this passed.)
    assert_eq!(
        evaluate_gate(1.0, Some(0.01), 60.0, 3.0),
        GateResult::Regressed
    );
}

#[test]
fn over_budget_fails_even_without_a_baseline() {
    assert_eq!(evaluate_gate(61.0, None, 60.0, 3.0), GateResult::OverBudget);
}

#[test]
fn no_baseline_applies_only_the_budget() {
    // first run / fresh checkout: no tracked baseline → delta arm skipped, budget holds.
    assert_eq!(evaluate_gate(0.2, None, 60.0, 3.0), GateResult::Pass);
}

#[test]
fn exactly_at_delta_boundary_passes() {
    // boundary: elapsed == baseline * factor is allowed (only STRICTLY slower regresses).
    assert_eq!(evaluate_gate(1.5, Some(0.5), 60.0, 3.0), GateResult::Pass);
}

#[test]
fn enforce_gate_panics_on_regression() {
    let r = std::panic::catch_unwind(|| {
        enforce_gate("unit", 1.0, Some(0.01), 60.0, 3.0);
    });
    assert!(
        r.is_err(),
        "a 100x regression must panic the enforce wrapper"
    );
}

#[test]
fn enforce_gate_panics_over_budget() {
    let r = std::panic::catch_unwind(|| {
        enforce_gate("unit", 61.0, None, 60.0, 3.0);
    });
    assert!(r.is_err());
}

#[test]
fn enforce_gate_passes_silently_when_within_bounds() {
    enforce_gate("unit", 0.2, Some(0.5), 60.0, 3.0);
}

#[test]
fn read_baseline_missing_file_is_none() {
    let p = std::path::Path::new("/nonexistent/perf-baseline-xyz.txt");
    assert_eq!(read_baseline(p), None);
}

#[test]
fn read_baseline_parses_a_tracked_value() {
    let dir = std::env::temp_dir();
    let p = dir.join(format!("perf-gate-test-{}.txt", std::process::id()));
    std::fs::write(&p, "0.123\n").unwrap();
    assert_eq!(read_baseline(&p), Some(0.123));
    let _ = std::fs::remove_file(&p);
}
