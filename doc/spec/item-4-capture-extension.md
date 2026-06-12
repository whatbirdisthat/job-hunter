# Item #4 — Capture extension + email ingestion (build-ready spec)

**Branch:** `item-4-capture-extension` · **Surface:** TypeScript only (no Rust workspace changes).
**Seam:** every output validates against `doc/schemas/normalized-job.schema.json` (the R-D1 contract the
desktop already consumes via `CoreJob::from_json`). **Items 1–3 merged.**

Two user-driven, compliant acquisition paths feeding the SAME strict Normalized Job JSON:
1. MV3 "clip this job" extension — content script extracts a job posting from the DOM of a LinkedIn/Seek
   page the user is already viewing, normalizes via the pure core, hands a `.json` to the desktop app.
2. Email saved-search ingestion — a deterministic parser turning a job-alert `.eml`/HTML into an array of
   Normalized Job JSON records.

All deterministic logic lives in PURE TS cores (no live DOM, no network, no browser globals). §F rules are
**ported verbatim in semantics** from `crates/jobparse/src/lib.rs`.

---

## 1. Requirements (EARS-ready)

### Extension / DOM clip (R-EXT-*)

- **R-EXT-1** WHEN the user clicks "Clip this job" in the extension popup on the active tab, the extension
  SHALL inject the capture script into THAT tab only (programmatic `chrome.scripting.executeScript`,
  user-gesture gated), and SHALL NOT inject into any other tab.
- **R-EXT-2** The `dom-extract` core SHALL accept an HTML string and SHALL return one Normalized Job that
  validates against `normalized-job.schema.json`.
- **R-EXT-3** The `dom-extract` core SHALL classify requirements using the §F must/nice cues, defaulting
  unmarked requirements to nice-to-have, identically to `aa-jobparse`.
- **R-EXT-4** The `dom-extract` core SHALL derive `title` (between `hiring a/an ` and ` at `) and `company`
  (after ` at ` to the next `.`) per §F; absent markers SHALL yield empty strings, never an error.
- **R-EXT-5** WHEN the input HTML contains no recognizable job content, the core SHALL return a job with
  empty `requirements.mustHave`/`requirements.niceToHave` and SHALL NOT throw.
- **R-EXT-6** The extension SHALL hand the Normalized Job to the desktop app as a downloadable `.json`
  (the baseline handoff), importable via the desktop's existing import path. (R-EXT-6b, optional/deferred:
  localhost POST handoff — NOT built this item.)
- **R-EXT-7** The MV3 manifest SHALL declare `manifest_version: 3`, request only `activeTab`, `scripting`,
  and `downloads`, and SHALL declare NO `host_permissions` and NO auto-injected `content_scripts`.
- **R-EXT-8** The manifest description and `extension/README.md` SHALL state the compliance posture:
  user-driven only; no automated login, navigation, background scraping, or anti-bot evasion.
- **R-EXT-9** The content/popup/background wiring SHALL contain no deterministic extraction logic — it
  SHALL only read `outerHTML`, call the pure core, and perform the handoff.

### Email ingestion (R-EML-*)

- **R-EML-1** The `email-extract` core SHALL accept a raw `.eml`/HTML string and SHALL return an ARRAY of
  Normalized Jobs, each validating against `normalized-job.schema.json`.
- **R-EML-2** The core SHALL decode MIME structure (multipart walk, quoted-printable and base64 transfer
  encodings) and SHALL select the HTML (or text) body before extraction.
- **R-EML-3** WHEN a job-alert email lists multiple postings, the core SHALL emit one Normalized Job per
  posting; WHEN it lists none, it SHALL return an empty array (never throw).
- **R-EML-4** The core SHALL apply the same §F cue/splitter/title/company rules as `dom-extract`
  (shared `jd-core`), so the two paths cannot diverge.
- **R-EML-5** The core SHALL be pure (string in, value out) — no file I/O, no network, no browser globals.

### Shared / boundary (R-JOB-*)

- **R-JOB-1** A new zero-dep structural validator `validate-job` SHALL enforce `normalized-job.schema.json`
  (camelCase; `additionalProperties:false`; required `title`/`company`/`requirements{mustHave,niceToHave}`;
  optional `location`/`responsibilities`/`keywords`) and SHALL exit non-zero on any violation.
- **R-JOB-2** Both cores' serialized output SHALL pass `validate-job` (L3 boundary).
- **R-JOB-3** The §F algorithm SHALL exist in exactly ONE module (`jd-core`) imported by both cores.

---

## 2. File layout

```
packages/capture-core/
  src/{jd-core, html-text, eml, dom-extract, email-extract, normalized-job, validate-job, index}.ts
  test/{jd-core, html-text, eml}.test.js          # L1
  test/{dom-extract, email-extract}.test.js       # L2
  test/boundary-schema.test.js                    # L3
  test/story.test.js                              # L5 (+ perf samples)
  fixtures/{linkedin-job,seek-job,hostile,empty}.html
  fixtures/{linkedin-alert,seek-alert,multi-posting}.eml
extension/
  manifest.json
  src/{content, popup.ts, popup.html, background}.ts
  test/{manifest, content-smoke}.test.js          # L4
  README.md
tools/fake-data/validate-job.js                   # CLI shim around validate-job (symmetry w/ validate.js)
doc/perf/capture-clip-story-baseline.txt
doc/perf/capture-email-story-baseline.txt
```

See SUBJECT_MATTER_UNDERSTANDING.md "Item #4" for the rationale (cores in `packages/`, thin wiring in
`extension/`, TS-build decision deferred to DISCUSS-TSBUILD).

---

## 3. Pure-core function signatures (TS)

```ts
// normalized-job.ts — the strict seam shape (camelCase, mirrors aa-jobparse output)
export interface Requirements { mustHave: string[]; niceToHave: string[]; }
export interface NormalizedJob {
  title: string;
  company: string;
  location?: string;
  responsibilities?: string[];
  requirements: Requirements;
  keywords?: string[];
}
export function toJson(job: NormalizedJob): string; // strict shape; omits empty optionals

// jd-core.ts — the ONE ported §F implementation (no DOM, no I/O)
export const MUST_CUES: readonly string[]; // required|must have|essential|you will need|minimum|mandatory
export const NICE_CUES: readonly string[]; // preferred|desirable|bonus|nice to have|advantageous|ideally
export function splitItems(body: string): string[];          // ';'-split, trim, trailing-period trim, drop empty
export function parseTitle(raw: string): string;             // between "hiring a/an " and " at "
export function parseCompany(raw: string): string;           // after " at " to next "."
export function parseJd(raw: string): NormalizedJob;         // full §F over a normalized text block

// html-text.ts — zero-dep tolerant HTML → normalized text block
export function htmlToText(html: string): string;            // strip script/style, tags→space, decode entities, collapse ws

// eml.ts — zero-dep .eml parsing
export interface EmlBody { contentType: string; text: string; } // decoded body part
export function parseEml(raw: string): EmlBody[];            // header split, MIME walk, QP/base64 decode
export function selectBody(parts: EmlBody[]): string;        // prefer text/html, fallback text/plain

// dom-extract.ts — the dom-extract CORE
export function extractFromHtml(html: string): NormalizedJob; // htmlToText → parseJd

// email-extract.ts — the email-extract CORE
export function extractFromEml(raw: string): NormalizedJob[]; // parseEml → selectBody → split postings → parseJd each

// validate-job.ts — zero-dep structural validator
export interface JobValidationError { path: string; message: string; }
export function validateJob(value: unknown): JobValidationError[]; // [] === valid
```

Porting note: `parseJd` MUST reproduce `aa-jobparse::parse` cue-walking semantics (earliest cue wins;
body terminates at the next cue; trim to the last `.` before the next cue; strip a leading `:` after a
cue). The Rust tests in `crates/jobparse/src/lib.rs` are the oracle — mirror each as a TS L1 case.

---

## 4. Handoff contract (DECISION: download baseline)

- The background/content script builds `toJson(job)`, creates an object URL, and triggers
  `chrome.downloads.download({ url, filename: "clipped-job-<ts>.json", saveAs: true })`.
- The user imports the `.json` through the desktop's existing import path. The JSON is byte-compatible
  with `CoreJob::from_json` (no Rust change). This is the chosen primary path: simplest, compliant,
  reuses an existing contract, no listening socket, no extra permission beyond `downloads`.
- **Deferred** (R-EXT-6b): optional localhost `POST` to a loopback port the desktop opens. Requires new
  Rust surface + a documented security model (origin allow-list, loopback-rebinding/CSRF defense). NOT in
  item #4. See DISCUSS-HANDOFF.

---

## 5. Manifest permission set (minimal)

```jsonc
{
  "manifest_version": 3,
  "name": "Job Hunter — Clip this job",
  "version": "0.1.0",
  "description": "User-driven only. Clips the job you are viewing on a click. No automated login, no background scraping, no anti-bot evasion.",
  "permissions": ["activeTab", "scripting", "downloads"],
  // NO "host_permissions", NO auto-injected "content_scripts".
  "action": { "default_popup": "src/popup.html" },
  "background": { "service_worker": "src/background.js" }
}
```

The L4 manifest test asserts: parses as JSON; `manifest_version === 3`; `permissions` is a subset of the
allow-list above; `host_permissions` absent; `content_scripts` absent.

---

## 6. Test levels with representative cases

**L1 unit** (`jd-core`, `html-text`, `eml`)
- each MUST cue → mustHave; each NICE cue → niceToHave; unmarked → niceToHave.
- `splitItems("A; B; C.")` → `["A","B","C"]`; trailing-period trim; empty segments dropped.
- `parseTitle`/`parseCompany`: "We are hiring a X at Y. …" → `X`/`Y`; `hiring an` branch; marker without
  ` at ` → empty.
- empty string → empty buckets; unicode ("café ☕; 日本語") → no panic, counts preserved.
- hostile: unclosed tags, `<script>` with cue text inside (must be stripped before §F), 1 MB input bound.
- `eml`: quoted-printable `=3D`→`=` and soft line breaks `=\r\n`; base64 body; multipart boundary walk.

**L2 module**
- `extractFromHtml(linkedin-job.html)` → valid job with expected title/company/must/nice.
- `extractFromHtml(empty.html)` / `(hostile.html)` → valid job, empty buckets, no throw.
- `extractFromEml(multi-posting.eml)` → N jobs, each valid; `(linkedin-alert.eml)` → ≥1 job.

**L3 boundary**
- for every fixture, `validateJob(JSON.parse(toJson(out)))` returns `[]`.
- negative self-test: a hand-broken object (extra key / missing `requirements`) yields a non-empty error
  list (proves the validator is non-vacuous).

**L4 system**
- `manifest.json` parses, MV3 v3, permissions ⊆ allow-list, no host perms, no content_scripts.
- content-script smoke: import the BUNDLED core, run it over `linkedin-job.html`, assert a valid job
  (proves the wiring calls the core correctly without a browser).

**L5 STORY** (perf-instrumented, gated)
- clip journey: htmlString fixture → `extractFromHtml` → `toJson` → `validateJob` == valid; record
  parse-time sample; assert `elapsed < BUDGET` AND `elapsed <= baseline * DELTA_FACTOR`
  (`doc/perf/capture-clip-story-baseline.txt`).
- email journey: `.eml` fixture → `extractFromEml` → each `validateJob` valid; same perf gate against
  `doc/perf/capture-email-story-baseline.txt`. Baselines hold a single wall-clock number, committed,
  not self-ratcheted (mirror `doc/perf/README.md`).

**Coverage:** 100% of reachable for the pure cores. Run via `node --test --experimental-test-coverage`.

---

## 7. CI plan

- New **blocking** job `capture-core`: `node --test packages/capture-core/test` + `node --test
  extension/test` + perf gate + `--experimental-test-coverage` floor. **No `npm install`** — runs on
  node 24 built-ins only, so it blocks honestly on the issue-#2 runners.
- Any job needing `npm install` (eslint/prettier/`tsc`) MUST be `continue-on-error: true` with an
  issue-#2 comment, like `ui`.
- fmt/lint: zero-dep house convention + `node --check` syntax pass in the blocking job. No eslint in the
  blocking path.
- `pii-guard`, `foundation`, `rust-workspace` stay green + blocking, untouched; `ui` unchanged. All
  `.html`/`.eml` fixtures are synthetic, PII-free, and any email tokens use reserved example domains so
  `pii-guard` stays green.

---

## 8. DISCUSS — spec gaps (do not improvise)

- **DISCUSS-DOM** We have NO real LinkedIn/Seek DOM samples — only synthetic fixtures we author. The
  `dom-extract` core is therefore specified against the §F text shape (`htmlToText` → cue parsing), NOT
  against site-specific selectors. Real pages embed the JD in nested containers, JSON-LD, or lazy-loaded
  nodes; a pure text reduction may capture nav/footer chrome as noise. Acceptance bar = synthetic
  fixtures (mirrors R6's posture for JD parsing). Need confirmation that selector-level scraping is
  explicitly OUT of scope this item, and whether a future item adds a per-site content selector.
- **DISCUSS-HANDOFF** The localhost handoff (R-EXT-6b) needs a security model before it can ship: which
  origins may POST, loopback-rebinding/DNS-rebinding defense, and that opening a listener doesn't violate
  the "no background server" posture. Recommend deferring to a dedicated item; confirm the download
  baseline is acceptable as the only shipped path for #4.
- **DISCUSS-TSBUILD** The blocking gate must need NO npm. Two options: (a) author the cores in plain
  `.mjs`/JS with JSDoc types (typecheck via a non-blocking `tsc` job), or (b) author in `.ts` and commit
  a built JS artifact for `node --test` to consume. Recommend (a) — zero build step in the blocking path.
  Confirm the choice; it shapes file extensions throughout §2/§3.
- **DISCUSS-EML-SCOPE** How rich must the `.eml` decoder be? Recommend supporting the common LinkedIn/Seek
  alert shape only: `multipart/alternative`, quoted-printable + base64, UTF-8. Out of scope: nested
  `multipart/related`, non-UTF-8 charsets, S/MIME, inline images. Confirm this bound.
- **DISCUSS-MULTI-POSTING** Email-alert "split into postings" needs a deterministic delimiter. Recommend
  splitting on the synthetic §F sentence pattern ("We are hiring a … at ….") so one alert → N jobs
  deterministically. Real alerts use per-card markup; with synthetic-only fixtures we cannot validate
  against real card boundaries. Confirm the synthetic split rule is the acceptance bar.
- **DISCUSS-RICHFIELDS** The strict `normalized-job.schema.json` carries only `title`/`company`/
  `location`/`responsibilities`/`requirements`/`keywords`. The richer `fixtures/jobs/*.json` (with
  `source`, `salary`, `workModel`, …) is a SEPARATE, non-strict shape and is NOT the seam. Confirm the
  extension targets the strict schema only (it does) and that source/url provenance is NOT required on
  the clip output for #4.
