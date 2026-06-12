# EARS — Item #6 — More CV templates + ATS-readability + keyword-coverage

Branch `item-6-templates-ats`. All DISCUSS items RESOLVED in the plan §6.0 (COO
resolutions); the values below are locked. Three deterministic capabilities, no LLM.

## A — Additional CV templates (`R-TPL-*`)

- **R-TPL-1** — WHEN a caller selects a `CvTemplate`, the Renderer SHALL render the
  tailored view through that template's `.typ` file, producing a non-empty,
  structurally-valid PDF.
- **R-TPL-2** — The system SHALL provide `Classic` (two-column, existing) and
  `Compact` (single-column, ATS-friendly). `Modern` is DEFERRED — the variant is
  OMITTED entirely (no dead branch). Enum = `{Classic, Compact}`.
- **R-TPL-3** — Every CV template SHALL consume the SAME data contract (a
  `MasterCv`/tailored-view conforming to `doc/schemas/master-cv.schema.json`).
- **R-TPL-4** — Each template SHALL be CLI-renderable via
  `typst compile templates/cv/<t>.typ out.pdf --input data=<persona>.cv.json --root .`.
- **R-TPL-5** — WHEN no template is selected, the system SHALL default to `Classic`
  (backward-compatible: behaviour identical to pre-#6).
- **R-TPL-6** — WHERE the export flow runs, the UI SHALL let the user select among
  available templates, threaded to the renderer via a Tauri command parameter.
- **R-TPL-7** — WHEN a template string at the command boundary is unrecognised, the
  system SHALL surface a typed error (`CoreError`), never panic, never silent default.
- **R-TPL-8** — Compact SHALL be documented in `doc/design/pdf-look.md`.

## B — ATS-readability checker (`R-ATS-*`)

- **R-ATS-1** — A PURE `ats_report(template: CvTemplate, view: &TailoredView) -> AtsReport`
  with NO IO and NO PDF parsing (checks framed over template properties + tailored content).
- **R-ATS-2** — `ats_report` SHALL NEVER mutate the view (`&TailoredView`, read-only).
- **R-ATS-3** — WHEN the template is multi-column (Classic) the column-reliance check
  SHALL WARN; WHEN single-column (Compact) it SHALL PASS.
- **R-ATS-4** — The report SHALL WARN on an overly-long document. RESOLVED measure
  (DISCUSS-ATS-LEN): total achievement count across visible/selected experiences > 30.
- **R-ATS-5** — The report SHALL check non-standard section headings. RESOLVED
  (DISCUSS-ATS-HEAD option a): template-property guard — the chosen template's heading
  vocabulary ⊆ a fixed standard allow-list → Pass for Classic/Compact (guard for future).
- **R-ATS-6** — The report SHALL WARN on missing extractable text (empty document, or an
  experience with no description-bearing achievements).
- **R-ATS-7** — Each check is a pinnable coordinate: stable `AtsCheckId` enum +
  `Pass|Warn` + a human message, in a deterministic order.
- **R-ATS-8** — `ats_report` SHALL be deterministic: identical `(template, view)` →
  identical `AtsReport`.
- **R-ATS-9** — Unusual-font check. RESOLVED (DISCUSS-ATS-FONT): an always-Pass
  coordinate keyed off the fixed bundled Liberation stack, with a one-line rationale.

## C — Keyword-coverage panel (`R-KWC-*`)

- **R-KWC-1** — A PURE `keyword_coverage(view: &TailoredView, job: &NormalizedJob) ->
  KeywordCoverage` reporting per-keyword FOUND vs MISSING over the TAILORED view.
  RESOLVED (DISCUSS-KWC-KEY): keys off `job.requirements.must_have`/`nice_to_have`
  (NOT `job.keywords`, which stays reserved).
- **R-KWC-2** — For each FOUND keyword the report SHALL list WHERE: the contributing
  evidence ids via `Candidate::matching_evidence_ids(&view.cv, kw)` INTERSECTED with
  `view.selected_ids` (RESOLVED DISCUSS-KWC-SEL: report only surfaced/tailored locations).
- **R-KWC-3** — The report SHALL distinguish must-have vs nice-to-have keyword classes.
- **R-KWC-4** — Computed over the TAILORED view, distinct from `coverage_report` (master CV).
- **R-KWC-5** — VISIBILITY-ONLY: NEVER insert/stuff/reorder/fabricate (`&` borrows, no mutation).
- **R-KWC-6** — WHEN a keyword appears in ≥2 sections, list ALL contributing evidence ids
  (deduped, deterministically ordered).
- **R-KWC-7** — WHEN a must-have keyword is absent from the tailored view → MISSING with
  an empty evidence list.
- **R-KWC-8** — `keyword_coverage` SHALL be deterministic.

## Feature scenarios (Gherkin sketch)

- Template: "render compact → valid PDF"; "default selects classic"; "unknown template → typed error".
- ATS: "classic → column WARN"; "compact → column PASS"; "empty doc → missing-text WARN"; ">30 achievements → length WARN"; "headings ⊆ allow-list → PASS"; "font always PASS".
- Keyword: "must-have in two sections → all evidence ids deduped"; "must-have absent → MISSING empty evidence"; "nice-to-have found"; "found only in dropped bullet → not surfaced".
