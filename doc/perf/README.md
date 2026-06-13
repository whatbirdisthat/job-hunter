# Tracked perf baselines (L5 STORY perf-delta gates)

These files are the **committed, stable** baselines the L5 STORY perf-delta gates compare
against (item #2, Finding 3). They are version-controlled on purpose so CI has a fixed point
of comparison — they are **not** rewritten on every test run (the previous gate wrote to a
gitignored `target/` file, which self-ratcheted and made the delta arm vacuous).

| File | Gated test |
|---|---|
| `cvimport-import-story-baseline.txt` | `crates/cvimport/tests/story_l5.rs` |
| `cvimport-jsonmine-story-baseline.txt` | `crates/cvimport/tests/mine_json_story_l5.rs` (item 8a adaptive JSON-miner journey: mine → completeness → schema-validate) |
| `desktop-story-baseline.txt` | `apps/desktop/src-tauri/tests/story_l5.rs` (deterministic export) |
| `desktop-advocate-story-baseline.txt` | `apps/desktop/src-tauri/tests/story_l5.rs` (item #3 advocate-rewrite journey, flag ON + stub) |
| `capture-clip-story-baseline.txt` | `packages/capture-core/test/story.test.mjs` (item #4 clip→json journey, `node --test`) |
| `capture-email-story-baseline.txt` | `packages/capture-core/test/story.test.mjs` (item #4 email→jobs journey, `node --test`) |
| `desktop-tracker-story-baseline.txt` | `apps/desktop/src-tauri/tests/tracker_story_l5.rs` (item #5 tracker journey: track → advance → call-sheet) |
| `desktop-templates-ats-story-baseline.txt` | `apps/desktop/src-tauri/tests/templates_ats_story_l5.rs` (item #6 journey: pick Compact → export → ats_report → keyword_coverage) |
| `cli-ingestion-story-baseline.txt` | `crates/cli/tests/ingestion_story_l5.rs` (item 8b journey: strict-then-mine → `--use-fakes` → watermarked `cv.SAMPLE.pdf`; normal export refused without the flag) |

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

The item-#6 templates-ATS journey runs ONE typst CV render (the dominant cost) plus two pure
in-memory reports (`ats_report`, `keyword_coverage`), so its **observed steady-state is ~0.084 s**
(measured 3× via `cargo test -p aa-desktop --test templates_ats_story_l5 -- --nocapture`: 0.086 /
0.083 / 0.084 s). Its baseline is **`0.300000`** (≈3.5× the observed steady-state, per the rule
above — NOT copied from another story). At `0.300000` the 3×-delta arm fires at 0.900 s, so a
real multi-hundred-ms regression FAILS while the 0.084 s steady-state passes with headroom; the
60 s absolute budget remains the hard ceiling.

The item-8a adaptive JSON-miner journey is a pure in-memory `serde_json::Value` walk
(`import_cv_json`) + `completeness` + ONE `validate.js` node subprocess (the dominant cost is
the node spawn, not the mine). Its **observed steady-state is ~0.022 s** (measured 3× via
`cargo test -p aa-cvimport --test mine_json_story_l5 -- --nocapture`: 0.022 / 0.023 / 0.021 s).
Its baseline is **`0.075000`** (≈3.4× the observed steady-state, per the rule above — NOT copied
from the import-story baseline, whose PDF/DOCX render+extract journey is orders of magnitude
slower, which would make the delta arm vacuous). At `0.075000` the 3×-delta arm fires at 0.225 s,
so a real multi-hundred-ms regression FAILS while the 0.022 s steady-state passes with headroom;
the 60 s absolute budget remains the hard ceiling.

The item-#5 tracker journey is pure in-memory cores + a tiny atomic JSON file write (no typst
render), so its **observed steady-state is ~0.001 s** (measured 3× via
`cargo test -p aa-desktop --test tracker_story_l5 -- --nocapture`). Its baseline is **`0.030000`**
(≈30× margin — item #5 Finding 3, replacing the old `0.500000` which was ~500× the runtime, so
loose that the 3×-delta arm at 1.5 s was effectively dead). At `0.030000` the delta arm fires at
0.090 s, so a multi-hundred-ms regression FAILS while the 0.001 s steady-state passes with
comfortable headroom; the 60 s absolute budget remains the hard ceiling.
