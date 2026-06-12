//! Shared perf-delta gate logic for the L5 STORY tests (Finding 3).
//!
//! The previous gate `elapsed <= prev*DELTA || elapsed < BUDGET` was vacuous: because
//! BUDGET (60 s) dwarfs a sub-second import, the right-hand disjunct was ALWAYS true, so
//! a 100× regression still passed. And the baseline was written to gitignored `target/`,
//! re-seeding itself every run (self-ratcheting), so CI never had a stable comparison.
//!
//! This module splits the gate into TWO independent obligations and reads the baseline
//! from a TRACKED file that is NOT silently overwritten every run:
//!   (a) the absolute I6 budget: `elapsed < budget_secs`;
//!   (b) the regression delta:   `elapsed <= baseline * delta_factor` (can actually fire).
//!
//! `#[path]`-included by both `crates/cvimport/tests/story_l5.rs` and
//! `apps/desktop/src-tauri/tests/story_l5.rs` so the logic lives in exactly one place.
//! The gate's own unit tests (including the provable 100× regression failure) live in the
//! sibling `perf_gate_l1.rs` test binary, which `#[path]`-includes this file. This file
//! itself is a (test-free) integration target — hence the blanket `dead_code` allow.

#![allow(dead_code)]

use std::path::Path;

/// Outcome of evaluating the perf gate — a value (not a panic) so it can be unit-tested.
#[derive(Debug, PartialEq, Eq)]
pub enum GateResult {
    /// Both obligations met (or no baseline yet, in which case only the budget applies).
    Pass,
    /// The absolute I6 budget was breached.
    OverBudget,
    /// A baseline exists and the run drifted more than `delta_factor`× slower than it.
    Regressed,
}

/// Evaluate the two-part gate. `baseline` is `None` when no tracked baseline exists yet
/// (first run / fresh checkout) — then only the absolute budget applies. When a baseline
/// IS present, BOTH the budget AND the delta must hold.
pub fn evaluate_gate(
    elapsed: f64,
    baseline: Option<f64>,
    budget_secs: f64,
    delta_factor: f64,
) -> GateResult {
    if elapsed >= budget_secs {
        return GateResult::OverBudget;
    }
    if let Some(base) = baseline {
        if elapsed > base * delta_factor {
            return GateResult::Regressed;
        }
    }
    GateResult::Pass
}

/// Read a tracked perf baseline if present. Returns `None` when the file does not exist
/// (so a fresh checkout / first CI run does not fail the delta arm) — but NEVER writes,
/// so the committed baseline is the single stable comparison point and is only updated
/// deliberately (by a human editing the tracked file), not silently each run.
pub fn read_baseline(path: &Path) -> Option<f64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
}

/// Run the gate and panic with a descriptive message on failure. Keeps the SAMPLE
/// emission at the call site; this only enforces the two obligations.
pub fn enforce_gate(
    label: &str,
    elapsed: f64,
    baseline: Option<f64>,
    budget_secs: f64,
    delta_factor: f64,
) {
    match evaluate_gate(elapsed, baseline, budget_secs, delta_factor) {
        GateResult::Pass => {}
        GateResult::OverBudget => panic!(
            "{label}: exceeded the {budget_secs}s budget: {elapsed:.3}s (I6 absolute budget)"
        ),
        GateResult::Regressed => panic!(
            "{label}: drifted >{delta_factor}x slower than the tracked baseline \
             ({:.3}s -> {elapsed:.3}s)",
            baseline.unwrap_or_default()
        ),
    }
}
