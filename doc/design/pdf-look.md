# PDF "finished look" — design tokens

This document captures the visual design of the original **DW_CV** app so the look is
maintainable without the original React/CSS source. The original rendered a React SPA to PDF via
the browser print dialog; we re-author it deterministically in **Typst** (`templates/cv/classic.typ`).
The CSS files below were the *only* design source and are reference-only — none of that code is in
this repo.

## Source of truth (read-only, in DW_CV — not copied here)
- `cv_spa/src/App.css` — print media rules, typography, page-break control, dark-mode (screen only)
- `cv_spa/src/components/WorkExperience/WorkExperienceJob.scss` — per-job `page-break-inside: avoid`
- `cv_spa/src/components/UI/ListWithRatings.scss` — skill list + proficiency, keep-together
- `cv_spa/src/components/PersonDetails/*.scss` — header / contact styling

## Layout
- **A4**, narrow margins (`x: 14pt, y: 20pt`) — matches `.App { padding-left: 14pt }`.
- **Full-width header**: name (xx-large/22pt, bold) · title (muted) · single contact line.
- **Two-column body** via a grid: **left sidebar ≈ 32%** (the DW_CV 3-of-15 column) holding the
  professional summary + skills; **right ≈ 1fr** (the 12-of-15 column) holding experience.
- Thin horizontal rule under the header.

## Typography
- System sans-serif stack (original used `-apple-system … sans-serif`); Typst falls back across
  `Liberation Sans / Helvetica Neue / Arial / DejaVu Sans` for portability.
- Sizes in **pt** for print stability (original mixed `pt` + keyword sizes):
  - Name 22pt/700 · Title 12pt · Section heads 10–12pt/700
  - Body 9–10pt · contact 8.5pt muted
  - **Achievements 7.5pt monospace** (`.experience-achievement-task { font-size: x-small; font-family: monospace }`)
  - **Emphasised achievements** italic, slightly larger (`.emphasised { font-style: italic; font-size: larger }`)
- Colour: ink `#1a1a1a`, muted `#5a5a5a`, faint `#9a9a9a`, rule `#d8d8d8`. (Dark-mode in the original
  was screen-only and intentionally dropped for print.)

## Skills & ratings
- Four categories preserved from DW_CV: **Languages, Skills, Tools & Technologies, Platforms & Services**
  (`programmingLanguages`, `skills`, `toolsTechnologies`, `asAServices`).
- Proficiency 1–5 rendered as **five dots** (filled = level). Original used a label/rating list.

## Page-break discipline (critical to the look)
- Every **job entry** and **skill block** is a `block(breakable: false)` — the direct analogue of the
  original `page-break-inside: avoid`. Jobs never split across a page boundary.

## Experience entry anatomy
`jobTitle` (bold) + date range (right, muted) → `businessName · consultancy · location` (muted) →
achievement bullets (monospace, or italic when emphasised) → tag row (faint, small).

## Reproduction notes for future maintainers
- The look is intentionally minimal; fidelity lives in the **two-column balance**, the
  **monospace micro-achievements**, and the **keep-together** rule. Preserve those three and the
  document reads as "the DW_CV look" regardless of font substitution.
- Additional templates (e.g. `modern.typ`, `compact.typ`) can be added alongside `classic.typ`; the
  data contract is always `doc/schemas/master-cv.schema.json`.

## `compact.typ` — the single-column, ATS-friendly template (item #6)

`templates/cv/compact.typ` is a deliberate ATS-readability contrast to `classic.typ`. It shares the
same `sys.inputs.data` JSON contract, the same built-in placeholder block, and the same bundled
Liberation font stack — only the **layout** differs.

- **Single-column linear flow** (no sidebar grid): full-width header → **Summary** → **Skills** →
  **Experience**, top to bottom. This is what the ATS column-reliance check (`R-ATS-3`,
  `CvTemplate::is_multi_column`) keys off — Classic is multi-column (a smell), Compact is not.
- **Skills as inline grouped lists**, `Label: A · B · C` (e.g. `Languages: Python · Go · TypeScript`),
  with **NO rating dots/circles**. The classic five-dot proficiency rating renders as small filled
  circles, which read as images (not text) to an ATS; Compact drops them so every skill is
  extractable plain text.
- **Standard section headings only** — `Summary`, `Skills`, `Experience` (plus the Classic skill
  sub-labels `Languages` / `Tools & Technologies` / `Platforms & Services`). The heading vocabulary
  is a subset of the ATS standard allow-list (`R-ATS-5`).
- **Achievements as plain `·` bullets** (not monospace micro-text), italic when `emphasise` — again
  prioritising text extractability over the DW_CV micro-typography.
- **Page-break discipline kept**: each job entry stays `block(breakable: false)`.
- **CLI-renderable** (used by the CI `foundation` smoke, `R-TPL-4`):
  ```
  typst compile templates/cv/compact.typ out.pdf \
    --input data=fixtures/personas/persona-001.cv.json --root .
  ```

`Modern` is intentionally **deferred** this cycle — the `CvTemplate` enum ships exactly
`{Classic, Compact}` with no dead variant.
