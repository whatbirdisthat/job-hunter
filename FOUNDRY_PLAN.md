# FOUNDRY Plan — job-hunter (Applicant Advocate) — ROADMAP item #1 — 2026-06-12

> Scope: **ROADMAP item #1 ONLY** — "First slice — JD → tailored CV + cover-letter PDF (full Tauri
> vertical)", PRIORITY: NOW. The LATER items (PDF/DOCX import, LLM layer, capture extension, tracker,
> automation) are **explicitly out of scope** and not planned here.
> Contract documents: `SUBJECT_MATTER_UNDERSTANDING.md` (this repo) + `doc/idea/applicant-advocate/`
> (`brief`, `smu-seed`, `first-slice` §A–H, `handoff`). The pinned algorithms §A–H are the build
> contract — encoded verbatim in the SMU; downstream agents honour them exactly.
> Builder-lead plans; **`lifecycle-orchestrator` runs the per-item 0–9 loop.** No production code here.

---

## Branch

```
slice-1-jd-to-tailored-cv
```

---

## Stack Manifest

- **Languages:** Rust 1.96.0, TypeScript 5.x (React), Typst 0.14.2 markup.
- **Frameworks:** Tauri 2.x (desktop shell), React + Vite (UI).
- **Test runners:** `cargo test` (Rust unit/module/boundary/system), Vitest (UI unit), Tauri/WebDriver
  or Playwright for the STORY journey (see Test Contract — driver choice is a DISCUSS item, D3).
- **Embedded render:** `typst` crate (or `typst-as-lib`) — custom `World`, bundled Liberation
  Sans/Mono fonts, in-memory VFS (§H). **No shell-out** in the app path.
- **Persistence:** SQLite + SQLCipher (encrypted local store).
- **Serde:** `serde` / `serde_json` (parse-don't-validate types), `thiserror` (typed errors).
- **Tooling (reuse):** Node 24 + `jq` for fixtures/schema (`tools/fake-data/`), existing CI.

Verified present (2026-06-12): cargo 1.96, rustc 1.96, node 24.16, npm 11.13, typst 0.14.2, jq, git.

---

## Subject Matter Understanding — Status

`SUBJECT_MATTER_UNDERSTANDING.md` **written** at repo root (expanded from `smu-seed.md`): domain
concepts, §A–H restated as the build contract, invariants I1–I6 (immutable master CV + evidence
ledger), design-value tie-breakers, stack/handler map. Complete for item #1.

---

## Architecture Decisions (Phase 2.5)

Item #1 crosses integration boundaries (new persistence: SQLite+SQLCipher; new delivery channel:
embedded Typst render; new bounded contexts: JD parsing, tailoring engine). However, the **pattern is
already pinned by the IDEA package** — the crate decomposition, the tailored-view-as-schema-conformant-
view contract (§H), and the ledger guard (§E) constitute the architecture decision, made upstream by
FOUNDER/ideator and recorded in `first-slice.md`. No fresh `handler-architect` ADR is required for the
*shape*. **One conditional architect spawn** is recommended if, during decomposition, the embedded
Typst `World` design (font provisioning + VFS lifetime + thread-safety across Tauri commands) proves
non-trivial — see Self-Improvement Flag SI-2. Absent that, Phase 2.5 is satisfied by the pinned spec.

---

## Crate graph (one-way dependency diagram + the rule)

```
            doc/schemas/master-cv.schema.json   ← shared DATA CONTRACT (not code)
                 ▲                       ▲
                 │ (types conform)       │ (tailored-view conforms, §H)
                 │                       │
   ┌─────────────┴──────┐     ┌──────────┴───────────────────────────────┐
   │   crates/jobparse  │     │             crates/core                  │
   │  JD text →         │     │  schema types (parse-don't-validate) ·   │
   │  Normalized Job    │     │  normalize+alias (§A) · match (§A) ·      │
   │  JSON (§F)         │     │  coverage (§B) · fit (§C) · ranking (§D) ·│
   │                    │     │  tailored-view assembly (§H) ·            │
   │  depends on:       │     │  ledger guard (§E) · embedded render (§H) │
   │  serde only        │     │  depends on: serde, thiserror, typst      │
   └─────────┬──────────┘     └──────────┬───────────────────────────────┘
             │                           │
             │   Normalized Job JSON     │   master CV + tailored view + render
             └───────────┬───────────────┘
                         ▼
            ┌────────────────────────────────────┐
            │   apps/desktop (Tauri)             │
            │   src-tauri (Rust commands) →      │
            │     binds to core + jobparse       │
            │   src (React/TS UI)               │
            │   SQLite + SQLCipher store         │
            └────────────────────────────────────┘
```

**Dependency rule (decided, binding):**
- **`crates/core` MUST NOT depend on `crates/jobparse`.** Core operates on a **Normalized Job** value
  (a plain typed struct it defines or shares via the schema contract); it never reaches into jobparse's
  parsing/IO internals.
- **`crates/jobparse` MUST NOT depend on `crates/core`.** Its sole output is the Normalized Job JSON.
- The two crates are siblings; **`apps/desktop` is the only place that depends on both** and wires the
  seam: `jobparse` output → `core` input.
- The **shared contract is data, not code**: the master-CV schema and the Normalized-Job shape. The
  **tailored view is a schema-conformant master-CV document** (§H) — never a bespoke object. This keeps
  `classic.typ` rendering it unchanged and keeps the core↔jobparse seam a pure value boundary.
- Topological order (no cycle — Phase 4.5 P2-7 pass): `{core, jobparse}` (parallel) → `desktop`.

---

## Station / Handler map (+ FOUNDER findings)

| Task area | Handler |
|---|---|
| `crates/core` (all §A–E, §H render) | **handler-rust** |
| `crates/jobparse` (§F) | **handler-rust** |
| `apps/desktop/src-tauri` (commands, SQLCipher) | **handler-rust** |
| `apps/desktop/src` (React/TS UI) | **handler-react** |
| `templates/letter/classic-letter.typ` (§G) | **handler-rust** (owns render path; template authored + render-tested here) |

**FOUNDER finding F-1 (station-map gap, carried from SMU §8):** no dedicated **Tauri/SQLCipher**
value-handler exists in the roster. Mapping the Tauri command layer to **handler-rust** and the UI to
**handler-react** is the closest viable mapping and is adequate for slice 1 (thin command-binding layer,
no Tauri-specific domain logic). Recorded for the KAIZEN covenant; propose `handler-tauri` only if a
later slice grows substantial IPC/SQLCipher logic. **No new handler improvised.**

**Roster cross-check (Phase 4.5 P2-4):** the only VALUE_HANDLERS named are `handler-rust` and
`handler-react`, both confirmed by the dispatch brief as available. No missing handler blocks item #1.
Reviewer roles named below (EARS, FEATURE/Gherkin, TEST, IMPLEMENT, STORY, plus REGRESSION-REVIEWER /
COVERAGE-REVIEWER at the lifecycle gates) are the standard lifecycle roster — see WARN W-1 if any
prove unregistered when `lifecycle-orchestrator` resolves them.

---

## Shared Infrastructure Map (Phase 3 — highest leverage)

Within a single-item slice, "shared" = components consumed by ≥2 tasks/crates. Build once, in `core`,
behind the schema contract.

| Component | Needed by | Build in | Rationale (build-once leverage) |
|---|---|---|---|
| Master-CV serde types (parse-don't-validate) | core (all algos), desktop (commands), tests (all levels) | **core (Task C1)** | Every algorithm and the Tauri seam deserialize the same types. Authoring twice = drift between engine and command layer. |
| Normalize + alias map (§A) | match (§A), coverage (§B), summary pick (§D), jobparse token compare | **core (Task C2)** | The matching primitive underlies coverage, fit, ranking and the summary choice. One canonical normalizer. |
| Normalized-Job type | jobparse (output), core (input), desktop (seam) | **core defines, jobparse + desktop consume** | The core↔jobparse value contract. Defining it in `core` keeps the rule "jobparse → no dep on core internals" honest (it depends only on a shared type, ideally a tiny shared contract; if cross-crate coupling is undesirable, mirror via the schema — DISCUSS D1). |
| Tailored-view assembler (schema-conformant) | render (§H), ledger guard (§E), desktop preview/export | **core (Task C6)** | The view is the universal currency: ledger guard validates it, Typst renders it, UI previews it. One assembler, one shape. |
| Embedded Typst `World` (fonts + VFS at `/view.json`) | CV render, cover-letter render | **core (Task C8)** | Both PDFs share the World, fonts, and VFS mechanism. Build once; render two templates through it. |
| Fixtures + schema validate (existing) | every test level | **reused** (`fixtures/`, `tools/fake-data/`) | Already authored; the test oracle. Do not re-author. |

Net: the normalize/match primitive and the tailored-view assembler are each built **once** in `core`
and reused across coverage, fit, ranking, ledger, render, and the Tauri seam — the dominant token saving.

---

## Token Budget Summary

No `IDEA_COST.jsonl` exists in this repo → **no historical comparables**. All estimates use the
priority→tier heuristic (`estimation_basis: HEURISTIC`). Item #1 is a single PRIMARY-tier vertical
slice; budgets are per-task, summed.

| Task | Handler | Est. tokens | Basis |
|---|---|---|---|
| C1 core types | handler-rust | ~6k | HEURISTIC |
| C2 normalize+alias (§A) | handler-rust | ~5k | HEURISTIC |
| C3 match primitive (§A) | handler-rust | ~5k | HEURISTIC |
| C4 coverage (§B) | handler-rust | ~5k | HEURISTIC |
| C5 fit score (§C) | handler-rust | ~3k | HEURISTIC |
| C6 ranking + summary + view assembly (§D, §H) | handler-rust | ~9k | HEURISTIC |
| C7 ledger guard (§E, non-vacuous test) | handler-rust | ~6k | HEURISTIC |
| C8 embedded Typst render (§H) | handler-rust | ~10k | HEURISTIC (World/VFS is the riskiest piece) |
| J1 jobparse (§F) | handler-rust | ~8k | HEURISTIC |
| T1 cover-letter template (§G) | handler-rust | ~6k | HEURISTIC |
| D1 Tauri commands + SQLCipher store | handler-rust | ~10k | HEURISTIC |
| D2 React/TS UI (5 screens) | handler-react | ~14k | HEURISTIC |
| STORY journey + perf-delta harness | (test) | ~6k | HEURISTIC |
| CI extension (rust fmt/clippy/test) | handler-rust | ~3k | HEURISTIC |
| **Total (item #1)** | | **~96k** | sum |

After cycle completion, `lifecycle-orchestrator` should write actuals to `IDEA_COST.jsonl`; builder-lead
reviews estimate accuracy under KAIZEN next cycle.

---

## Work Decomposition (item #1)

### Item #1 — First slice: JD → tailored CV + cover-letter PDF

**Tier:** PRIMARY · **Priority:** NOW (HIGH) · **Budget:** ~96k (HEURISTIC) ·
**Depends on:** none · **Parallel-safe with:** n/a (sole item this cycle).

The slice is decomposed into the tasks below. Each task is test-first (EARS → Gherkin → failing tests →
implement → story), owned by the named handler. **Tasks ordered by the topological build order** within
the slice; `core` and `jobparse` task families are mutually parallel-safe (no shared files, no
dependency); `desktop` and the STORY journey come last.

| ID | Task | Handler | §ref | Depends on (intra-slice) | Parallel-safe with |
|---|---|---|---|---|---|
| C1 | core: master-CV serde types (parse-don't-validate, typed `thiserror`) | handler-rust | types | — | J1 |
| C2 | core: normalize + alias map | handler-rust | §A | C1 | J1 |
| C3 | core: match primitive | handler-rust | §A | C2 | J1 |
| C4 | core: coverage report | handler-rust | §B | C3 | J1 |
| C5 | core: fit score | handler-rust | §C | C4 | J1 |
| C6 | core: bullet ranking + summary pick + tailored-view assembly | handler-rust | §D, §H | C3 | J1 |
| C7 | core: evidence-ledger guard (+ non-vacuous dangling-id test) | handler-rust | §E | C6 | J1 |
| C8 | core: embedded Typst `World` + CV render (bundled fonts, VFS `/view.json`) | handler-rust | §H | C6 | J1, T1 |
| J1 | jobparse: JD text → Normalized Job JSON (must/nice cue classification; unmarked→nice) | handler-rust | §F | — | all C* |
| T1 | author `templates/letter/classic-letter.typ` (matches CV look) | handler-rust | §G | — | C1–C7, J1 |
| C9 | core: cover-letter render through the same `World` | handler-rust | §G, §H | C7, C8, T1 | — |
| D1 | Tauri commands (import/validate, parse, tailor, render×2, export) + SQLCipher store | handler-rust | — | C1–C9, J1 | — |
| D2 | React/TS UI: onboarding/import · JD paste · coverage+review (approve/reject) · preview · export two PDFs | handler-react | — | D1 (command surface) | — |
| CI | extend `.github/workflows/ci.yml` for the Rust workspace + new tests | handler-rust | — | C1–C9, J1, D1 | — |
| ST | STORY journey + perf-delta harness | (test) | acceptance | D2 | — |

**Per task, the lifecycle conveyor runs:** EARS-AGENT → FEATURE-AGENT (Gherkin) → TEST-AGENT
(+handler) failing tests → IMPLEMENT-AGENT (+handler) → STORY-AGENT where a user-visible journey
exists. **Reviewers invoked at gates:** the standard lifecycle reviewers + REGRESSION-REVIEWER and
COVERAGE-REVIEWER (100% coverage floor). The evidence-ledger guard (C7) additionally functions as a
domain reviewer at export time.

**Build-shared callouts:** C2/C3 build the normalize+match primitive consumed by C4/C5/C6 and J1's
token compare. C6 builds the tailored-view assembler consumed by C7/C8/C9/D1/D2. C8 builds the `World`
reused by C9.

---

## Phase 4.5 — Cycle-Integrity self-heal (pre-flight gates)

- **Roster cross-check (P2-4):** PASS. Only `handler-rust` + `handler-react` named; both available.
  Station-map gap (no `handler-tauri`) **degraded** to handler-rust and recorded as FOUNDER finding
  F-1 + Self-Improvement Flag SI-1, not silently dropped.
- **Topological sort (P2-7):** PASS. Edges form a DAG: `{C1→C2→C3→{C4→C5, C6→{C7,C8}}, C8→C9,
  T1→C9}` and `{C1..C9, J1}→D1→D2→ST`; `J1` independent of all `C*`. **No cycle.** Legal build order
  exists; parallel grouping below is well-defined.
- **IDEA_COST high-variance flag (P2-9):** N/A — no `IDEA_COST.jsonl`, so no variance to compute. All
  estimates flagged `HEURISTIC`. The single watch-item is C8 (embedded Typst World) — flagged SI-2 for
  a conditional `handler-architect` consult, not a budget anomaly.
- **Catastrophic-regression policy (P2-2):** if a delivered change later collapses coverage AND fails
  the suite, `lifecycle-orchestrator` **PROPOSES a revert** to the prior-good `main` SHA (last green
  `SENTINEL::DELIVERY_COMPLETE`) and **stops — human decides**. Never auto-revert. (Stated for the
  running phase; no action at plan time.)

---

## Test Contract (five levels — non-negotiable, perf-instrumented, STORY perf-delta gated)

100% coverage is the **floor**; the only path below is an explicit pragma **with a stated reason**.
Every level emits a **perf sample** (wall-clock for the slice it exercises) accumulated against the
< 60 s offline budget (I6). Representative cases cover **empty / max / unicode / hostile-input** where
the input domain admits them.

### Level 1 — unit (CORE)
| Unit | Representative cases | Perf sample |
|---|---|---|
| normalize + alias (§A) | empty token; `JS`/`js`/`Js` → `javascript`; `CI/CD` → `continuous integration`; unicode (`café`, `Go`/`golang`); hostile: 10k-char token, control chars | per-call µs (bulk-normalize 1k tokens) |
| match primitive (§A) | matches via skill, via experience tag, via achievement description, via achievement tag; no-match; alias-only match; case-insensitive | match latency over fixture persona |
| coverage (§B) | all-covered; none-covered; mixed; **empty must bucket** (define 1.0/sentinel — tested); evidence-id list correctness | coverage over persona-001 × job-linkedin-001 |
| fit score (§C) | 0.0, 1.0, the 0.6/0.4 blend exactly; empty-bucket interaction | negligible |
| ranking + summary (§D) | total-order tie-break determinism (two equal achievements → stable by id); metrics-before-no-metrics; recency tie-break; summary verbatim pick + `summary:<index>` provenance | rank full persona |
| ledger guard (§E) | all-resolvable → pass; **dangling `sourceEvidenceId` → block + names node (non-vacuous)**; scaffold node exempt; summary variant matching no achievement → block | guard over assembled view |
| Typst render (§H) | render persona-001 view → **non-empty PDF**; empty experience array renders; unicode name renders | render wall-clock (the dominant cost) |

### Level 2 — module (crate public surface)
- `crates/core` public API: `tailor(master_cv, normalized_job) -> TailoredView`, coverage/fit
  accessors, `render(view, template) -> Pdf`, `ledger_check(view, master_cv) -> Result`. Exercise the
  public surface only (encapsulation honoured).
- `crates/jobparse` public API: `parse(raw_text) -> NormalizedJob`. Cases: the §F cue phrases (each
  must/nice cue), unmarked→nice default, empty input, multi-line headings, unicode, hostile (no cues /
  garbage → no panic, empty buckets). Perf: parse latency per fixture.

### Level 3 — boundary (seams)
- **core ↔ jobparse seam:** `jobparse.parse(fixture.descriptionRaw)` output **equals the fixture's
  structured `requirements.mustHave[]`/`niceToHave[]` oracle** (the §F expected-output oracle), then
  feeds `core` cleanly. Round-trip across all 6 job fixtures.
- **Tauri-command serialization seam:** the **tailored-view JSON conforms to `master-cv.schema.json`**
  (validate the serialized view with the existing schema validator); the **Normalized Job contract**
  serializes/deserializes losslessly across the command boundary. Hostile: oversized payload, non-UTF8
  rejected with typed error (no panic across the IPC boundary).

### Level 4 — system (assembled app path, offline)
- On synthetic fixtures, **offline** (no network): `JD-text → parse → tailor → ledger-check → render×2
  → two PDFs (cv.pdf + cover-letter.pdf)`. Assert: both PDFs non-empty; **every rendered CV bullet maps
  to an evidence id** present in the master CV (automated ledger check); coverage report enumerates
  must/nice with covered/uncovered. Injected-unsupported-claim fixture → **export blocked**. Perf
  sample: end-to-end wall-clock per (persona × job) pair, asserted **< 60 s** (I6).

### Level 5 — STORY (user journey, perf-delta gated)
- Journey: **import master CV → paste JD → see coverage → approve/reject bullets → export two PDFs**,
  driven through the real UI + command layer on a synthetic persona+job, fully offline.
- **Perf-delta gate:** baseline = the **< 60 s offline budget** (I6). The STORY test records the
  end-to-end wall-clock and **fails if it exceeds the baseline** (and flags a regression if a run
  drifts materially slower than the prior recorded story run — perf-delta, not just absolute). The
  budget for the deterministic engine is dominated by the two Typst renders (C8/C9), so the perf sample
  watches render time most closely.

**Coverage:** 100% floor enforced by COVERAGE-REVIEWER at every gate; pragmas require a stated reason.
**Determinism note:** PDF byte-comparison must neutralise non-deterministic Typst metadata (timestamps)
— compare structural/text content or pin a fixed timestamp in the `World`, not raw bytes (DISCUSS D2).

---

## Parallel Grouping

### PRIMARY Tier (the only tier — single item)

**Round 1 (concurrent):**
- `crates/core` family C1→C2→C3→{C4→C5, C6→{C7, C8}} (internally sequential by data deps)
- `crates/jobparse` J1 (independent of all C*)
- `templates/letter/classic-letter.typ` T1 (authoring; independent until C9 renders it)

**Round 2 (after C6/C8 + T1):**
- C9 cover-letter render (needs C8 World + T1 template + C7-validated view)

**Round 3 (after all C*, J1):**
- D1 Tauri commands + SQLCipher store (wires the core↔jobparse seam)

**Round 4 (after D1):**
- D2 React/TS UI (consumes the command surface)
- CI extension (parallel with D2 — touches `.github/`, no overlap with UI files)

**Round 5 (after D2):**
- ST STORY journey + perf-delta harness

Parallel-safety holds: no two concurrent tasks write the same file, depend on each other's output, or
build a shared component the other needs mid-run. (C* and J1 live in separate crates; T1 is a new file.)

---

## CI gate sequence (reconciled with existing `.github/workflows/ci.yml`)

Existing jobs **kept green and unchanged in intent**: `pii-guard` (email-domain + private-key rules)
and `foundation` (deterministic fixtures, schema validate, CV render smoke + artifact). The Rust
workspace adds a new job; the **gate order** the cycle enforces:

```
1. cargo fmt --check
2. cargo clippy --all-targets -- -D warnings
3. test L1 unit (core)        ─┐
4. test L2 module             │  cargo test --workspace
5. test L3 boundary           │  (+ schema-conformance of the serialized tailored view,
6. test L4 system (offline)   │   reusing tools/fake-data/validate.js)
7. test L5 STORY perf-delta   ─┘  (UI/journey driver — D3)
8. typst render smoke         ─ existing `foundation` job: classic.typ CLI render stays green
9. pii-guard                  ─ existing job (email domains + no private keys)
```

Reconciliation notes:
- `foundation`'s **CV render smoke must stay green** — proves `classic.typ` is still CLI-renderable
  after the §H light adaptation (the explicit §H requirement). The embedded render (C8) is additional,
  not a replacement.
- The schema-validate step is reused to assert the **tailored view conforms to master-cv.schema.json**
  (boundary L3), not just the source fixtures.
- `pii-guard` remains the outermost guarantee: new fixtures/tests use `@example.*` only; no real career
  data; no private keys. Any new test data goes through `tools/fake-data/` (deterministic, seeded).
- Steps 3–7 run under `cargo test --workspace`; the STORY journey (7) may need a separate runner/driver
  (D3) — gate it after the cargo levels so a driver flake doesn't mask a logic regression.

---

## VALUE_HANDLER_POOL Required

- **handler-rust** — `crates/core`, `crates/jobparse`, Tauri commands, cover-letter template, CI.
- **handler-react** — `apps/desktop/src` UI.

---

## Missing Handlers (self-improvement flags)

- **F-1 / SI-1 — no `handler-tauri`.** Tauri command layer + SQLCipher mapped to **handler-rust**
  (closest viable). Adequate for slice 1 (thin binding). Propose a dedicated handler only if a later
  slice grows substantial IPC/SQLCipher/native logic. Recorded for KAIZEN; **not improvised now.**

---

## Self-Improvement Flags (KAIZEN covenant)

- **SI-1** — station-map gap (no `handler-tauri`); see Missing Handlers.
- **SI-2** — **C8 embedded Typst `World` is the highest-risk task** (font provisioning, in-memory VFS
  lifetime, thread-safety across concurrent Tauri commands). If it proves non-trivial during
  decomposition, spawn `handler-architect` for a focused ADR on the `World`/VFS design before C8
  implements. (Conditional Phase 2.5 trigger.)
- **SI-3** — the **evidence-ledger guard (§E)** is the brand wedge and a recurring reviewer surface
  across every claim-bearing node. If future items add an LLM rewrite, the guard's non-vacuous test
  discipline should harden into a dedicated SECURITY/INTEGRITY reviewer prompt — note for when item #3
  (LLM layer) is planned.
- **SI-4** — no `IDEA_COST.jsonl` yet. `lifecycle-orchestrator` should emit per-task actuals this cycle
  so the next cycle has comparables (replacing HEURISTIC estimates).

---

## DISCUSS items (genuine spec gaps — surfaced, not improvised)

- **D1 — Normalized-Job type ownership.** The dependency rule forbids `jobparse`→`core` coupling. Two
  honourable options: (a) `core` defines `NormalizedJob` and `jobparse` depends on a *tiny shared
  contract type* only; or (b) the contract is purely the **JSON shape** (each crate owns its own type,
  validated against a shared JSON Schema for the Normalized Job — symmetric with the master-CV schema).
  Recommendation: **(b)** keeps the crates fully decoupled and matches the "contract is data, not code"
  rule, at the cost of authoring a `normalized-job.schema.json`. Needs a FOUNDER/architect call.
- **D2 — PDF determinism in tests.** Typst embeds timestamps/metadata; raw-byte equality is unstable.
  Plan assumes content/structural comparison or a pinned `World` timestamp. Confirm the chosen approach
  so L1/L4 render assertions are deterministic (I5).
- **D3 — STORY driver.** Tauri desktop journey driving (import→paste→review→export) needs a driver
  choice: `tauri-driver`/WebDriver vs. Playwright vs. a headless command-level harness that exercises
  the same path the UI calls. A command-level harness is cheapest and most deterministic for the perf
  gate; full-UI driving is higher fidelity. Needs a call before ST.

---

## DISCUSS resolutions (FOUNDER, 2026-06-12 — binding; supersede the open questions above)

These three gaps were *how to build/test* (not *what* we build), and are resolved from the package's
own constraints. They are binding spec for the orchestrator.

- **R-D1 — Normalized-Job type ownership → option (b), data-not-code.** Author
  `doc/schemas/normalized-job.schema.json`, **symmetric with `master-cv.schema.json`**. `crates/jobparse`
  owns its emit type and validates its output against this schema; `crates/core` owns its input type
  validated against the same schema. The seam is the JSON shape — the crates stay fully decoupled
  (honours the binding dependency rule and reuses the existing `tools/fake-data/validate.js`-style schema
  validation). Do this **before J1 and before the boundary L3 test**.
- **R-D2 — PDF determinism → pinned timestamp + structural/ledger assertions, never raw bytes.** The
  custom Typst `World` (§H) pins a **fixed timestamp** so renders are reproducible (I5). Render tests
  assert **non-empty PDF + valid PDF structure (header/page count) + the ledger invariant** (every
  claim-bearing node traces to an evidence id, §E) — NOT byte-equality. The CI `foundation` CV render
  smoke stays green unchanged.
- **R-D3 — STORY driver → headless command-level harness through the real Tauri commands.** The STORY
  drives `import → parse → tailor → coverage → approve/reject → export` through the **actual Tauri
  command layer** (the same commands the UI invokes), fully offline — cheapest and most deterministic for
  the perf-delta gate (I5/I6). Add a **React Testing Library + user-event** component test for the
  review-UI approve/reject interaction (DESIGN/STORY for the UI seam). Full `tauri-driver`/WebDriver E2E
  is **deferred** (KAIZEN note SI-5), not required to prove slice 1.

## Resumption Instructions (cold-start — no conversation history needed)

1. **Read** `SUBJECT_MATTER_UNDERSTANDING.md` (§7 = the §A–H build contract, §6 = invariants I1–I6) and
   `doc/idea/applicant-advocate/first-slice.md`. These are the contract; honour verbatim.
2. **Branch:** `slice-1-jd-to-tailored-cv`.
3. **Build order (topological, no cycle):** Round 1 → `crates/core` (C1→C2→C3→{C4→C5, C6→{C7,C8}}) and
   `crates/jobparse` (J1) and `templates/letter/classic-letter.typ` (T1) in parallel → Round 2 C9
   (cover-letter render) → Round 3 D1 (Tauri commands + SQLCipher) → Round 4 D2 (UI) + CI extension →
   Round 5 ST (STORY perf-delta). See **Work Decomposition** + **Parallel Grouping**.
4. **Hand each task to `lifecycle-orchestrator`** to run its 0–9 loop (EARS → Gherkin → failing tests →
   implement → story) with the named handler. Builder-lead does not run the loop.
5. **Test contract:** satisfy all five levels; 100% coverage floor; every level emits a perf sample;
   STORY perf-delta gated on the < 60 s offline budget (I6). The **non-vacuous dangling-id ledger test**
   (§E) and the **tailored-view-conforms-to-schema** boundary test are mandatory.
6. **CI:** keep `pii-guard` + `foundation` green (esp. the `classic.typ` CLI render smoke — §H requires
   the template stay CLI-renderable); add the Rust gate `fmt → clippy -D warnings → L1..L5 → render
   smoke → pii-guard`.
7. **Invariants every station inherits:** master CV immutable (I1); evidence ledger blocks export on any
   dangling id (I2); no fabrication (I3); **no PII in repo — synthetic `fixtures/`+`tools/fake-data/`
   only** (I4); offline + deterministic (I5); < 60 s (I6).
8. **Resolve the three DISCUSS items (D1–D3)** before the tasks they gate: D1 before J1/D1-task, D2
   before render tests, D3 before ST.
9. **Out of scope (do NOT build):** PDF/DOCX import, LLM layer, capture extension, tracker, any
   LinkedIn/Seek automation.
