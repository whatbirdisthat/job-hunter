# Tracked perf baselines (L5 STORY perf-delta gates)

These files are the **committed, stable** baselines the L5 STORY perf-delta gates compare
against (item #2, Finding 3). They are version-controlled on purpose so CI has a fixed point
of comparison — they are **not** rewritten on every test run (the previous gate wrote to a
gitignored `target/` file, which self-ratcheted and made the delta arm vacuous).

| File | Gated test |
|---|---|
| `cvimport-import-story-baseline.txt` | `crates/cvimport/tests/story_l5.rs` |
| `desktop-story-baseline.txt` | `apps/desktop/src-tauri/tests/story_l5.rs` (deterministic export) |
| `desktop-advocate-story-baseline.txt` | `apps/desktop/src-tauri/tests/story_l5.rs` (item #3 advocate-rewrite journey, flag ON + stub) |
| `capture-clip-story-baseline.txt` | `packages/capture-core/test/story.test.mjs` (item #4 clip→json journey, `node --test`) |
| `capture-email-story-baseline.txt` | `packages/capture-core/test/story.test.mjs` (item #4 email→jobs journey, `node --test`) |
| `desktop-tracker-story-baseline.txt` | `apps/desktop/src-tauri/tests/tracker_story_l5.rs` (item #5 tracker journey: track → advance → call-sheet) |

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
headroom over the observed steady-state (~0.17 s cvimport, ~0.09 s desktop deterministic,
~0.09 s desktop advocate-rewrite, ~0.001 s desktop tracker) so normal machine/CI variance does
not flake the gate while a real regression still trips it. The advocate journey adds only the
deterministic stub rewrite (no network) on top of the deterministic export, so its steady-state
matches the deterministic desktop story.

**Rule: baseline ≈ 3–5× observed steady-state; never copy another story's value.** Set the
baseline a few × above the measured time so the delta arm has just enough headroom to ride out
CI variance while still tripping on a genuine regression — NOT orders of magnitude above it (a
grossly loose baseline makes the 3×-delta arm vacuous, leaving only the 60 s budget as a real
check).

The item-#5 tracker journey is pure in-memory cores + a tiny atomic JSON file write (no typst
render), so its **observed steady-state is ~0.001 s** (measured 3× via
`cargo test -p aa-desktop --test tracker_story_l5 -- --nocapture`). Its baseline is **`0.030000`**
(≈30× margin — item #5 Finding 3, replacing the old `0.500000` which was ~500× the runtime, so
loose that the 3×-delta arm at 1.5 s was effectively dead). At `0.030000` the delta arm fires at
0.090 s, so a multi-hundred-ms regression FAILS while the 0.001 s steady-state passes with
comfortable headroom; the 60 s absolute budget remains the hard ceiling.
