# PR Review — item #9 · One-page cover letter

**Range:** `main..item-9-one-page-letter` · **Date:** 2026-06-13 · **Governance:** `pr-approval`
**Verdict:** **PASS** (max-severity rule over all lenses; one MEDIUM finding closed before sign-off)

This review gates; the human merges after verifying gates.

## What changed
- `crates/core/src/lib.rs` — private `truncate_ellipsis(s, max_chars)` (unicode-safe, word-boundary,
  panic-free) bounds each cover-letter strength to ≤200 chars and `whyRole` to ≤300 chars in
  `build_cover_letter`; evidence ids untouched. New L1 unit tests for the helper + budget tests.
- `templates/letter/classic-letter.typ` — 14pt header (was 18pt), tighter leading/spacing, strengths as
  a compact bulleted `#list`. Item-8b SAMPLE watermark block byte-intact.
- `crates/core/Cargo.toml` — `lopdf = { version = "0.38", default-features = false }` dev-dep (already in
  Cargo.lock transitively via pdf-extract → no new vendored crate).
- `crates/core/tests/cover_letter_one_page.rs` — deterministic page-count==1 proof via the PDF page tree
  (lopdf), incl. a long-content fixture and a non-vacuous load-bearing coordinate.
- `ROADMAP.md` — item #9 → DONE.

## Adversarial panel

| Lens | Verdict | Notes |
|---|---|---|
| CORRECTNESS | PASS | Could not break `truncate_ellipsis` (char-boundary safe incl. multi-byte whitespace; never exceeds budget for the real consts; deterministic). Raised one MEDIUM test-non-vacuity finding (now closed). Evidence-id preservation proven. |
| REGRESSION | PASS | 8b watermark 5/5 green (empty `#list` renders fine); item-#3 rewrite path semantics unchanged (truncates seed text only, id preserved); empty-field `whyRole` still contains "this role"/"your team"; no new vendored package; coverage floor holds with no new pragmas. |
| PERFORMANCE | PASS | No cover-letter perf baseline is regressed; the STORY perf-delta gate is one-directional (fires only on >3× slowdown) and content-shrink is strictly faster. `truncate_ellipsis` is O(n) for n≤300. |
| SECURITY | PASS | lopdf is dev-dep only (absent from the non-dev graph), parses self-rendered PDFs in tests only; new fixture is synthetic/PII-free (example.com, lorem); strength text is serde-serialized into a Typst `--input data=` JSON value (escaped, no injection path). |

## Findings

| # | Severity | Locus | Status |
|---|---|---|---|
| 1 | MEDIUM | `tests/cover_letter_one_page.rs` — hostile fixture page-count arm was vacuous w.r.t. the truncation budget (template + 3-cap deliver one page even without truncation; overflow begins ~2000+ chars/strength). | **RESOLVED** — added `budget_is_load_bearing_raw_is_multipage_budgeted_is_one_page`: the SAME ~2600-char content renders ≥2 pages raw and ==1 page budgeted; removing `truncate_ellipsis` makes it FAIL (empirically verified). |
| 2 | LOW | Empty-job `whyRole` reads "for the **this role** position at **your team**" (grammatically awkward). | Accepted — the existing scaffold-defaults test requires those exact literal tokens; cosmetic only. |
| 3 | LOW | `truncate_ellipsis` returns `"…"` (1 char) for `max_chars == 0` on over-length input. | Accepted — helper is private with two call sites passing compile-time consts 200/300; `max_chars` is never 0/1; unreachable in production. |

## Gates (re-run by FOUNDER after the fix)
- `cargo fmt --all --check` → clean
- `cargo clippy --all-targets -- -D warnings` → clean (default features; `--all-features` is a pre-existing
  `embedded-typst` build break, DISCUSS-RENDER, not a CI gate)
- `cargo test --workspace` → all suites green, 0 failures
- `cargo test -p aa-core --test cover_letter_one_page` → 4 passed (raw ≥2 pages, budgeted ==1 page)
- `cargo test -p aa-core --test watermark_render` → 5 passed (8b regression)
- coverage floor `--ignore-filename-regex 'crates/cli/' --fail-under-lines 99` → **99.34% lines, exit 0**,
  no new pragmas

## Not reviewed
- `--all-features` / `embedded-typst` path (pre-existing DISCUSS-RENDER build break in this mirror; not a
  CI gate — items 1–8b shipped under the default-feature gate).
- The `ui` (vitest) job — unaffected by this Rust/Typst-only change.
- Visual/typographic fidelity beyond page count (out of scope; design tokens matched to
  `doc/design/pdf-look.md`).
