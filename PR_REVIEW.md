# Adversarial PR Review — item #6 (templates + ATS-readability + keyword-coverage)

**Range:** `main..item-6-templates-ats` · **Date:** 2026-06-13 · **Governance:** `pr-approval` (always-on adversarial gate; human merges after PASS)
**Final verdict:** **PASS** (after one NEEDS_REVISION round — a HIGH correctness defect was found, fixed, and re-verified)

## Roles fanned out (each prompted to refute the change)

| Role | Verdict | Notes |
|---|---|---|
| CORRECTNESS | PASS (after fix) | Found 1 HIGH (keyword skill-blindspot) — fixed + re-reviewed clean |
| REGRESSION | PASS | Render seam additive + backward-compatible; full suite 0 failures; items 1–5 untouched |
| ARCHITECTURE | PASS | Pure cores (no IO), one-way dep graph, read-only, deterministic ordering |
| PERFORMANCE | PASS | New STORY perf-delta gate non-vacuous (baseline 0.300s ≈3.5× observed 0.084s; not a copy; registered) |
| SECURITY / PII | PASS | No PII, no new network/IO, no stuffing/fabrication, no template path-traversal (enum allow-list) |
| DOCUMENT | PASS | EARS R-TPL/R-ATS/R-KWC traceable; plan §6.0 decisions match shipped artifacts; ROADMAP/pdf-look accurate |

## The HIGH finding (resolved)

**CORRECTNESS #1 (HIGH, gating → FIXED).** `keyword_coverage::surfaced_evidence` intersected the
heterogeneous `matching_evidence_ids` (skill `evidenceIds` + experience ids + achievement bullet ids)
with `view.selected_ids` (achievement bullet ids ONLY). Consequence: any keyword matched via a declared
**skill name** was wrongly reported MISSING, even though the Skills section renders unconditionally in
both templates. Proven over persona-001: all declared skills → MISSING even at top_n=1000.

**Fix (approach A — tagged ids).** `matching.rs` gained `EvidenceKind {Skill, Experience, Achievement}` +
`EvidenceId {id, kind}` + `matching_evidence_ids_kinded`; the existing `matching_evidence_ids` delegates
(byte-identical output, so `coverage.rs` is unaffected). `surfaced_evidence` now gates **only**
`Achievement` ids on `selected_ids`; `Skill`/`Experience` ids are always surfaced (those sections always
render). Three regression tests added (skill-name FOUND; experience-token FOUND; skill-evidence always
surfaced) — RED before the fix, GREEN after — and the must-not-regress invariant
(`keyword_found_only_in_dropped_bullet_is_not_surfaced`) still passes.

**Re-review:** CORRECTNESS re-ran adversarially → PASS. Defect fixed, invariant intact, delegation
byte-identical, classification correct per namespace.

## Residual non-gating items (SUGGESTIONs — not blocking)

- **heading_vocabulary(self) ignores self** (render.rs): returns one hand-maintained list for both
  templates. Harmless today (enum is exactly `{Classic, Compact}`, both emit that vocabulary, Modern
  omitted); R-ATS-5 is explicitly scoped as a future-template guard. Re-check when a 3rd variant is added.
- **EmbeddedRenderer** does not override `render_cv_with_template` → under the (CI-deferred,
  non-default) `embedded-typst` feature a Compact selection silently renders Classic. Documented
  deferral; the shipping CLI path is correct. Override (or error) when the embedded backend ships.
- **Evidence-id provenance collision** (matching.rs): a skill `evidenceIds` string that coincides with a
  pruned achievement bullet id would display that pruned id as provenance. Cosmetic only (never a wrong
  FOUND/MISSING verdict); 0 occurrences in real fixtures (all persona skills carry empty evidenceIds).
- **CliRenderer temp-file** (pre-existing item-1 code, out of scope): predictable repo-root temp name;
  worth a future hardening pass to a private temp dir.
- **KAIZEN:** `job.keywords` is parsed but unused by coverage and capability C (keyed off
  `requirements`) — a reserved dead-contract field to wire or document in a future cycle.

## What was NOT independently reviewed

- The React/Vitest UI suite (`apps/desktop`) runs under the `ui` CI job, which is `continue-on-error`
  (issue #2 — runners cannot reach any npm registry). UI tests were confirmed **passing locally** (24/24)
  but the green is self-attested, not observed by the CI gate. The user-facing journey is independently
  proven by the Rust command-level STORY (L5).

## Gates (verified by FOUNDER directly, post-fix)

```
cargo fmt --all --check                               → exit 0
cargo clippy --workspace --all-targets -- -D warnings → exit 0
cargo test --workspace                                → 0 failures (all bins)
cargo llvm-cov --workspace --fail-under-lines 99      → TOTAL 99.26% lines (ats.rs 100%, keyword_coverage.rs 100%)
STORY L5 templates_ats_story_l5                       → 0.084s ≤ 0.300s baseline (perf-delta gate green)
typst compile templates/cv/compact.typ … persona-001 → valid PDF (49528 bytes); classic still renders
npm test (apps/desktop)                               → 24/24 (local; ui job non-blocking per issue #2)
```
