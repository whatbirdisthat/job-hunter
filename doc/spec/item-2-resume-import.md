# SPECIFICATION ONLY — NOT EXECUTABLE

> Item #2 — PDF/DOCX résumé import → master-CV schema. EARS requirements (Step 1) + Gherkin
> acceptance scenarios (Step 2) for ROADMAP item #2. Deterministic; **NO LLM** (item #3 owns the LLM
> layer). Authored by `lifecycle-orchestrator` from FOUNDRY_PLAN.md (Item #2), the completed spike
> (`doc/idea/applicant-advocate/spike-resume-import.md`), and SUBJECT_MATTER_UNDERSTANDING.md §12.
>
> The `.feature`-style scenarios below are **specification only** and live under `doc/spec/`
> (DoD §2) — each maps to an executable Rust test (L1–L5) carrying the same `R-CVI-*` id in a
> comment. The traceability table at the foot names the proving test for every requirement.

---

## EARS requirements (Step 1) — the R-CVI family

| ID | EARS statement |
|---|---|
| **R-CVI-1** | WHEN `import_resume` is given PDF bytes with `ResumeKind::Pdf`, the importer SHALL extract the résumé text via `pdf-extract` as a flat character stream (no assumed newline structure). |
| **R-CVI-2** | WHEN `import_resume` is given DOCX bytes with `ResumeKind::Docx`, the importer SHALL extract one text string per `word/document.xml` `w:p` paragraph by walking its `w:t` runs with `zip` + `quick-xml` (decoding text via `BytesText::decode()`). |
| **R-CVI-3** | WHEN a header block is present at the top of the extracted text, the importer SHALL map its first line to `person.name`, its second non-empty line to `person.professionalTitle`, and SHALL set the top-level `headline` to the same professional title. |
| **R-CVI-4** | WHEN a labelled skills segment (`Languages`, `Skills`, `Tools & Technologies`, or `Platforms & Services`) is present, the importer SHALL map its entries into the corresponding master-CV skill list (`programmingLanguages`, `skills`, `toolsTechnologies`, `asAServices`), each skill carrying a default proficiency so the output is schema-valid. |
| **R-CVI-5** | WHEN an experience block (`<jobTitle> … <startDate> – <endDate>` followed by a `<businessName> · <location>` line and bullet lines) is present, the importer SHALL map it to one `experience[]` entry (jobTitle, businessName, startDate, optional endDate/location) and SHALL map each bullet line to one `achievementsTasks[]` entry. |
| **R-CVI-6** | The importer SHALL assign every produced experience node a deterministic synthetic id `imp_exp_N` (N = 0-based block index) and every produced achievement node `imp_exp_N_bM` (M = 0-based bullet index), so the same input bytes yield byte-identical ids. |
| **R-CVI-7** | The importer SHALL emit output that deserializes as `aa_core::MasterCv` AND validates against `doc/schemas/master-cv.schema.json` (`schemaVersion = "1.0.0"`; required `person`/`experience`; no additional properties). |
| **R-CVI-8** | IF the kind is unsupported, OR the bytes are undecodable/garbage, OR extraction yields no recognisable content, THEN the importer SHALL return a typed `ImportError` (`UnsupportedKind` / `Decode` / `Extract` / `Empty`) and SHALL NOT panic. |
| **R-CVI-9** | The importer SHALL produce a NEW master-CV document and SHALL NOT mutate any loaded or installed master CV (I1). |
| **R-CVI-10** | WHEN `import_resume` is invoked at the Tauri command boundary (`Session::import_resume(bytes, kind)`), it SHALL return the parsed MasterCv JSON for user review, and installation SHALL reuse the existing `import_master_cv` validation path; on bad kind/garbage it SHALL return a typed `CommandError` without panicking. |

### Delegated implementation calls (recorded, per plan)

- **Byte transport across the Tauri boundary (R-CVI-10):** `Session::import_resume(bytes: &[u8], kind: &str)` takes raw bytes; the React layer transports the file as a `number[]` (`Array.from(new Uint8Array(...))`) which Tauri marshals to `Vec<u8>`. Deterministic; no encoding ambiguity. The crate's public surface (`import_resume(&[u8], ResumeKind)`) is unchanged.
- **Cue-token vocabulary (R-CVI-4/R-CVI-5):** section labels matched case-insensitively: skills = `languages | skills | tools & technologies | tools and technologies | technologies | platforms & services | platforms and services`; experience header = `experience | employment | work experience | experience history`. Bullet markers stripped: `▹ ‣ • · -` (leading). The `<… Mon YYYY – Mon YYYY|Present>` date tail and the `<business> · <location>` line are recognised structurally. Bounded by R3a (synthetic-persona acceptance bar).

---

## Gherkin acceptance scenarios (Step 2) — happy / unhappy / abuse

```gherkin
Feature: Deterministic résumé import (PDF/DOCX) → master-CV schema

  # ── happy: DOCX (the exact-recovery path) ──────────────────────────────────
  @R-CVI-2 @R-CVI-3 @R-CVI-4 @R-CVI-5 @R-CVI-6 @R-CVI-7
  Scenario: A persona synthesised to DOCX round-trips into a valid master CV
    Given a DOCX synthesised from persona-001 at test time (no committed binary)
    When I import_resume(bytes, Docx)
    Then person.name is "Devin Voss"
    And person.professionalTitle and headline are recovered
    And at least one skill is recovered into a skill list
    And at least one experience has the recovered jobTitle and businessName
    And at least one achievement description is recovered
    And every experience id matches imp_exp_N and every achievement id matches imp_exp_N_bM
    And the serialized output validates against master-cv.schema.json

  # ── happy: PDF (containment path; R3b line-join tolerated) ──────────────────
  @R-CVI-1 @R-CVI-3 @R-CVI-7
  Scenario: A persona rendered to PDF imports to a schema-valid master CV
    Given a PDF rendered from persona-001 via templates/cv/classic.typ at test time
    When I import_resume(bytes, Pdf)
    Then person.name is "Devin Voss"
    And each skill-section label present in the résumé is recognised
    And recovered achievement text is present (containment, not byte-equality)
    And the serialized output validates against master-cv.schema.json

  # ── unhappy / abuse: typed errors, never a panic ───────────────────────────
  @R-CVI-8
  Scenario Outline: Bad input yields a typed ImportError, never a panic
    When I import_resume(<bytes>, <kind>)
    Then the result is Err(<error>)
    Examples:
      | bytes                  | kind | error           |
      | well-formed but text-less | Pdf  | Extract or Empty |
      | truncated zip bytes    | Docx | Decode          |
      | structureless text pdf | Pdf  | Empty           |
      | any bytes              | "xlsx" (unknown kind) | UnsupportedKind |
      | zip whose word/document.xml decompresses beyond the size cap (decompression bomb) | Docx | Decode (bounded read; rejected without OOM) |

  # ── immutability (I1) ──────────────────────────────────────────────────────
  @R-CVI-9 @R-CVI-10
  Scenario: Import produces a new document and never mutates the installed master CV
    Given a Session with an installed master CV A
    When I import_resume a different résumé and install the review JSON as B
    Then B is a new document distinct from A
    And A was never mutated in place (a fresh MasterCv was produced for review)

  # ── boundary (R-CVI-10) ────────────────────────────────────────────────────
  @R-CVI-10
  Scenario: The Tauri command returns review JSON and installs via the existing path
    Given a DOCX résumé as bytes at the command boundary
    When Session::import_resume(bytes, "docx") is invoked
    Then it returns master-CV JSON for review
    And routing that JSON through import_master_cv installs it (reusing slice-1 validation)
    And an unknown kind returns a typed CommandError, not a panic
```

### UI ELEMENTS REQUIRING INTERACTION TESTS

- **"Import résumé (PDF/DOCX)" button** in the onboarding `import` step (alongside the existing
  "Import master CV" JSON button). Test: render → button visible → click → `importResume` invoked →
  returned JSON routed through `importMasterCv` → step advances to `paste`. (RTL + user-event,
  `apps/desktop/src/App.test.tsx` / extended onboarding component test.)

---

## EARS → test → implementation traceability

| EARS | Proving test(s) | Level | Implementation |
|---|---|---|---|
| R-CVI-1 | `extract::pdf` unit + L2 pdf path | L1/L2 | `crates/cvimport/src/extract/pdf.rs` |
| R-CVI-2 | `extract::docx` unit + L2 docx path | L1/L2 | `crates/cvimport/src/extract/docx.rs` |
| R-CVI-3 | `segment`/`map` header unit + L5 | L1/L5 | `segment.rs` (header), `map.rs` (person/headline) |
| R-CVI-4 | `segment`/`map` skills unit + L5 | L1/L5 | `segment.rs` (skills cue), `map.rs` (skill lists) |
| R-CVI-5 | `map` experience unit + L5 | L1/L5 | `segment.rs` (experience cue), `map.rs` (experience+achievements) |
| R-CVI-6 | `map` id-determinism unit | L1 | `map.rs` (`imp_exp_N` / `imp_exp_N_bM`) |
| R-CVI-7 | boundary schema test (validate.js) | L3 | `lib.rs` output + `crates/cvimport/tests/boundary_schema.rs` |
| R-CVI-8 | every `ImportError` arm | L2 | `error.rs` + guards in extract/segment/lib |
| R-CVI-9 | immutability L4 system test | L4 | `Session::import_resume` (new doc; reuse install path) |
| R-CVI-10 | Tauri command L4 + React component test | L4/UI | `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src/{commands.ts,App.tsx}` |

All five test levels (L1–L5) run under `cargo test --workspace`; L3 reuses `tools/fake-data/
validate.js`; each level emits a perf sample; the L5 STORYs carry a perf-delta gate with TWO
independent obligations — the absolute I6 budget AND a regression delta vs a TRACKED baseline
under `doc/perf/` (`cvimport-import-story-baseline.txt`, `desktop-story-baseline.txt`), never the
gitignored `target/`. The shared gate (`crates/cvimport/tests/perf_gate.rs`) is unit-tested for
non-vacuity (a 100× regression FAILS) in `tests/perf_gate_l1.rs`.
