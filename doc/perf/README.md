# Tracked perf baselines (L5 STORY perf-delta gates)

These files are the **committed, stable** baselines the L5 STORY perf-delta gates compare
against (item #2, Finding 3). They are version-controlled on purpose so CI has a fixed point
of comparison — they are **not** rewritten on every test run (the previous gate wrote to a
gitignored `target/` file, which self-ratcheted and made the delta arm vacuous).

| File | Gated test |
|---|---|
| `cvimport-import-story-baseline.txt` | `crates/cvimport/tests/story_l5.rs` |
| `desktop-story-baseline.txt` | `apps/desktop/src-tauri/tests/story_l5.rs` |

Each file holds a single number: the baseline wall-clock in seconds. The gate enforces TWO
independent obligations (see `crates/cvimport/tests/perf_gate.rs`):

1. **Absolute budget (I6):** `elapsed < 60s`.
2. **Regression delta:** `elapsed <= baseline * 3.0` — fires independently of the budget, so a
   >3× regression fails even though it is far under the 60 s budget.

The gate's non-vacuity (a simulated 100× regression FAILS) is proven by the unit tests in
`crates/cvimport/tests/perf_gate_l1.rs`.

## Updating a baseline

Only update a baseline **deliberately** — when a genuine, reviewed performance change moves the
honest steady-state time. Measure with `cargo test -p <crate> --test story_l5 -- --nocapture`
(read the emitted `[L5 STORY perf] … ` line) and edit the value. The current values carry
headroom over the observed steady-state (~0.17 s cvimport, ~0.09 s desktop) so normal
machine/CI variance does not flake the gate while a real regression still trips it.
