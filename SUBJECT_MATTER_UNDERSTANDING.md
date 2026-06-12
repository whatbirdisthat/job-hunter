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

---

## Item #4 — Capture extension + email ingestion

This item adds a **TypeScript** acquisition surface (the first non-Rust slice). It does NOT touch the
Rust workspace. It feeds the SAME strict seam every other ingest path feeds:
`doc/schemas/normalized-job.schema.json` (camelCase; `additionalProperties:false`; required
`title`/`company`/`requirements{mustHave,niceToHave}`; optional `location`/`responsibilities`/`keywords`).
The desktop already consumes exactly that JSON via `CoreJob::from_json` (the R-D1 seam in
`apps/desktop/src-tauri/src/lib.rs::seam`). So the extension is upstream of an existing contract — it
produces, it does not extend, the Normalized Job shape.

### Pure-core architecture (the hard rule)

Two deterministic cores, each a PURE TS module: no live DOM, no `window`/`document`, no `fetch`, no
browser globals. They are the **coordinates** — unit-testable in plain Node with zero environment.

- **`dom-extract` core** — input is an **HTML string** (not a live DOM). A thin, zero-dep, tolerant HTML
  text-extractor reduces the markup to a normalized text block, then the §F rules run over that block.
  Rationale for "HTML string in", not "DOM in": a live `Document` is not constructible in `node --test`
  without jsdom (an npm dep that cannot install on the issue-#2 runners). An HTML string is trivially a
  test fixture and the extractor is itself pure and testable. The content script is the ONLY place a live
  DOM is touched; it calls `document.documentElement.outerHTML` (or a scoped container's `outerHTML`) and
  passes the string to the core.
- **`email-extract` core** — input is a raw `.eml`/HTML string → an **array** of Normalized Job JSON
  (a job-alert email lists multiple postings). MIME/quoted-printable decode + HTML→text + §F per posting.

Both cores **port** the §F rules from `crates/jobparse/src/lib.rs` verbatim in semantics:
must cues `required|must have|essential|you will need|minimum|mandatory`; nice cues
`preferred|desirable|bonus|nice to have|advantageous|ideally`; unmarked → nice; `;`-split; trailing-period
trim; title = between `hiring a/an ` and ` at `; company = after ` at ` to next `.`. A shared
`jd-core` module holds the ported §F algorithm so both cores (and the Rust crate) stay in lockstep on one
rule set — divergence here is the one bug that would silently corrupt every downstream coverage score.

### Module / file layout

The cores are **shared** between the extension and the email path, and the email path is not
browser-bound. They live in **`packages/`** (the repo's already-declared home for "the deterministic core
engine", `packages/README.md`), NOT under `extension/`. `extension/` keeps ONLY the thin MV3 wiring.
This keeps the pure cores reusable (the desktop could call `email-extract` directly later) and keeps the
browser bundle's testable surface out of the manifest tree.

```
packages/
  capture-core/
    src/
      jd-core.ts        # ported §F: cues, splitItems, parseTitle, parseCompany, classify → NormalizedJob
      html-text.ts      # zero-dep tolerant HTML → normalized text (strip tags/scripts, decode entities)
      eml.ts            # zero-dep .eml: header split, MIME walk, quoted-printable/base64 decode → html/text
      dom-extract.ts    # htmlString → NormalizedJob   (dom-extract core; uses html-text + jd-core)
      email-extract.ts  # emlString  → NormalizedJob[] (email-extract core; uses eml + html-text + jd-core)
      normalized-job.ts # the TS type + a toJson() that emits the strict camelCase shape
      validate-job.ts   # zero-dep structural validator for normalized-job.schema.json (new; see below)
      index.ts          # public re-exports
    test/
      jd-core.test.js        # L1
      html-text.test.js      # L1
      eml.test.js            # L1
      dom-extract.test.js    # L2
      email-extract.test.js  # L2
      boundary-schema.test.js# L3 (validate-job over both cores' output)
      story.test.js          # L5 (clip→json, email→jobs; perf sample + delta gate)
    fixtures/
      linkedin-job.html  seek-job.html  hostile.html  empty.html      # synthetic, PII-free
      linkedin-alert.eml seek-alert.eml multi-posting.eml             # synthetic, PII-free
extension/
  manifest.json         # MV3, minimal permissions (activeTab + scripting; NO broad host perms)
  src/
    content.ts          # THIN: reads outerHTML, calls dom-extract core, hands off
    popup.html popup.ts # THIN: "Clip this job" button → triggers capture on the active tab
    background.ts       # THIN: service worker; download handoff (chrome.downloads) + optional localhost
  test/
    manifest.test.js    # L4 system: manifest parses, MV3 v3, minimal perms, NO broad host perms
    content-smoke.test.js # L4: bundled core over a synthetic fixture yields a valid job
  README.md             # compliance posture (user-driven only) — already stubbed, to be expanded
```

The new TS structural validator (`validate-job.ts`) is needed because `tools/fake-data/validate.js`
validates the **master-CV** schema only. It is a sibling: zero-dep, structural, exits non-zero on
violation, runnable as `node`. Recommended to ALSO expose a CLI shim
`tools/fake-data/validate-job.js` for symmetry with the existing validator (the L3 test imports the TS
function directly; the CLI shim lets CI lint stray fixtures the same way the CV path does).

### Handoff design — DECISION

**Baseline (always works): downloadable `.json`.** The content/background script serializes the
Normalized Job to a `.json` blob and triggers a browser download (`chrome.downloads.download` of an
object URL). The user then imports it through the desktop's existing import path — the JSON is exactly
what `CoreJob::from_json` already accepts (the R-D1 seam). Zero new Rust surface, zero new permission,
no listening socket. This is the **chosen primary** path because it is the simplest compliant route and
reuses an existing contract end to end.

**Optional (deferred, behind a flag): localhost handoff.** A `POST` to a loopback port the desktop app
opens. NOT enabled by default; it requires the Tauri app to run a local listener (new Rust surface, NOT
in item #4 scope) and raises a security model (origin allow-listing, CSRF/loopback-rebinding) that must
be designed before it ships. Documented as a future path, not built this item. See DISCUSS-HANDOFF.

### Compliance posture (non-negotiable)

User-driven capture ONLY. The content script runs ONLY on a tab the user has already opened, triggered by
the user's click — `activeTab` + a user gesture, never a persistent broad-host content-script registration.
- Manifest declares `"manifest_version": 3`, permissions limited to `activeTab` and `scripting`
  (and `downloads` for the baseline handoff). **NO** `"host_permissions"` wildcards, **NO**
  `content_scripts` auto-injected on `*://*.linkedin.com/*` / `*://*.seek.com.au/*` match patterns
  (auto-injection = "scraping pages the user is viewing without a gesture" — disallowed). Injection is
  programmatic via `chrome.scripting.executeScript` on the active tab in response to the popup click.
- NO automated login, NO navigation/automation, NO background scraping, NO anti-bot evasion, NO reading of
  tabs the user did not explicitly clip. These prohibitions are stated verbatim in BOTH `manifest.json`
  (as a description / comment block) and `extension/README.md`.

### Test contract mapping (five levels, perf-instrumented)

| Level | Scope | Where |
|---|---|---|
| L1 unit | pure functions: cues, splitItems, parseTitle/parseCompany, html-text, eml decode; empty/max/unicode/hostile | `packages/capture-core/test/{jd-core,html-text,eml}.test.js` |
| L2 module | each core's public surface end to end: htmlString→job, eml→jobs | `dom-extract.test.js`, `email-extract.test.js` |
| L3 boundary | output validates against `normalized-job.schema.json` via new zero-dep `validate-job` | `boundary-schema.test.js` |
| L4 system | manifest validity (MV3 v3, minimal perms, no broad host perms) + content-script smoke over a synthetic fixture | `extension/test/{manifest,content-smoke}.test.js` |
| L5 STORY | clip→json journey + email→jobs journey as behaviour; each emits a parse-time perf sample, gated vs a recorded baseline with a perf-delta budget | `packages/capture-core/test/story.test.js` + `doc/perf/capture-*-baseline.txt` |

Perf gate mirrors the Rust posture (`doc/perf/README.md`): a committed baseline file holds a single
wall-clock number; the gate enforces an absolute budget AND a `<= baseline * DELTA_FACTOR` regression arm
(independent of the budget). New baselines: `doc/perf/capture-clip-story-baseline.txt`,
`doc/perf/capture-email-story-baseline.txt`. Coverage floor: **100% of reachable** for the pure cores
(`jd-core`, `html-text`, `eml`, `dom-extract`, `email-extract`, `validate-job`).

### CI strategy (issue #2 reality)

The runners cannot reach any npm registry; `npm install` hard-times-out (that is why `ui` is
`continue-on-error`). Therefore:
- **Zero runtime npm deps. Zero dev npm deps for the gate.** Tests run on Node's built-in test runner
  `node --test` (node 24 confirmed on the runners). The cores are authored so they can be exercised by
  `node --test` directly. If the source is `.ts`, the test gate runs against a **checked-in plain-JS build
  or `.mjs` sources** so NO `tsc`/loader install is needed at CI time (DISCUSS-TSBUILD: pick "author in JS
  with JSDoc types + a non-blocking typecheck job" vs "author in TS, commit a build" — recommend the
  former so the BLOCKING gate needs no npm at all).
- A NEW **blocking** job `capture-core` runs `node --test packages/capture-core/test` +
  `node --test extension/test` + the perf gate + a coverage check using `node --test --experimental-test-coverage`
  (built-in, no npm). It needs NO `npm install`, so it can block honestly.
- Any job that DOES need `npm install` (e.g. an eslint/prettier or `tsc` job) MUST be
  `continue-on-error: true` with a comment referencing **issue #2**, exactly like `ui`.
- fmt/lint for TS: adopt a **zero-dep convention** (documented house style; `node --check` syntax pass on
  each file in the blocking job) rather than eslint/prettier. If an eslint job is added it is non-blocking.
- Do NOT disturb `pii-guard`, `foundation`, `rust-workspace` (stay green + blocking). `ui` stays as is.
  All synthetic fixtures (`.html`/`.eml`) must pass `pii-guard` — invented companies/roles only, emails (if
  any appear in an alert fixture) must use reserved example domains.
