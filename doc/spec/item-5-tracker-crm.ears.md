# Item #5 — Application tracker / CRM — EARS requirements + FEATURE scenarios

> **Specification only.** Each `R-*` statement below is an EARS requirement; each Gherkin
> scenario in the FEATURE section is spec-only and carries the `R-*` id it proves in a comment.
> The executable test that discharges each requirement is named in the traceability table at the
> foot. Authored against the FROZEN spec `doc/spec/item-5-tracker-crm.md` (FOUNDER resolutions
> §8) and the storage decision `doc/design/item-5-storage-decision.md`. No relitigation.

---

## EARS requirements

### R-TRK-* — lifecycle state machine (`crates/tracker/src/lifecycle.rs`)

- **R-TRK-1** — The lifecycle core SHALL define exactly the states
  `Discovered, Tailored, Applied, FollowUpDue, Interview, Closed`.
- **R-TRK-2** — WHEN `transition(from, to)` is called with an ordered pair listed in
  `legal_transitions()`, the lifecycle core SHALL return `Ok(to)`.
- **R-TRK-3** — WHEN `transition(from, to)` is called with an ordered pair NOT listed in
  `legal_transitions()`, the lifecycle core SHALL return `Err(TransitionError::Illegal { from, to })`
  and SHALL NOT panic and SHALL NOT silently no-op.
- **R-TRK-4** — The lifecycle core SHALL treat `Closed` as terminal: `legal_transitions()` SHALL
  contain no edge whose `from` is `Closed`.
- **R-TRK-5** — The legal-transition table SHALL be exposed as data (`legal_transitions()`) so a
  test can enumerate the full `AppState × AppState` matrix and assert each cell is exactly
  legal-or-error (the table is proven complete and non-vacuous).
- **R-TRK-6** — An `Application` SHALL link a `aa_core::NormalizedJob` (by value) plus a
  caller-supplied `document_ids: Vec<String>`, and SHALL be identified by a deterministic synthetic
  id of the form `ap_<n>`.

### R-SCH-* — follow-up scheduler (`crates/tracker/src/scheduler.rs`)

- **R-SCH-1** — `days_since(submitted, today)` SHALL return the whole calendar-day count from
  `submitted` to `today`, counting across month and year boundaries (e.g. Jan 30 → Feb 2 = 3).
- **R-SCH-2** — WHEN `today < submitted` (a future-dated or clock-skewed submission),
  `days_since` SHALL clamp the result to `0` (never negative) — DISCUSS-FUTUREDATE resolution.
- **R-SCH-3** — The scheduler SHALL be clock-injected: `today` is ALWAYS a parameter; no scheduler
  function SHALL read the wall clock.
- **R-SCH-4** — WHEN the day offset is in `0..=2`, `aging_action` SHALL return `AgingAction::None`;
  the day-**2** coordinate SHALL be `None`.
- **R-SCH-5** — WHEN the day offset is in `3..=5`, `aging_action` SHALL return
  `AgingAction::FirstFollowUp`; the day-**3** (opens) and day-**5** (closes) coordinates SHALL both
  be `FirstFollowUp`.
- **R-SCH-6** — WHEN the day offset is **6**, `aging_action` SHALL return `AgingAction::None`
  (the gap between windows — DISCUSS-WINDOW-GAP resolution); WHEN in `7..=10`, it SHALL return
  `AgingAction::SecondFollowUp` (day-**7** opens, day-**10** closes).
- **R-SCH-7** — WHEN the day offset is `>= 11`, `aging_action` SHALL return `AgingAction::Archive`;
  and `follow_up_window` SHALL return the deterministic constant window
  (`{3,5}` for `FirstFollowUp`, `{7,10}` for `SecondFollowUp`, `None` for `None`/`Archive`).

### R-CSH-* — daily call-sheet builder (`crates/tracker/src/callsheet.rs`)

- **R-CSH-1** — `build_call_sheet(apps, contacts, today)` SHALL return a `CallSheetRow` carrying
  every brief field per row: `application_id, company, role, application_date, follow_up_window,
  contact, suggested_channel, next_action, draft_message, priority_score`.
- **R-CSH-2** — The call sheet SHALL be ordered deterministically by `priority_score` descending,
  then by `application_id` ascending (a stable, reproducible total order).
- **R-CSH-3** — `draft_message` SHALL be produced by a deterministic template selected by
  `next_action` and filled with company/role/contact — string in, string out, NO LLM invoked.
- **R-CSH-4** — The call sheet SHALL include only applications needing action on `today` — those
  whose `aging_action` is `FirstFollowUp` or `SecondFollowUp` — and SHALL EXCLUDE rows whose action
  is `None` or `Archive`, and rows with no `submitted` date.
- **R-CSH-5** — The call-sheet builder SHALL be clock-injected: two different `today` values over
  the same applications SHALL be able to yield different sheets (no wall clock read).

### R-CRM-* — recruiter/contact CRM model (`crates/tracker/src/crm.rs`)

- **R-CRM-1** — A `Contact` SHALL carry `{ id, name, org, role, channel }` where `channel` is one of
  `Email, Phone, LinkedIn, Other`, and SHALL be identified by a deterministic synthetic id `ct_<n>`.
- **R-CRM-2** — A `Note` SHALL carry `{ at: Date, outcome: Outcome, text: String }` where `outcome`
  is one of `Contacted, Replied, Voicemail, NextStep`.
- **R-CRM-3** — `add_note(app, note)` SHALL append the note to the application's `notes` timeline and
  return the updated application (value in, value out; no clock, no IO).
- **R-CRM-4** — An `Application` SHALL link to at most one `Contact` via `contact_id`, and
  `contact_for(app, contacts)` SHALL resolve that linkage (returning `None` when unset or unresolved).
- **R-CRM-5** — The `notes` vector SHALL BE the per-application event timeline, preserving insertion
  order (newest appended last).

### R-STO-* — persistence seam (`apps/desktop/src-tauri` — the command layer)

- **R-STO-1** — A `TrackerStore` trait SHALL define the persistence port (`load`/`save` a
  `TrackerDoc { applications, contacts }`); the pure cores SHALL have NO dependency on it.
- **R-STO-2** — `JsonFileStore::save` SHALL write ATOMICALLY (serialize → write a temp sibling →
  `rename` over the target), so a crash mid-write leaves the prior good file intact.
- **R-STO-3** — Loading a previously-saved document SHALL round-trip equal (`save` then a fresh
  `load` over the same path yields the same `TrackerDoc`); the persisted JSON SHALL validate against
  `doc/schemas/tracker-doc.schema.json` (camelCase, `additionalProperties:false`, required
  `applications`/`contacts`).

### Boundary/command requirements (ride the existing convention — no new family)

- **R-TRK-CMD-1** — The new `Session` commands (`track_application`, `advance_application`,
  `add_contact`, `link_contact`, `add_note`, `daily_call_sheet`, `list_applications`) SHALL return a
  typed `CommandError`, never a panic, on bad input.
- **R-TRK-CMD-2** — `advance_application` SHALL set `submitted` to the boundary `today` WHEN and ONLY
  WHEN the application enters the `Applied` state; an illegal transition SHALL surface as
  `CommandError::Tracker`.
- **R-TRK-CMD-3** — `Channel`/`Outcome`/`AppState` SHALL parse from lowercase strings via a
  `::parse(&str)` helper (mirroring `ResumeKind::parse`) → typed error on a bad value, never a panic.
- **R-TRK-CMD-4** — The command boundary SHALL read the wall clock exactly ONCE and pass `today`
  into the pure cores; the cores SHALL stay clock-free (R-SCH-3).

---

## FEATURE — Gherkin scenarios (spec-only; `R-*` links to executable tests)

```gherkin
Feature: Application lifecycle state machine
  # R-TRK-2 — a legal transition advances the state
  Scenario: Tailoring a discovered application
    Given an application in state Discovered
    When it transitions to Tailored
    Then the new state is Tailored

  # R-TRK-3 — an illegal transition is a typed error, never a silent no-op
  Scenario: Cannot skip straight from Discovered to Interview
    Given an application in state Discovered
    When it transitions to Interview
    Then a TransitionError::Illegal { from: Discovered, to: Interview } is returned

  # R-TRK-4 — Closed is terminal
  Scenario: A closed application cannot move
    Given an application in state Closed
    When it transitions to any state
    Then a TransitionError::Illegal is returned

  # R-TRK-5 — the full matrix is enumerated (non-vacuous twin)
  Scenario: Every state pair is exactly legal-or-error
    Given the legal_transitions table
    When every ordered (from, to) pair in States x States is evaluated
    Then each pair is either in the table and returns Ok, or absent and returns Err

Feature: Follow-up scheduler (clock-injected aging boundaries)
  # R-SCH-4 — day 2 is before the first window
  Scenario: Day 2 needs no follow-up
    Given an application submitted on day 0
    When today is day 2
    Then the aging action is None

  # R-SCH-5 — first follow-up window opens at day 3, closes at day 5
  Scenario Outline: First follow-up window
    Given an application submitted on day 0
    When today is day <day>
    Then the aging action is FirstFollowUp
    Examples: | day | | 3 | | 5 |

  # R-SCH-6 — day 6 is the gap between windows
  Scenario: Day 6 falls in the inter-window gap
    Given an application submitted on day 0
    When today is day 6
    Then the aging action is None

  # R-SCH-6 — second follow-up window
  Scenario Outline: Second follow-up window
    Given an application submitted on day 0
    When today is day <day>
    Then the aging action is SecondFollowUp
    Examples: | day | | 7 | | 10 |

  # R-SCH-7 — archive beyond day 10
  Scenario: Day 11 archives the application
    Given an application submitted on day 0
    When today is day 11
    Then the aging action is Archive

  # R-SCH-1 — calendar-day math across a month boundary
  Scenario: Days span a month boundary
    Given an application submitted on Jan 30
    When today is Feb 2
    Then days_since is 3

  # R-SCH-2 — future-dated submission clamps to 0 (DISCUSS-FUTUREDATE)
  Scenario: A future-dated submission ages zero days
    Given an application submitted on day 5
    When today is day 0
    Then days_since is 0 and the aging action is None

Feature: Daily call sheet (deterministic, no LLM)
  # R-CSH-1 / R-CSH-4 — rows carry every field; only actionable rows appear
  Scenario: Call sheet surfaces an application due for first follow-up
    Given an application submitted on day 0 linked to a contact
    When the call sheet is built for day 3
    Then the row carries company, role, application date, follow-up window, contact,
      suggested channel, next action, draft message, and priority score

  # R-CSH-2 — deterministic ordering
  Scenario: Rows are ordered by priority then id
    Given two actionable applications with different priority scores
    When the call sheet is built
    Then the higher priority_score row sorts first, ties broken by application_id

  # R-CSH-3 — deterministic draft template
  Scenario: Draft message is a filled template, no model
    Given an actionable application for company Northwind in role Archivist
    When the call sheet is built
    Then the draft_message contains "Northwind" and "Archivist" and no model is invoked

  # R-CSH-5 — clock-injected
  Scenario: Different days yield different sheets
    Given a fixed set of applications
    When the call sheet is built for day 3 and again for day 100
    Then the two sheets differ

Feature: Recruiter/contact CRM
  # R-CRM-3 / R-CRM-5 — notes are the event timeline
  Scenario: Appending a note extends the timeline
    Given an application with one note
    When a Replied note is added
    Then the timeline has two notes in insertion order

  # R-CRM-4 — contact linkage
  Scenario: Resolving a linked contact
    Given an application linked to contact ct_0
    When contact_for is resolved against the contact list
    Then the ct_0 contact is returned

Feature: Persistence seam (atomic JSON file store)
  # R-STO-3 — round-trip
  Scenario: A saved tracker document loads back equal
    Given a tracker document with applications and contacts
    When it is saved to a temp path and loaded from a fresh store
    Then the loaded document equals the saved one

  # R-STO-2 — atomic write leaves the prior file intact on crash
  Scenario: A crash mid-write does not corrupt the live file
    Given a previously saved good document
    When a save is interrupted after the temp write but before rename
    Then the live file still holds the prior good document

Feature: Tauri command surface (typed errors, clock read once)
  # R-TRK-CMD-2 — advancing into Applied stamps submitted; illegal advance is typed error
  Scenario: Advancing through the lifecycle via commands
    Given a tracked application
    When it is advanced Discovered -> Tailored -> Applied
    Then submitted is set on entering Applied
    And an illegal advance returns CommandError::Tracker

  # R-TRK-CMD-3 — bad enum string is a typed error, never a panic
  Scenario: A bad channel string is rejected
    Given add_contact is called with channel "smoke-signal"
    Then a typed CommandError is returned, not a panic
```

---

## Traceability — requirement → proving test

| Requirement | Proving test (level) |
|---|---|
| R-TRK-1 | `crates/tracker/src/lifecycle.rs` unit `all_states_present` (L1) |
| R-TRK-2 | `crates/tracker/src/lifecycle.rs` unit `legal_edges_transition_ok` (L1) |
| R-TRK-3 | `crates/tracker/src/lifecycle.rs` unit `illegal_edge_is_typed_error` (L1) |
| R-TRK-4 | `crates/tracker/src/lifecycle.rs` unit `closed_is_terminal` (L1) |
| R-TRK-5 | `crates/tracker/src/lifecycle.rs` unit `full_matrix_is_legal_or_error` + non-vacuous `removing_an_edge_flips_a_cell` (L1) |
| R-TRK-6 | `crates/tracker/src/crm.rs` unit `application_links_job_and_ids` + `synthetic_ap_id` (L1) |
| R-SCH-1 | `crates/tracker/src/scheduler.rs` unit `days_since_month_boundary` (L1) |
| R-SCH-2 | `crates/tracker/src/scheduler.rs` unit `future_today_clamps_to_zero` (L1) |
| R-SCH-3 | structural — no scheduler fn takes/reads a clock (whole module signature) |
| R-SCH-4 | `crates/tracker/src/scheduler.rs` unit `day_2_is_none` (L1) |
| R-SCH-5 | `crates/tracker/src/scheduler.rs` unit `day_3_and_5_first_follow_up` (L1) |
| R-SCH-6 | `crates/tracker/src/scheduler.rs` unit `day_6_is_none`, `day_7_and_10_second_follow_up` (L1) |
| R-SCH-7 | `crates/tracker/src/scheduler.rs` unit `day_11_is_archive`, `follow_up_window_constants` (L1) |
| R-CSH-1 | `crates/tracker/src/callsheet.rs` unit `row_carries_every_field` (L1) |
| R-CSH-2 | `crates/tracker/src/callsheet.rs` unit `ordered_by_priority_then_id` (L1) |
| R-CSH-3 | `crates/tracker/src/callsheet.rs` unit `draft_template_fills_company_role` (L1) |
| R-CSH-4 | `crates/tracker/src/callsheet.rs` unit `only_actionable_rows` (L1) |
| R-CSH-5 | `crates/tracker/src/callsheet.rs` unit `clock_injected_two_days_differ` (L1) |
| R-CRM-1 | `crates/tracker/src/crm.rs` unit `contact_fields_and_synthetic_ct_id` (L1) |
| R-CRM-2 | `crates/tracker/src/crm.rs` unit `note_outcomes` (L1) |
| R-CRM-3 | `crates/tracker/src/crm.rs` unit `add_note_appends` (L1) |
| R-CRM-4 | `crates/tracker/src/crm.rs` unit `contact_for_resolves_and_none` (L1) |
| R-CRM-5 | `crates/tracker/src/crm.rs` unit `notes_preserve_insertion_order` (L1) |
| R-TRK (assembled) | `crates/tracker/tests/module_l2.rs` `assembled_tracker_scenario` (L2) |
| R-STO-1/3 | `crates/tracker/tests/boundary_l3.rs` `tracker_doc_round_trips` + schema validate via `tools/fake-data/validate-tracker.js` (L3) |
| R-STO-2 | `apps/desktop/src-tauri/tests/tracker_l4.rs` `atomic_save_survives_interrupted_write` + `second_store_loads_same_doc` (L4) |
| R-TRK-CMD-1..4 | `apps/desktop/src-tauri/tests/tracker_l4.rs` command-journey + illegal-twin + bad-enum (L4) |
| STORY | `apps/desktop/src-tauri/tests/tracker_story_l5.rs` perf-delta gated (L5) |
