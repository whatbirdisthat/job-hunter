# SPECIFICATION ONLY — NOT EXECUTABLE

> Item 8a — Adaptive JSON miner + completeness (PURE engine). EARS requirements (Step 1) + Gherkin
> acceptance scenarios (Step 2) for ROADMAP item 8a. Makes the Master-CV schema **INTERNAL-ONLY** by
> mining the fields the app needs out of **arbitrary** CV JSON. Deterministic; **NO LLM, NO network**.
> Master CV immutable — builds a **NEW** `aa_core::MasterCv` (I1). Authored by
> `lifecycle-orchestrator` from FOUNDRY_PLAN.md (Item 8a) against the verified code in
> `crates/cvimport/{lib,map,segment,error}.rs` + `crates/core/src/types.rs`.
>
> The `.feature`-style scenarios below are **specification only** and live under `doc/spec/`
> (same convention as `doc/spec/item-2-resume-import.md`) — each maps to an executable Rust test
> (L1–L5) carrying the same `R-INGEST-*` id in a comment. The traceability table at the foot names
> the proving test for every requirement.
>
> Item **8b** (the CLI flow that renders `CompletenessReport` to a user) is a SEPARATE item and is
> **NOT** specified here. It consumes `import_cv_json` + `completeness` + `CompletenessReport` as
> defined below.

---

## Ratified DISCUSS decisions (pinned into the EARS below)

- **DISCUSS-8a-1 (API shape).** `completeness(cv: &MasterCv, ignored_role_arrays: &[String]) ->
  CompletenessReport` is a **separate pure function** (NOT folded into `import_cv_json`'s return).
  `ignored_role_arrays` is not reconstructable from a collapsed `MasterCv`, so it is passed in. Pinned
  in R-INGEST-11/12.
- **DISCUSS-8a-2 / R-INGEST-13.** An experience element is emitted **iff it yields a non-empty
  `jobTitle`**; absent `businessName`/`startDate` become `""`. The completeness report flags the
  absence of a jobTitle+businessName experience. L3 schema-validity is asserted **only** on fixtures
  whose emitted experiences carry all three of `jobTitle`/`businessName`/`startDate`. Keeps
  `validate.js` unchanged and honest.
- **DISCUSS-8a-3 / R-INGEST-7.** A present numeric proficiency is used **only when it is already an
  integer in `1..=5`**; anything else (0, 7, 4.5, `"expert"`, a 0–100 scale) collapses to the honest
  default `3`. Never invents a scale mapping.

---

## EARS requirements (Step 1) — the R-INGEST family

| ID | EARS statement |
|---|---|
| **R-INGEST-1** | WHEN `import_cv_json` is given a JSON object, the miner SHALL map person fields by **case-insensitive synonym keys** (name/fullName/candidateName; professionalTitle/title/headline/role/label; professionalDescription/summary/about/bio; email; phone; location; linkedin; github; website/url) onto `aa_core::Person`, never by fixed JSON paths. The FIRST present, non-empty synonym in each priority order wins. |
| **R-INGEST-2** | WHEN a dedicated `person`/`basics`/`profile` object is present, the miner SHALL prefer it (in that priority order) as the person source, falling back to the top-level object for contact fields it does not contain. |
| **R-INGEST-3** | WHEN one or more experience elements are present under the winning role-array, the miner SHALL map each to one `experience[]` entry by synonym keys (jobTitle/title/position/role; businessName/company/employer/organisation/**name**; startDate/start/from; endDate/end/to). The `name` businessName synonym is LOWEST priority and consulted ONLY inside a role-array element (JSON Resume keys the employer as `work[].name` — DISCUSS-8a-4). |
| **R-INGEST-4** | WHEN multiple candidate role-shaped arrays are present, the miner SHALL select the highest-priority synonym key (`experience` → `work` → `workExperience` → `employment` → `positions` → `history`) holding a **non-empty array**, SHALL ignore the others, and SHALL NAME every ignored array (its source key) in the completeness report (no silent merge, v1). |
| **R-INGEST-5** | WHEN an achievement value is a single string, the miner SHALL split it into bullets on newlines (trimming, dropping empties); a string with **no** newline SHALL remain exactly one bullet. Achievement elements MAY be strings OR objects keyed `description`/`text`/`name` (that priority). |
| **R-INGEST-6** | WHEN a date field is a JSON number, the miner SHALL coerce it to its integer string (e.g. `2019` → `"2019"`); a string date SHALL pass through verbatim (odd formats unaltered). |
| **R-INGEST-7** | The miner SHALL map skill arrays under `programmingLanguages` / `skills` / `languages` / `tools` / `technologies` / `toolsTechnologies` / `asAServices` / `services` into the corresponding master-CV list (default `skills`), each element a string OR `{name, proficiency\|level\|rating}`; proficiency SHALL be the source rating **only when it is an integer in `1..=5`**, otherwise the honest default `3` (never invented, never inflated). |
| **R-INGEST-8** | The miner SHALL assign deterministic synthetic ids `imp_exp_N` (N = 0-based source index) to experiences and `imp_exp_N_bM` (M = 0-based bullet index) to achievements, so the same input value yields **byte-identical** output. |
| **R-INGEST-9** | The miner SHALL build the `MasterCv` **directly** and SHALL NOT route through the text `Segments`/`map::to_master_cv` path (which would drop the contact block and real proficiencies); it SHALL produce a NEW document and SHALL NOT mutate any input (I1). |
| **R-INGEST-10** | The miner SHALL emit output that deserializes as `aa_core::MasterCv` AND validates against `doc/schemas/master-cv.schema.json` (`schemaVersion = "1.0.0"`; required `person`/`experience`; `person` additionalProperties:false; every skill proficiency an integer 1..5; every experience `id/jobTitle/businessName/startDate` non-empty; every achievement `id/description` non-empty). |
| **R-INGEST-11** | `completeness(&MasterCv, &[ignored])` SHALL report which IMPORTANT classes are empty: `missing_person_name` (person.name empty); `missing_experience` (no experience with BOTH jobTitle AND businessName non-empty); `missing_achievement` (no achievement.description anywhere); `missing_skill` (no skill in any of the four lists). `is_complete()` SHALL be true iff all four flags are false. |
| **R-INGEST-12** | `completeness` SHALL list, in `ignored_role_arrays`, the source key names of every role-shaped array ignored during disambiguation (R-INGEST-4), passed through verbatim, so item 8b's CLI can surface them. |
| **R-INGEST-13** | An experience source element SHALL be emitted as an `experience[]` entry **iff it yields a non-empty `jobTitle`**; absent `businessName`/`startDate` SHALL be `""`; the completeness report SHALL flag absence of a jobTitle+businessName experience. (Pins DISCUSS-8a-2; keeps `validate.js` unchanged. L3 schema-validity is asserted only on fixtures whose emitted experiences carry all three required fields.) |
| **R-INGEST-14** | IF the value carries no recognisable CV content (no person name **AND** no experience **AND** no skill), THEN `import_cv_json` SHALL return `Err(ImportError::Empty)` and SHALL NOT panic on **any** JSON input (typed-error guarantee, I5). No new `ImportError` arm is added — `Empty` ("produced no recognisable content") is format-agnostic and fits exactly. |

### Recorded design decisions (per plan)

- **Build `MasterCv` directly (R-INGEST-9).** `segment::Segments`/`map::to_master_cv` are `pub(crate)`,
  have no contact slots, and hardcode `IMPORTED_PROFICIENCY = 3` for every skill. Routing arbitrary CV
  JSON through them would silently drop the entire contact block and every real proficiency — fatal for
  the real DW_CV file. The miner constructs `aa_core::{MasterCv, Person, Skill, Experience, Achievement}`
  itself. It reuses ONLY two **conventions** from `map.rs` (re-implemented locally, not the symbols):
  the id format (`imp_exp_N` / `imp_exp_N_bM`) and the honesty default
  (`const DEFAULT_PROFICIENCY: u8 = 3` — proficiency 3 only when the source carries no rating).
- **Synonym matching (R-INGEST-1/3/7).** Case-insensitive KEY match via a `lc_get` primitive — never
  fixed JSON paths. Each field has a left→right priority order; the first present, non-empty key wins.
- **Empty/whitespace → absent.** A person contact field whose value is empty or whitespace-only maps to
  `None` (honesty: don't emit blank fields; also keeps `person` additionalProperties clean for L3).
- **`languages` bucketing (known limitation v1).** `languages` is bucketed to `skills` — the
  spoken-vs-programming ambiguity is out of scope for v1 (§ Known limitations).

---

## Gherkin acceptance scenarios (Step 2) — happy / unhappy / abuse

```gherkin
Feature: Adaptive JSON miner — arbitrary CV JSON → master-CV schema (internal-only schema)

  # ── happy: the DW_CV-shaped (PascalCase legacy) file — the motivating regression ───────────
  @R-INGEST-1 @R-INGEST-7 @R-INGEST-9 @R-INGEST-10
  Scenario: A DW_CV-shaped JSON mines into a valid master CV preserving contact + real proficiencies
    Given a synthetic DW_CV-shaped JSON (PascalCase Name/ProfessionalTitle/WorkExperience/ProgrammingLanguages)
    When I import_cv_json(value)
    Then person.name, person.email, person.linkedin, person.github and person.website are all recovered
    And each programming-language skill keeps its real source proficiency (an integer 1..5, not forced to 3)
    And the experiences carry recovered jobTitle/businessName/startDate
    And the serialized output validates against master-cv.schema.json

  # ── happy: JSON-Resume shape (dedicated `basics` object preference + alt synonyms) ──────────
  @R-INGEST-2 @R-INGEST-3 @R-INGEST-5
  Scenario: A JSON-Resume-shaped value mines via the basics object and work/position/highlights synonyms
    Given a JSON-Resume-shaped value with basics{name,label,summary,email} and work[]{position,name,startDate,highlights}
    When I import_cv_json(value)
    Then person.name and person.professionalTitle come from the basics object
    And person.email falls back to / is read from the available source
    And each work[] element maps to one experience via position/name/startDate synonyms
    And each highlight maps to one achievement bullet
    And the serialized output validates against master-cv.schema.json

  # ── happy: multi-array disambiguation (highest-priority key wins, the rest are NAMED) ───────
  @R-INGEST-4 @R-INGEST-12
  Scenario: When both experience[] and work[] are present, experience[] wins and work is named ignored
    Given a value with a non-empty experience[] AND a non-empty work[]
    When I import_cv_json(value) then completeness(cv, ignored)
    Then the experiences come only from experience[]
    And the completeness report's ignored_role_arrays contains "work"

  # ── happy: numeric date coercion ───────────────────────────────────────────────────────────
  @R-INGEST-6
  Scenario: A numeric startDate/endDate is coerced to its integer string
    Given an experience with startDate 2019 (a JSON number) and endDate 2022 (a JSON number)
    When I import_cv_json(value)
    Then the experience startDate is "2019" and endDate is "2022"

  # ── happy: achievement newline split ───────────────────────────────────────────────────────
  @R-INGEST-5
  Scenario: A newline-joined achievement blob becomes multiple bullets; a no-newline blob stays one
    Given an experience whose achievement value is "line one\nline two" and a sibling "single line"
    When I import_cv_json(value)
    Then "line one\nline two" yields two achievement bullets
    And "single line" yields exactly one achievement bullet

  # ── happy: proficiency honesty (R-INGEST-7 / DISCUSS-8a-3) ──────────────────────────────────
  @R-INGEST-7
  Scenario Outline: A source proficiency is used only when an integer in 1..5, else default 3
    Given a skill {name, <ratingKey>: <ratingValue>}
    When I import_cv_json(value)
    Then the produced skill proficiency is <expected>

    Examples:
      | ratingKey   | ratingValue | expected |
      | proficiency | 4           | 4        |
      | level       | 1           | 1        |
      | rating      | 5           | 5        |
      | proficiency | 0           | 3        |
      | level       | 7           | 3        |
      | rating      | 4.5         | 3        |
      | proficiency | "expert"    | 3        |
      | (absent)    | (absent)    | 3        |

  # ── happy: completeness over a sparse but valid value ──────────────────────────────────────
  @R-INGEST-11 @R-INGEST-13
  Scenario: A minimal value (name + one skill) yields a valid CV; completeness flags what is missing
    Given a value { "name": "A. Tester", "skills": ["Rust"] }
    When I import_cv_json(value) then completeness(cv, [])
    Then person.name is "A. Tester" and one skill is present
    And the completeness report has missing_experience true and missing_achievement true
    And missing_person_name false and missing_skill false
    And is_complete() is false

  # ── unhappy: the Empty gate (no recognisable content) ──────────────────────────────────────
  @R-INGEST-14
  Scenario: A value with no person name, no experience and no skill returns Err(ImportError::Empty)
    Given the value {} (and a sibling { "notes": "hi" })
    When I import_cv_json(value)
    Then it returns Err(ImportError::Empty) and does not panic

  # ── abuse: arbitrary / hostile JSON shapes never panic (typed-error guarantee, I5) ─────────
  @R-INGEST-14
  Scenario Outline: Arbitrary JSON values are total — never a panic
    Given a hostile value <value>
    When I import_cv_json(value)
    Then it returns Ok or Err(ImportError::Empty) but never panics

    Examples:
      | value                                              |
      | a JSON array at the top level                      |
      | a JSON string at the top level                     |
      | a JSON number at the top level                     |
      | null                                               |
      | { "experience": "not-an-array" }                   |
      | { "experience": [ 42, "x", { "jobTitle": "T" } ] } |
      | { "skills": { "name": "wrong-shape" } }            |
      | { "name": "   " }  (whitespace-only name)          |

  # ── determinism (R-INGEST-8) ───────────────────────────────────────────────────────────────
  @R-INGEST-8
  Scenario: The same value mines to byte-identical output across runs
    Given any value V
    When I import_cv_json(V) twice and serialize each
    Then the two serializations are byte-identical
    And every experience id matches imp_exp_N and every achievement id matches imp_exp_N_bM
```

---

## Known limitations (v1, recorded — no behaviour)

- Non-English synonym keys are out of scope; recorded, no behaviour.
- The spoken-vs-programming `languages` ambiguity: `languages` is bucketed to `skills` in v1.
- No silent merge of multiple role-shaped arrays: the highest-priority wins, the rest are **named**
  (not merged) in `ignored_role_arrays`.
- Proficiency scales other than integer-`1..5` collapse to the honest default `3` (never an invented
  scale mapping).

---

## Traceability — R-INGEST → proving test

| R-INGEST | Proving test(s) |
|---|---|
| 1 | L1 `person_*_synonym_order` tests; L2 `dwcv_recovers_name_and_contact_block` |
| 2 | L1 `dedicated_person_object_preference_and_fallback`; L2 `json_resume_recovers_from_basics` |
| 3 | L1 `experience_synonym_mapping`; L2 `json_resume_work_position_highlights` |
| 4 | L1 `multi_role_array_disambiguation_names_ignored`; L4 `multi_role_arrays_reports_ignored` |
| 5 | L1 `achievement_newline_split_and_object_form` |
| 6 | L1 `numeric_date_coerced_string_verbatim`; L2 `numeric_dates_fixture_coerces` |
| 7 | L1 `skill_bucketing_and_proficiency_honesty`; L2 `dwcv_preserves_real_proficiencies` |
| 8 | L1 `synthetic_id_synthesis`; L2 `import_is_deterministic_byte_identical` |
| 9 | L2 `dwcv_contact_block_preserved_proves_not_segment_routed`; L1 `input_value_not_mutated` |
| 10 | L3 `mined_dwcv_validates`; L3 `mined_json_resume_validates` |
| 11 | L1 `completeness_flags_each_class`; L4 `minimal_flags_missing_experience_and_achievement` |
| 12 | L1 `completeness_lists_ignored_role_arrays`; L4 `multi_role_arrays_reports_ignored` |
| 13 | L1 `experience_emitted_iff_nonempty_jobtitle` |
| 14 | L1 `empty_gate_and_abuse_never_panics`; L2 `empty_json_returns_err_empty` |
