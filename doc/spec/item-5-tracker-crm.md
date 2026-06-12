# Item #5 — Application tracker / CRM (on-device) — build-ready spec

**Branch:** `item-5-tracker-crm` (off `main`; items 1–4 merged) · **Surface:** new Rust crate
`crates/tracker` (aa-tracker) + thin command/persistence wiring in `apps/desktop/src-tauri` +
a React tracker/call-sheet view in `apps/desktop`. **All deterministic + on-device. NO LLM
required** (item #3's `aa-advocate` stays optional/flagged and is NOT a dependency here).

> The phase-2 **workflow layer** (ARCHITECTURE.md layer 7 "Workflow"; brief lines 547–630).
> Four capabilities, all extracted as PURE, clock-injected Rust cores so each becomes a tested
> coordinate; persistence + Tauri commands + UI are wired THINLY on top.
>
> The `.feature`-style scenarios below are **specification only** and live under `doc/spec/`
> — each maps to an executable Rust/TS test (L1–L5) carrying the same `R-*` id in a comment.
> The traceability table at the foot names the proving test for every requirement.
>
> **Storage:** see `doc/design/item-5-storage-decision.md` — local-file JSON store behind a
> `TrackerStore` seam this slice; SQLCipher encryption-at-rest deferred to a dedicated storage
> slice (DISCUSS-STORAGE) behind the SAME seam.

---

## 0. Architecture: new crate `aa-tracker` + one-way dep proof

**Decision: a NEW workspace crate `crates/tracker` (`aa-tracker`), depending on `aa-core` ONLY.**

Justification (the one-way graph holds):

- The tracker needs to *reference* a Normalized Job and the evidence/document ids produced by
  earlier slices. `aa-core` already exposes `NormalizedJob` and the `MasterCv`/`Achievement`
  evidence-id model. The tracker references jobs + evidence ids **by value/id**, not by reaching
  into `aa-jobparse`/`aa-cvimport`/`aa-advocate`. So `aa-tracker → aa-core` is the only edge —
  identical to the `cvimport`/`advocate` pattern (each depends on `aa-core` ONLY; the command
  crate `aa-desktop` is the sole multi-domain crate).
- The pure cores (lifecycle SM, scheduler, call-sheet builder, CRM model) are **clock-injected
  pure functions** — they take `today: Date` as a parameter and perform NO IO, so they have NO
  storage dependency and cannot pull a wider graph.
- The persistence adapter (`TrackerStore` impl) + the new Tauri commands live in/under
  `apps/desktop/src-tauri` (the command layer), NOT in `aa-tracker`. The crate stays IO-free.

**Cargo wiring** (mirrors `crates/advocate`): add `crates/tracker` to `[workspace] members`;
`aa-tracker` deps = `aa-core { path = "../core" }` + `serde`/`serde_json`/`thiserror` (workspace).
**No** `sqlite`/`rusqlite`/`sqlcipher` dependency enters the workspace (storage decision).
`apps/desktop/src-tauri/Cargo.toml` gains `aa-tracker = { path = "../../../crates/tracker" }`.

---

## 1. Pure cores (clock-injected; the tested coordinates)

All four cores live in `aa-tracker`, are pure (value in, value out), and NEVER read the wall clock
— `today` is always a parameter. A small `Date` value type (`{ year, month, day }`, `Ord`,
serde) carries dates deterministically; day arithmetic is calendar-day counting (no timezones).

### 1.1 Lifecycle state machine (`lifecycle.rs`) — R-TRK-*

States: `Discovered → Tailored → Applied → FollowUpDue → Interview → Closed`. Transitions are
EXPLICIT and TESTED; an illegal transition is a typed error, never a silent no-op.

```rust
pub enum AppState { Discovered, Tailored, Applied, FollowUpDue, Interview, Closed }

pub enum TransitionError { Illegal { from: AppState, to: AppState } }

/// Pure, total: legal → Ok(to); illegal → typed Err. NO clock, NO IO.
pub fn transition(from: AppState, to: AppState) -> Result<AppState, TransitionError>;

/// The legal-transition table as data (the single source of truth the test enumerates).
pub fn legal_transitions() -> &'static [(AppState, AppState)];
```

Legal edges (proposed; confirm in EARS authoring): `Discovered→Tailored`, `Tailored→Applied`,
`Applied→FollowUpDue`, `FollowUpDue→Interview`, `FollowUpDue→Closed`, `Interview→Closed`,
`Applied→Closed` (withdrawn/rejected without follow-up). `Closed` is terminal (no outgoing edge).
Every NON-listed ordered pair → `TransitionError::Illegal`. The L1 test enumerates the full
`states × states` matrix and asserts each cell is exactly legal-or-error (non-vacuous: the table
is proven complete).

### 1.2 Follow-up scheduler (`scheduler.rs`) — R-SCH-*

Pure, date-driven rules engine. Given a submission date + current state + `today`, compute the
follow-up window and aging action per the brief: **day 0 submitted; day 3–5 first follow-up;
day 7–10 second follow-up; archive/deprioritise** beyond. **Clock injected** — `today` is a
parameter; the core NEVER reads the wall clock.

```rust
pub struct FollowUpWindow { pub opens_day: u32, pub closes_day: u32 } // inclusive day offsets

pub enum AgingAction {
    None,            // before the first window opens (days 0–2)
    FirstFollowUp,   // days 3–5 inclusive
    SecondFollowUp,  // days 7–10 inclusive
    Archive,         // beyond day 10 (deprioritise)
}

/// Whole calendar days from submitted → today (>= 0; future `today` clamps to 0 or errors — pin in EARS).
pub fn days_since(submitted: Date, today: Date) -> i64;

/// Pure: the aging action for an application given its submission date and today. NO wall clock.
pub fn aging_action(submitted: Date, today: Date) -> AgingAction;

/// The recommended follow-up window for an application (deterministic constant windows).
pub fn follow_up_window(action: &AgingAction) -> Option<FollowUpWindow>;
```

**Aging-boundary edge coordinates (MUST be explicit L1 cases):** day **2** → `None`; day **3**
→ `FirstFollowUp` (window opens); day **5** → `FirstFollowUp` (window closes); day **6** → gap
(`None` — between windows; confirm in EARS); day **7** → `SecondFollowUp`; day **10** →
`SecondFollowUp` (closes); day **11** → `Archive`. The boundaries 3, 5, 7, 10, archive are each
their own pinned coordinate (a fence-post off-by-one fails a test).

### 1.3 Daily call sheet builder (`callsheet.rs`) — R-CSH-*

Deterministically build, for a given `today`, the list of applications needing action. Pure:
takes a slice of tracker records + contacts + `today`, returns the sorted call sheet. Draft
message text uses **deterministic templates** only (LLM advocate stays optional/flagged, NOT
invoked here).

```rust
pub struct CallSheetRow {
    pub application_id: String,
    pub company: String,           // from the linked NormalizedJob
    pub role: String,              // job title
    pub application_date: Date,
    pub follow_up_window: FollowUpWindow,
    pub contact: Option<ContactRef>,   // name/org/channel if linked
    pub suggested_channel: Channel,    // from the contact, else a deterministic default
    pub next_action: NextAction,       // derived from state + aging
    pub draft_message: String,         // deterministic template fill (no LLM)
    pub priority_score: u32,           // deterministic ordering key (see below)
}

/// Pure: applications needing action on `today`, sorted by priority_score desc then id (stable).
/// `today` injected — NO wall clock. Only rows whose aging_action != None/Archive-handled appear.
pub fn build_call_sheet(apps: &[Application], contacts: &[Contact], today: Date) -> Vec<CallSheetRow>;
```

`priority_score` is a deterministic function of state + days-overdue + (optional) role priority —
pinned by an L1 test so ordering is reproducible. `draft_message` selects a template by
`NextAction` and fills company/role/contact — string in, string out, no model.

### 1.4 Recruiter/contact CRM model (`crm.rs`) — R-CRM-*

A contact entity + notes/outcomes + linkage to applications. On-device only; pure data + pure
transitions over notes.

```rust
pub enum Channel { Email, Phone, LinkedIn, Other }

pub struct Contact {
    pub id: String,            // deterministic synthetic id, e.g. "ct_<n>"
    pub name: String,
    pub org: String,
    pub role: String,
    pub channel: Channel,
}

pub enum Outcome { Contacted, Replied, Voicemail, NextStep }

pub struct Note { pub at: Date, pub outcome: Outcome, pub text: String }

pub struct Application {
    pub id: String,                  // deterministic synthetic id, e.g. "ap_<n>"
    pub job: aa_core::NormalizedJob,  // linked by value (referenced from earlier slices by id/ref)
    pub document_ids: Vec<String>,    // generated CV/cover-letter evidence/doc ids from prior slices
    pub state: AppState,
    pub submitted: Option<Date>,      // set when state reaches Applied; drives the scheduler
    pub contact_id: Option<String>,   // linkage to a Contact
    pub notes: Vec<Note>,             // event timeline (CRM outcomes)
}

/// Pure helpers (no clock): append a note, resolve a contact by id, etc. — all value→value.
pub fn add_note(app: Application, note: Note) -> Application;
pub fn contact_for<'a>(app: &Application, contacts: &'a [Contact]) -> Option<&'a Contact>;
```

The `notes` vec IS the per-application event timeline (brief: "event timeline per application").
Dedup of repeated job ads, reminders via email/calendar, and LLM outreach drafts are **OUT of
this slice** (see DISCUSS items) — they are phase-2 breadth the brief lists but are not the four
core capabilities this slice commits to.

---

## 2. The `TrackerStore` seam + `JsonFileStore` (persistence — thin, in the command layer)

Per `doc/design/item-5-storage-decision.md`. The seam + impl live UNDER
`apps/desktop/src-tauri` (NOT in `aa-tracker`, which stays IO-free).

```rust
/// The tracker document: the whole persisted state (single-writer, on-device).
pub struct TrackerDoc { pub applications: Vec<Application>, pub contacts: Vec<Contact> }

/// The persistence port. The cores never see this — only the command layer does.
pub trait TrackerStore {
    fn load(&self) -> Result<TrackerDoc, StoreError>;
    fn save(&self, doc: &TrackerDoc) -> Result<(), StoreError>;
}

/// This slice's concrete impl: one JSON document, written ATOMICALLY (temp + rename),
/// at an on-device path (OS app-data dir in prod; an injected temp dir in tests).
pub struct JsonFileStore { path: PathBuf }
```

- **Atomic write:** serialize → write to `path.with_extension("tmp")` → `fs::rename` over `path`.
  A crash mid-write leaves the prior good file intact (R-STO-2).
- **Tests inject a temp dir** so no global/home state is touched and the L4 path is deterministic.
- **DISCUSS-STORAGE:** a future `SqlCipherStore` implements the SAME trait — a localized swap, not
  a rewrite. The cores never change (they have no store dependency).

---

## 3. New Tauri commands on `Session` (the thin command surface)

Added to `apps/desktop/src-tauri/src/lib.rs`'s `Session`. The `Session` gains a
`Box<dyn TrackerStore + Send + Sync>` (default `JsonFileStore` at the app-data path; tests inject
a temp-dir store) and an in-memory `TrackerDoc` mirror loaded on first use. Each command is a thin
wrapper: it calls a pure `aa-tracker` core, mutates the in-memory doc, and persists via the store.
The wall clock is read ONCE at the command boundary and passed as `today` into the pure cores —
the cores stay clock-free (R-SCH-*).

| Command | Signature (sketch) | Calls |
|---|---|---|
| `track_application` | `(&mut self, job_json: &str, document_ids: Vec<String>) -> Result<String, CommandError>` | creates an `Application` (state `Discovered`), persists, returns its id |
| `advance_application` | `(&mut self, app_id: &str, to: &str) -> Result<(), CommandError>` | `lifecycle::transition` (illegal → typed `CommandError`); sets `submitted` when entering `Applied` |
| `add_contact` | `(&mut self, name, org, role, channel: &str) -> Result<String, CommandError>` | creates a `Contact`, persists, returns id |
| `link_contact` | `(&mut self, app_id: &str, contact_id: &str) -> Result<(), CommandError>` | sets `Application.contact_id`, persists |
| `add_note` | `(&mut self, app_id: &str, outcome: &str, text: &str, today: …) -> Result<(), CommandError>` | `crm::add_note`, persists |
| `daily_call_sheet` | `(&self, today: …) -> Result<Vec<CallSheetRow>, CommandError>` | `callsheet::build_call_sheet` over the loaded doc + injected `today` |
| `list_applications` | `(&self) -> Result<Vec<Application>, CommandError>` | reads the loaded doc (for the tracker board) |

`CommandError` gains an arm for tracker failures (`Tracker(String)` covering `TransitionError`,
`StoreError`) via `From` impls — mirroring the existing `From<ImportError>`/`From<AdvocateError>`
pattern. The boundary reads `today` from the system clock and passes it down; the cores never do.

`date` transport across the Tauri boundary follows the existing byte/string convention: dates
marshal as `{year,month,day}` objects (serde), `Channel`/`Outcome`/`AppState` as lowercase
strings parsed by a `::parse(&str)` helper (mirrors `ResumeKind::parse`) → typed error on a bad
value, never a panic.

---

## 4. React surfaces (handler-react; UI tests local-only per npm CI reality)

A **tracker board** + **call-sheet view** in `apps/desktop/src`:

- **Tracker board:** columns per `AppState`; each card shows company/role/state + advance buttons
  that call `advanceApplication(appId, toState)`; an illegal transition surfaces the typed error
  with `role="alert"` (no silent failure). Cards link to the contact + notes timeline.
- **Call-sheet view:** for `today`, renders `dailyCallSheet()` rows — company, role, application
  date, follow-up window, contact, suggested channel, next action, draft message, priority score.
  An "export call sheet" affordance writes the deterministic rows (no LLM).
- **Contact panel:** add/edit a contact, link it to an application, append notes (outcome + text).

`apps/desktop/src/commands.ts` gains the typed wrappers (`trackApplication`, `advanceApplication`,
`addContact`, `linkContact`, `addNote`, `dailyCallSheet`, `listApplications`).

**CI reality (issue #2):** the `ui` job stays `continue-on-error` (runners cannot reach npm). UI
component tests (RTL + user-event in `App.test.tsx`/a tracker component test) are authored and run
**locally only**; they do NOT block. The Rust work (crate + command layer + cores) rides the
blocking `rust-workspace` job — that is where the tested coordinates live. **No** npm-install-
dependent blocking job is added.

---

## 5. EARS requirement IDs to author

Four families, allocated for the lifecycle conveyor's EARS step (counts are the expected floor;
the EARS author may add within-range as edge coordinates demand):

| Family | Range | Covers |
|---|---|---|
| **R-TRK-*** | R-TRK-1 … R-TRK-6 | lifecycle state machine: legal transitions, illegal→typed-error, terminal `Closed`, the full matrix is enumerated, an application links a `NormalizedJob` + document ids, deterministic synthetic ids (`ap_<n>`) |
| **R-SCH-*** | R-SCH-1 … R-SCH-7 | scheduler: `days_since` calendar-day math, clock injected (no wall clock in the core), day-2 `None`, day-3/5 `FirstFollowUp`, day-7/10 `SecondFollowUp`, day-11+ `Archive`, window constants |
| **R-CSH-*** | R-CSH-1 … R-CSH-5 | call sheet: all brief fields present per row, deterministic priority ordering, deterministic draft template (no LLM), rows filtered to those needing action on `today`, clock injected |
| **R-CRM-*** | R-CRM-1 … R-CRM-5 | contact entity (name/org/role/channel), note outcomes (contacted/replied/voicemail/next-step), application↔contact linkage, notes-as-event-timeline, deterministic synthetic ids (`ct_<n>`) |
| **R-STO-*** | R-STO-1 … R-STO-3 | `TrackerStore` seam exists; `JsonFileStore` writes atomically (temp+rename, crash-safe); the pure cores have NO store dependency (load/save only in the command layer) |

Plus the boundary/command requirements ride the existing convention (no new family): the Tauri
commands return typed errors not panics; a bad enum string → typed `CommandError`.

---

## 6. Test levels (L1–L5 + perf-delta), with explicit edge coordinates

**L1 unit** (pure cores — 100% of reachable):
- `lifecycle`: enumerate the full `AppState × AppState` matrix; each cell is exactly legal or
  `TransitionError::Illegal`; `Closed` has no outgoing legal edge (non-vacuous: a deliberately
  removed legal edge must flip a case to error).
- `scheduler`: the aging boundaries as named coordinates — **day 2 → None; day 3 → FirstFollowUp;
  day 5 → FirstFollowUp; day 6 → None; day 7 → SecondFollowUp; day 10 → SecondFollowUp; day 11 →
  Archive.** `days_since` across a month boundary (e.g. Jan 30 → Feb 2 = 3 days) and same-day = 0.
- `callsheet`: priority ordering is stable + deterministic; draft template fills company/role;
  a row carries every brief field; clock-injected (two different `today` values yield different
  sheets from the same data).
- `crm`: `add_note` appends to the timeline; `contact_for` resolves linkage; ids are deterministic.

**L2 module:** a small assembled tracker scenario in `aa-tracker` — create app → advance through
`Discovered→Tailored→Applied`, set `submitted`, build the call sheet at several `today` values and
assert the row's window/next-action track the aging rules.

**L3 boundary:** the persisted `TrackerDoc` JSON round-trips (serialize → deserialize → equal),
and validates against a NEW `doc/schemas/tracker-doc.schema.json` (camelCase, `additionalProperties:
false`, required `applications`/`contacts`) via `tools/fake-data/validate.js` (or a `validate-
tracker.js` shim mirroring `validate-job.js`). Negative self-test: a hand-broken doc (extra key /
missing field) yields a non-empty error list (proves the validator is non-vacuous).

**L4 system** (command path, in `apps/desktop/src-tauri/tests/`): drive the new `Session` commands
end to end against a **temp-dir `JsonFileStore`** — `track_application` → `advance_application`
(legal path) → `add_contact` → `link_contact` → `add_note` → `daily_call_sheet` returns the
expected row; an **illegal** `advance_application` returns a typed `CommandError` (non-vacuous twin:
the legal advance succeeds). Assert persistence: a second `JsonFileStore` over the same temp path
`load()`s the same doc (proves atomic save wrote it). All data synthetic, PII-free.

**L5 STORY** (perf-instrumented, gated): a tracker journey — build a synthetic set of applications
+ contacts → advance lifecycle → `daily_call_sheet(today)` → assert the sheet is well-formed →
record a parse-time sample; assert `elapsed < 60s` (I6 absolute) AND `elapsed <= baseline * 3.0`
against a **NEW tracked baseline `doc/perf/desktop-tracker-story-baseline.txt`** (single wall-clock
number, committed, not self-ratcheted — mirrors `doc/perf/README.md`). Reuse the shared gate via
`#[path = "../../../crates/cvimport/tests/perf_gate.rs"]`-include (same pattern as the existing
desktop story tests); the gate's non-vacuity is already proven by `perf_gate_l1.rs`.

**Coverage:** 100% of reachable for the pure cores; documented pragmas only per `doc/COVERAGE.md`.
The blocking `rust-workspace` job (`fmt → clippy -D warnings → cargo test --workspace L1-L5 →
llvm-cov --fail-under-lines 99`) carries all of the above. No npm in the blocking path.

**Test data:** all synthetic — companies/roles/contacts authored in fixtures; any contact emails
use reserved example domains (`@example.com/.org/.net`) so `pii-guard` stays green.

---

## 7. Task decomposition (for the lifecycle-orchestrator + ds-step pool)

The 0–9 loop the conveyor will run for item #5, with handler + reviewer gates at each transition:

| Step | Work | Handler | Reviewer gate(s) at transition |
|---|---|---|---|
| **PLAN** | (this doc + storage decision) — done | foundry-builder-lead | ARCHITECTURE-REVIEWER (crate placement + one-way graph + seam) |
| **EARS** | author R-TRK / R-SCH / R-CSH / R-CRM / R-STO statements | (ears author) | EARS-REVIEWER |
| **FEATURE** | Gherkin scenarios (happy / illegal-transition / aging-boundary / persistence) | (feature author) | BDD-REVIEWER |
| **TEST (red)** | L1–L5 failing tests incl. the aging-boundary coordinates + non-vacuous twins | handler-rust | TEST-DESIGN-REVIEWER |
| **IMPLEMENT (green)** | `aa-tracker` cores + `TrackerStore`/`JsonFileStore` + new `Session` commands; React surfaces | handler-rust (crate + command layer), handler-react (UI) | COVERAGE-REVIEWER, CORRECTNESS-REVIEWER |
| **STORY** | L5 journey + new `desktop-tracker-story-baseline.txt` | handler-rust | REGRESSION-REVIEWER, PERFORMANCE-REVIEWER |
| **DELIVERY** | fmt/clippy/llvm-cov green; SENTINEL::DELIVERY_COMPLETE | lifecycle-orchestrator | ARCHITECTURE-REVIEWER (final), REGRESSION-REVIEWER |

`handler-rust` owns the crate, the cores, the persistence seam, and the command layer (the blocking
`rust-workspace` surface). `handler-react` owns the tracker/call-sheet UI (local-only tests; the
`ui` job stays `continue-on-error`). No `handler-tauri` exists in the roster — Tauri command work
maps to `handler-rust` (the commands are plain Rust on `Session`).

---

## 8. DISCUSS — genuine spec gaps + FOUNDER resolutions

> **FOUNDER resolutions (2026-06-13).** The recommended resolutions below are authorized as
> within-spec-intent and the spec is now FROZEN against them — the conveyor builds to these.
> DISCUSS-STORAGE alone is surfaced to the user as a deliberate cross-slice deferral.
>
> - **DISCUSS-FUTUREDATE → RESOLVED:** `days_since` clamps `today < submitted` to `0` →
>   `AgingAction::None`. Pin as an explicit L1 case.
> - **DISCUSS-WINDOW-GAP → RESOLVED:** day 6 → `None`; day 11+ → `Archive`. These are the
>   pinned boundary coordinates.
> - **DISCUSS-DOCREF → RESOLVED:** "by reference" = the evidence/selection ids
>   (`TailoredView.selected_ids`) + a caller-supplied `document_ids: Vec<String>`. No new
>   document-store artefact is introduced this slice.
> - **DISCUSS-DEDUP → RESOLVED (out of scope #5):** carry to a later item; needs a dedup-key
>   definition the strict normalized-job schema does not currently support.
> - **DISCUSS-REMINDERS → RESOLVED (out of scope #5):** the deterministic call sheet IS the
>   in-app reminder surface; external email/calendar/notification channels need their own ADR.
> - **DISCUSS-STORAGE → DEFERRED (surfaced to user):** encryption-at-rest via SQLCipher is
>   deferred to a dedicated storage slice behind the same `TrackerStore` seam; this slice ships
>   the plaintext `JsonFileStore`. See `doc/design/item-5-storage-decision.md`.

### Original DISCUSS detail (do not improvise)

- **DISCUSS-STORAGE** (carried from `doc/design/item-5-storage-decision.md`) — encryption-at-rest
  via SQLCipher is **deferred to a dedicated storage slice**, behind the same `TrackerStore` seam.
  This slice ships the plaintext `JsonFileStore`. Confirm the deferral and that a future storage
  slice runs `/security-gate` (if SENTINEL available) when encryption lands.
- **DISCUSS-DEDUP** — the brief lists "deduplication for repeated job ads across sources." This is
  NOT one of the four committed capabilities and needs a dedup key definition (by normalized
  title+company? by source url, which the strict schema does not carry — see the normalized-job
  strict-vs-rich split). **Recommend OUT of scope for #5**; carry to a later item. Confirm.
- **DISCUSS-REMINDERS** — the brief lists "reminders via email/calendar/web notifications." That
  crosses an integration boundary (email/calendar/OS-notification channels) and would trigger a
  Phase-2.5 ADR. **Recommend OUT of scope for #5** (the deterministic call sheet IS the in-app
  reminder surface this slice ships); defer the external-channel reminders to a dedicated item.
- **DISCUSS-FUTUREDATE** — what should `days_since(submitted, today)` do when `today < submitted`
  (a future-dated or clock-skewed submission)? Recommend **clamp to 0 → `AgingAction::None`** (a
  not-yet-submitted application needs no follow-up), pinned by an L1 case. Confirm the clamp over
  an error.
- **DISCUSS-WINDOW-GAP** — the brief's windows are day 3–5 and day 7–10, leaving **day 6** and
  the post-day-10 region ambiguous. Recommend: day 6 → `None` (between windows, no action surfaced
  until the second window opens); day 11+ → `Archive`. Confirm the gap-day semantics so the
  boundary coordinates are unambiguous.
- **DISCUSS-DOCREF** — applications link the generated CV/cover-letter "documents from earlier
  slices (by reference/id)." Slices 1–3 produce PDFs in-memory at export time and do NOT persist a
  durable document id today (the export is ephemeral). Recommend storing the **evidence/selection
  ids** already present on the `TailoredView` (`selected_ids`) plus a slice-local `document_ids:
  Vec<String>` the caller supplies, rather than inventing a document-store. Confirm that "by
  reference" means evidence ids + caller-supplied labels, not a new persisted artefact store.
</content>
