# First vertical slice — Applicant Advocate

The **smallest shippable, end-to-end increment** that proves the core value. Per the user's decision,
it is a **full Tauri vertical** (real desktop UI + engine) — kept thin by deferring import parsing,
the LLM layer, capture, and tracking.

## The slice, end to end
1. **Onboard** — user imports a **master-CV JSON** (canonical schema). Validated on load; stored in the
   encrypted local DB (SQLite + SQLCipher).
2. **Bring in a job** — user **pastes a JD** into the app. Deterministic parse/normalize →
   **Normalized Job JSON** (title, company, location, responsibilities, must/nice requirements, keywords).
3. **See the fit** — deterministic engine matches requirements → evidence; shows a **coverage report**
   (must/nice covered vs gaps) and a **fit score**.
4. **Tailor** — engine **selects and reorders** master-CV bullets (never invents); user can
   **approve/reject** bullets in the review UI. Strongest aligned evidence first.
5. **Generate** — render the **tailored CV → PDF via embedded Typst** (`templates/cv/classic.typ`) +
   a **templated draft cover letter**.
6. **Export** — user exports the PDF locally. Nothing is uploaded; nothing is submitted.

## Explicitly NOT in this slice
PDF/DOCX import · the LLM Applicant Advocate rewrite · the browser capture extension · email
ingestion · application tracker / CRM / call sheet · any LinkedIn/Seek automation.

## Proves the core
The evidence-grounded, deterministic, local, honest tailoring loop — the wedge — works end to end and
produces a polished artefact, on-device, in seconds.

## Acceptance (testable)
- Fixture: a synthetic persona (`fixtures/personas/*.cv.json`) + a synthetic JD (`fixtures/jobs/*.json`).
- Output: a valid tailored CV PDF + a draft cover letter, produced in < 60 s, fully offline.
- **Integrity:** every rendered bullet maps to an evidence id in the master CV (automated check);
  an injected unsupported claim is blocked/flagged.
- A coverage report enumerates must-have / nice-to-have requirements with covered/uncovered status.

## Algorithms to pin (slice 1 — fully specified, no black boxes)

These close the deterministic mechanisms that are the product's wedge, so a fresh builder need not
guess. The richer brief formula (`doc/001-design-brief.md`) is a *later* item, not slice 1.

### A. Skill/keyword matching (the matching primitive)
A requirement is **matched** iff any of its normalized tokens equals (case-insensitive) a token or
alias of: any skill (all four categories), any experience `tag`, or any token of an achievement
`description`/`tags`. Normalization = lowercase + a small alias map (`js→javascript`,
`ts→typescript`, `k8s→kubernetes`, `ci/cd→continuous integration`, …) seeded in `crates/core`.

### B. Coverage (fully defined, drives the testable metric)
- `must_have_coverage = matched_must / total_must` · `nice_have_coverage = matched_nice / total_nice`.
- The **coverage report** lists each must/nice requirement with covered (true/false) + the matching
  evidence ids. This is the testable success criterion.

### C. Fit score (slice 1 = the minimal defined form)
`fit_score = 0.6 * must_have_coverage + 0.4 * nice_have_coverage` (0–1). Title/domain/seniority/
recency alignment and penalties from the brief's full formula are **DEFERRED** (a later "scoring v2"
item) — the schema has no seniority field and those terms need a taxonomy not in slice 1. The fit
score is **not** part of acceptance; coverage is.

### D. Bullet selection / ordering (deterministic)
Rank achievements by: (1) matches a must-have, (2) has `metrics`, (3) higher `evidenceStrength`,
(4) recency (parsed `startDate`), (5) `emphasise`. Select top-N per role to fit one–two pages; never
invent. Tailored **summary** = the `summaryVariants` entry with the most requirement-token overlap
(verbatim; carries provenance `summary:<index>`) — no free-text generation in slice 1.

### E. Evidence-ledger guard (the headline wedge — exact contract)
Every **claim-bearing output node** carries a `sourceEvidenceId`:
- CV bullet → the achievement `id` it was copied from (verbatim).
- CV summary → `summary:<index>`.
- Cover-letter **strength** paragraph → the achievement `id` it wraps.
- Cover-letter **boilerplate** (greeting, why-company/role from job fields) → marked `scaffold`,
  exempt (it asserts no experience claim).

The guard asserts **every claim-bearing node has a `sourceEvidenceId` resolvable in the loaded master
CV**; on failure it **blocks export** (hard fail) and names the node. In slice 1 (verbatim
select/reorder) this is an invariant that should always hold — it becomes load-bearing when the LLM
layer arrives, but ships from day 1. **Non-vacuous test:** a fixture injects a tailored node with a
dangling `sourceEvidenceId` (and a variant whose text matches no master-CV achievement) and asserts
the guard blocks it.

### F. JD parse — must/nice classification (pinned cues + accepted risk)
Split requirements by cue phrases: **must** = `required | must have | essential | you will need |
minimum | mandatory`; **nice** = `preferred | desirable | bonus | nice to have | advantageous |
ideally`. An unmarked requirement defaults to **nice-to-have**. On the synthetic JD fixtures (authored
with clear cues) classification is 100%. Robustness on free-form real-world JDs is an **accepted risk
(R6)** — see `handoff.md` — not part of the slice-1 bar.

### G. Cover-letter output (format + template)
A **second Typst PDF**, `templates/letter/classic-letter.typ` (to be authored, matching the CV look),
plus editable text shown in the UI before render. Structure: greeting → why-this-role/company
(templated from job title + company = scaffold) → 2–3 **strength** paragraphs, each wrapping one
selected achievement (carrying its evidence id) → close. **Export produces two PDFs** (cv.pdf +
cover-letter.pdf).

### H. Typst embedding contract (Rust, no shell-out)
Use the `typst` crate (or `typst-as-lib`) with a custom `World` that provides **bundled fonts**
(ship Liberation Sans/Mono with the app — no system dependency) and an in-memory virtual filesystem.
The tailored-view JSON is exposed at a fixed virtual path `/view.json` and `sys.inputs.data` is set to
it, so the **same `classic.typ` renders both via CLI and embedded** (the existing data-loader already
resolves a root-relative path). `classic.typ` may be lightly adapted but must stay CLI-renderable so
the fixture render test keeps working. **The tailored-view JSON conforms to
`master-cv.schema.json`** — it is a filtered/reordered master CV (a view, never a bespoke object), so
`classic.typ` (which consumes a full master-CV document) renders it unchanged.

## Suggested build decomposition (for FOUNDRY)
- `crates/core` (Rust) — schema types, normalize, deterministic match/score, tailored-view selection,
  evidence-ledger guard, Typst render (embedded). *(handler-rust)*
- `crates/jobparse` (Rust) — JD text → Normalized Job JSON (rule/regex/heading segmentation).
- `apps/desktop` (Tauri) — React/TS UI: onboarding, JD paste, coverage/review screen, preview, export;
  Tauri commands bind to `core`. *(handler-react + handler-rust)*
- Tests: unit (normalize, scoring, ranking, ledger guard, render) + fixture E2E, all on synthetic data.
