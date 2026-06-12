# Subject Matter Understanding — Applicant Advocate (job-hunter)

> Status: expanded from `doc/idea/applicant-advocate/smu-seed.md` by FOUNDRY builder-lead, 2026-06-12.
> Scope of authority: this SMU is **append-only** — add sections; never rewrite or delete existing
> content. Where `doc/ARCHITECTURE.md` and the IDEA package disagree on **slice scope**, the IDEA
> package supersedes (handoff.md): slice 1 is **JSON import only** (PDF/DOCX is later item R3).

---

## 1. What the product is

A **local-first, privacy-absolute desktop app** (Tauri: Rust core + React/TS UI) that turns one
canonical **master CV** into an **honest, tailored CV + draft cover letter** for a specific job —
every rendered claim click-traceable to evidence the user actually wrote. Deterministic; no LLM in
this slice. Output is two typeset PDFs, produced fully offline in **< 60 s**.

## 2. Who it's for

AU mid-career tech/knowledge workers (developer, PM, designer, consultant) job-hunting on Seek +
LinkedIn AU, comfortable running a desktop app. Later: career-changers, high-volume applicants.

## 3. Problem framing

Per-application tailoring is slow and most people do it badly or generically. Cloud tools exfiltrate
sensitive career data and optimise for ATS keyword-gaming rather than honest fit. The gap:
**honest, local, polished** tailoring. The wedge is the **evidence ledger** — proof, not gaming.

## 4. Core domain concepts / terms

- **Master CV** — the immutable, canonical source-of-truth (`doc/schemas/master-cv.schema.json`).
  Imported as JSON; validated on load; never mutated by tailoring.
- **Evidence** — an id'd skill / experience / achievement bullet. The atom of traceability. Achievement
  ids look like `exp_1_0_b0`; experience ids like `exp_1_0`; the summary carries `summary:<index>`.
- **Normalized Job** — a parsed JD: title, company, location, responsibilities, requirements
  (must-have / nice-to-have), keywords. Produced deterministically by `crates/jobparse` from pasted text.
- **Requirement** — a single job need classified **must-have** vs **nice-to-have**.
- **Match (matching primitive)** — see §A below. The unit operation under coverage and fit.
- **Coverage** — per-requirement covered/uncovered + matching evidence ids; the **testable** metric.
- **Fit score** — a single 0–1 number derived from coverage (§C). **Not** part of acceptance.
- **Evidence Ledger / Map** — the mapping from each output line → master-CV evidence id + the JD
  requirement it answers. The integrity backbone *and* the brand. Visible feature, not just a guard.
- **Tailored View** — a job-specific **selection/ordering over the master CV**. It is a *view*, never
  a mutation, and (critically, §H) **conforms to `master-cv.schema.json`** so `classic.typ` renders it
  unchanged. Never a bespoke object.
- **Applicant Advocate** — the *optional, later* LLM layer (Ollama / BYO-key). **Not in slice 1.**

## 5. Design values (tie-breakers, in priority order)

1. **Privacy > convenience** — on-device always; no PII off the machine or in the repo.
2. **Honesty/traceability > polish** — never fabricate; every claim proven (evidence ledger).
3. **Deterministic > LLM** — the deterministic engine *is* the product; LLM is an optional accelerant.
4. **Local > cloud** — no backend that receives personal data.

When two implementation choices are otherwise equal, decide in this order. (E.g. an export that is
prettier but contains one unproven claim is **rejected** — honesty outranks polish.)

## 6. Invariants (non-negotiable — every station inherits these)

- **I1 — Master CV immutable.** Tailoring produces views; the loaded master CV is read-only.
- **I2 — Evidence ledger.** *Every claim-bearing output node carries a `sourceEvidenceId` resolvable
  in the loaded master CV.* On any dangling/unresolvable id → **export is blocked (hard fail)** and the
  node is named. Scaffold nodes (greeting, why-company/role) are marked `scaffold` and exempt (§E).
- **I3 — No fabrication / no keyword-stuffing.** Engine selects and reorders verbatim; never invents
  text, metrics, or claims.
- **I4 — No PII in the repo, ever.** Tests/CI use ONLY `fixtures/` + `tools/fake-data/`. The schema's
  `person` and `experience.contact` blocks are empty in committed fixtures; emails use reserved
  `@example.*` domains (enforced by the `pii-guard` CI job).
- **I5 — Offline + deterministic.** No network in the tailoring path. Same inputs → byte-stable
  outputs (modulo non-deterministic PDF metadata, which tests must neutralise — see plan).
- **I6 — < 60 s budget.** JD-text → two PDFs, offline, on a typical laptop. The STORY test gates a
  perf-delta against this baseline.

## 7. The pinned algorithms §A–H — the build contract (encode verbatim, no black boxes)

These are copied from `doc/idea/applicant-advocate/first-slice.md` and are the contract. Downstream
EARS / tests / implementation must encode them exactly.

### §A — Skill/keyword matching (the matching primitive)
A requirement is **matched** iff any of its normalized tokens equals (case-insensitive) a token or
alias of any of: any **skill** (all four categories: `programmingLanguages`, `skills`,
`toolsTechnologies`, `asAServices`), any **experience `tag`**, or any token of an **achievement
`description`/`tags`**. Normalization = lowercase + a small **alias map** seeded in `crates/core`:
`js→javascript`, `ts→typescript`, `k8s→kubernetes`, `ci/cd→continuous integration`, … (extensible).
Aliases declared on a `skill.aliases[]` participate in normalization too.

### §B — Coverage (drives the testable metric)
- `must_have_coverage = matched_must / total_must`
- `nice_have_coverage = matched_nice / total_nice`
- The **coverage report** lists each must/nice requirement with `covered` (bool) + the **matching
  evidence ids**. This enumeration is the testable success criterion (not the fit score).
- Edge: division by zero when a bucket is empty — define `coverage = 1.0` (or documented sentinel) for
  an empty bucket; must be an explicit, tested decision, not an unhandled panic.

### §C — Fit score (slice-1 minimal form)
`fit_score = 0.6 * must_have_coverage + 0.4 * nice_have_coverage`  (range 0–1).
Title/domain/seniority/recency alignment and penalties from the brief's full formula are **DEFERRED**
(a later "scoring v2" item — the schema has no seniority field and those terms need a taxonomy not in
slice 1). Fit score is **not** part of acceptance; coverage is.

### §D — Bullet selection / ordering (deterministic)
Rank achievements by, in order:
1. matches a must-have requirement, then
2. has `metrics` (non-empty), then
3. higher `evidenceStrength`, then
4. recency (parsed `startDate` of the owning experience — later is better), then
5. `emphasise` flag.
Select **top-N per role** to fit one–two pages; **never invent**. Ranking must be a **total order**
(stable, deterministic tie-break — e.g. final tie-break on achievement `id`) so output is reproducible.
**Tailored summary** = the `summaryVariants` entry with the most requirement-token overlap, taken
**verbatim**, carrying provenance `summary:<index>`. No free-text generation in slice 1.

### §E — Evidence-ledger guard (the headline wedge — exact contract)
Every **claim-bearing output node** carries a `sourceEvidenceId`:
- CV bullet → the achievement `id` it was copied from (verbatim).
- CV summary → `summary:<index>`.
- Cover-letter **strength** paragraph → the achievement `id` it wraps.
- Cover-letter **boilerplate** (greeting, why-company/role from job fields) → marked `scaffold`,
  exempt (it asserts no experience claim).

The guard asserts **every claim-bearing node has a `sourceEvidenceId` resolvable in the loaded master
CV**; on failure it **blocks export (hard fail)** and **names the node**. In slice 1 (verbatim
select/reorder) this invariant should always hold; it becomes load-bearing when the LLM layer arrives,
but ships day 1. **Non-vacuous test (mandatory):** a fixture injects a tailored node with a dangling
`sourceEvidenceId` (and a summary variant whose text matches no master-CV achievement) and asserts the
guard **blocks** it. (Non-vacuous = the test would fail if the guard were a no-op.)

### §F — JD parse: must/nice classification (pinned cues + accepted risk R6)
Split requirements by cue phrases:
- **must** = `required | must have | essential | you will need | minimum | mandatory`
- **nice** = `preferred | desirable | bonus | nice to have | advantageous | ideally`
- An **unmarked** requirement defaults to **nice-to-have**.

On the synthetic JD fixtures (authored with clear cues — `descriptionRaw` carries `Required: …` /
`Nice to have: …`) classification is 100%. The structured `requirements.mustHave[]` / `niceToHave[]`
arrays in the job fixtures are the parser's **expected-output oracle** for tests. Robustness on
free-form real-world JDs is **accepted risk R6** — not part of the slice-1 bar.

### §G — Cover-letter output (format + template)
A **second Typst PDF**, `templates/letter/classic-letter.typ` (**to be authored**, matching the CV
look from `doc/design/pdf-look.md`), plus editable text shown in the UI before render. Structure:
greeting → why-this-role/company (templated from job title + company = **scaffold**) → **2–3 strength
paragraphs**, each wrapping one selected achievement (carrying its evidence id) → close.
**Export produces two PDFs: `cv.pdf` + `cover-letter.pdf`.**

### §H — Typst embedding contract (Rust, no shell-out)
Use the `typst` crate (or `typst-as-lib`) with a custom `World` that provides **bundled fonts**
(ship Liberation Sans / Liberation Mono with the app — no system dependency) and an **in-memory
virtual filesystem**. The tailored-view JSON is exposed at a fixed virtual path **`/view.json`** and
`sys.inputs.data` is set to it, so the **same `classic.typ` renders both via CLI and embedded** (the
existing data-loader resolves a root-relative path). `classic.typ` may be **lightly adapted** but
**must stay CLI-renderable** so the fixture render test keeps working. **The tailored-view JSON
conforms to `master-cv.schema.json`** — a filtered/reordered master CV (a view, never a bespoke
object) — so `classic.typ` (which consumes a full master-CV document) renders it unchanged.

## 8. Stack & station/handler map

| Concern | Technology | FOUNDRY handler |
|---|---|---|
| Rust core (`crates/core`) — types, normalize, match, coverage, fit, ranking, view assembly, ledger guard, embedded Typst render | Rust 1.96, `serde`/`serde_json`, `thiserror`, `typst` (or `typst-as-lib`), bundled Liberation fonts | **handler-rust** |
| JD parser (`crates/jobparse`) — pasted text → Normalized Job JSON | Rust 1.96 | **handler-rust** |
| Tauri command layer (`apps/desktop/src-tauri`) — binds commands to core | Rust 1.96, Tauri 2.x, SQLite + SQLCipher | **handler-rust** |
| React/TS UI (`apps/desktop/src`) — onboarding, JD paste, coverage/review, preview, export | React, TypeScript 5.x, Vite | **handler-react** |
| Typst CV + cover-letter templates | Typst 0.14.2 | rendered/validated by **handler-rust** (render path) / CI smoke |
| Fixtures & schema validation tooling (existing) | Node 24, `jq` | existing — reused, not re-authored |

**FOUNDER finding (station-map gap):** there is **no dedicated Tauri / SQLCipher value-handler** in
the roster. The agreed mapping is: **Rust core + Tauri command layer → handler-rust**, **React/TS UI
→ handler-react**. This is the closest viable mapping and is adequate for slice 1 (the Tauri surface
is a thin command-binding layer over `crates/core`; no Tauri-specific domain logic). If a later slice
grows substantial Tauri/IPC/SQLCipher-specific logic, a dedicated `handler-tauri` should be proposed
under the KAIZEN covenant. Recorded as a FOUNDER finding, not improvised.

## 9. Toolchain parity (verified 2026-06-12)

cargo 1.96.0 · rustc 1.96.0 · node v24.16.0 · npm 11.13.0 · typst 0.14.2 · jq-1.7 · git. All present.

## 10. Existing assets to REUSE (do not re-author)

- `doc/schemas/master-cv.schema.json` — the data contract for both master CV and tailored view.
- `templates/cv/classic.typ` — the CV template (light adaptation only; must stay CLI-renderable).
- `doc/design/pdf-look.md` — design tokens; the cover-letter template must match this look.
- `fixtures/personas/*.cv.json` (4) + `fixtures/jobs/*.json` (6) + `fixtures/manifest.json` — the test
  oracle. Jobs carry both `descriptionRaw` (parser input) and structured `requirements.*` (oracle).
- `tools/fake-data/{generate,validate}.js` — deterministic fixture generation + schema validation.
- `.github/workflows/ci.yml` — `pii-guard` + `foundation` jobs; extend, keep green.

## 11. Accepted risks carried into slice 1

R2 (deterministic-only cover letters may read flat — accepted), R3 (PDF/DOCX import deferred),
R4 (Tauri/SQLCipher setup friction — accepted for technical primary actor), R6 (JD-parse robustness on
free-form JDs — accepted; slice-1 bar is the synthetic fixtures only). R5 (license) RESOLVED: MIT.

---

## 12. Item #2 — PDF/DOCX résumé import (R3, deferred from slice 1)

> Append-only addition by FOUNDRY builder-lead, 2026-06-13, for ROADMAP item #2. The parsing-strategy
> spike (`doc/idea/applicant-advocate/spike-resume-import.md`) is **COMPLETE** and its library/architecture
> choices are FINAL — see §12.3. Deterministic; **NO LLM** (the LLM layer is item #3).

### 12.1 What item #2 adds

A second onboarding path: instead of importing a master CV as JSON (slice 1), the user imports an existing
**PDF or DOCX résumé**, which is parsed **deterministically** into a **new** master-CV document
(`doc/schemas/master-cv.schema.json`) that the user reviews before installing. Honours I1 (immutable): the
import produces a *new* document for review; it never silently mutates a loaded master CV. The product value
is **honest, deterministic structure** — no invented fields, no LLM guessing.

### 12.2 New domain terms (item #2)

- **Résumé import** — the act of turning an existing PDF/DOCX résumé file into a master-CV document.
- **Extraction** — getting raw text out of a file: PDF → flat text stream (`pdf-extract`); DOCX →
  per-paragraph text (`zip` + `quick-xml` over `word/document.xml`). DOCX is higher fidelity (preserves
  paragraph boundaries); PDF text is a flat stream with **no reliable newlines** (spike finding).
- **Segment** — a labelled region of the extracted text recognised by heuristic cue tokens
  (header block, a "Skills/Technologies" segment, experience blocks `title @ company · dates`).
- **Segmenter / mapper** — the hand-written deterministic component that turns segments into master-CV
  nodes (`person`/`headline`, skill lists, `experience[]` + `achievementsTasks[]`).
- **Synthetic id** — a stable, deterministic id assigned to every produced experience/achievement node
  (`imp_exp_0`, `imp_exp_0_b1`, …) since an imported résumé carries no evidence ids. These mirror the
  evidence-ledger id shape (§6 I2) so downstream tailoring resolves them unchanged.
- **`ImportError`** — the typed error surfaced on unsupported `kind`, undecodable/garbage bytes, or
  empty/structureless extraction; surfaced across the Tauri boundary without panicking (parse-don't-
  validate, I5).

### 12.3 Station / handler map — item #2 addition (extends §8)

| Concern | Technology | FOUNDRY handler |
|---|---|---|
| Résumé import (`crates/cvimport`) — PDF/DOCX text extraction + deterministic segment→master-CV mapping | Rust 1.96, `pdf-extract 0.10`, `zip`, `quick-xml`; **`docx-rs 0.4` dev-dependency only** (synthetic DOCX fixtures); depends on **`crates/core` only** | **handler-rust** |
| `import_resume(bytes, kind)` Tauri command (`apps/desktop/src-tauri`) — returns parsed MasterCv JSON for review; reuses existing `import_master_cv` validation to install | Rust 1.96, Tauri 2.x | **handler-rust** |
| Onboarding "import résumé" option (`apps/desktop/src`) — second import option **alongside** the existing JSON import | React, TypeScript 5.x | **handler-react** |

**One-way crate graph (preserved):** `crates/cvimport` → `crates/core` only. It MUST NOT depend on
`crates/jobparse`, `aa-desktop`, or the render path. Added to `[workspace].members`. `aa-desktop` gains a
dependency on `cvimport` (it already depends on `core` + `jobparse`; it is the only crate wiring seams).

### 12.4 Invariant inheritance (item #2)

- **I1 (immutable):** import yields a NEW master-CV document for review; never mutates a loaded one.
- **I4 (no PII):** **no committed binary fixtures.** Tests render a *persona* (`fixtures/personas/*.cv.json`)
  to PDF via `templates/cv/classic.typ` and synthesise a DOCX from the same persona at test time. pii-guard
  stays green.
- **I5 (deterministic):** same input bytes → same master-CV output. No network, no LLM.

### 12.5 Accepted risk (carried, slice-scoped)

**R3a:** real-world résumé layouts (multi-column, tables, graphics) defeat flat-text heuristics. The
acceptance bar for item #2 is the **synthetic personas rendered to PDF / synthesised to DOCX** (mirrors R6's
posture for JD parsing). Robustness on arbitrary real résumés is out of scope and is the natural home for
the later evidence-bounded LLM layer (item #3). **R3b:** `pdf-extract` line-joins adjacent layout lines —
mitigated by segmenting on cue tokens, not newlines, and proven by the round-trip STORY.
