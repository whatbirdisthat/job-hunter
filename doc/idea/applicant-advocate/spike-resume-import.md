# Spike — résumé import parsing strategy (ROADMAP item #2, R3)

> Status: COMPLETE — 2026-06-13. Resolves R3's "run a parsing-strategy spike first".
> Append-only record of the chosen approach + the evidence behind it.

## Question

How should `crates/cvimport` deterministically extract text + structure from an existing
**PDF** and **DOCX** résumé and map it into the canonical master CV
(`doc/schemas/master-cv.schema.json`), with **no LLM**, using libraries that **actually
resolve and build** against the fleet crate mirror (`sparse+http://mirror.local:8088`)?

The mirror is the binding constraint: slice 1 already hit it (the embedded `typst` crate is
uncompilable because the mirror tops out at `time 0.3.48`; see DISCUSS-RENDER / doc/COVERAGE.md
P-COV-3). So a spike that only checks "is the crate listed" is insufficient — the spike must
**build the dependency closure and run a functional round-trip**.

## Method

A throwaway crate (`/tmp/spike`) under the project's mirror config:

1. Probed the sparse index for candidates: `lopdf 0.41`, `pdf-extract 0.10`, `pdf 0.10`,
   `docx-rs 0.4.20`, `docx 1.1.2`, `quick-xml 0.40.1`, `zip` (2.x picked), `jsonschema 0.46`.
   All resolved.
2. `cargo generate-lockfile` for `{pdf-extract, docx-rs, quick-xml, zip}` → 137 packages
   locked, **no `time`-style blocker**.
3. `cargo build` → full closure compiled in ~20 s. Notably `pdf-extract 0.10` pulls
   `lopdf 0.38` + `time 0.3.48` — **within** the mirror ceiling (unlike embedded typst).
4. Functional round-trips:
   - **PDF:** rendered a tiny résumé via the project's `typst` CLI, then
     `pdf_extract::extract_text` recovered the text.
   - **DOCX read:** built a synthetic DOCX with `docx-rs`, then walked `word/document.xml`
     `w:t` runs with `zip` + `quick-xml` → clean per-paragraph text.

## Findings

- **PDF text is a flat stream, not lines.** `pdf-extract` concatenated adjacent layout lines
  (`"Jane SyntheticSenior Platform Engineer"`). Structure must be reconstructed from the
  extracted text using **heuristic section/segment cues**, never assumed newlines.
- **DOCX preserves paragraph structure.** Walking `w:p` → `w:t` gives one clean string per
  paragraph — strictly richer than the PDF path. DOCX is the higher-fidelity input.
- `quick-xml 0.40` API drift: text is decoded via `BytesText::decode()` (not `unescape()`).

## Decision (chosen approach)

| Concern | Choice | Why |
|---|---|---|
| PDF text extraction | **`pdf-extract` 0.10** | Builds in-mirror (lopdf 0.38 + time 0.3.48 ≤ ceiling); pure-Rust; deterministic; one function `extract_text`. No headless browser, no system dep beyond what slice 1 already ships. |
| DOCX text extraction | **`zip` + `quick-xml`** (NOT `docx-rs` for reading) | We only need text out of `word/document.xml`. `zip`+`quick-xml` is a tiny, auditable, deterministic surface; avoids taking `docx-rs`'s full doc model + `image`/`time` weight on the *runtime* path. |
| DOCX fixture authoring (tests only) | **`docx-rs` 0.4**, behind `[dev-dependencies]` | Lets tests synthesise messy-but-synthetic DOCX from personas deterministically, without committing any binary fixture. Not on the shipped path. |
| Text → master-CV mapping | **hand-written deterministic segmenter** in `crates/cvimport` | The product value is honest, deterministic structure. Heuristics: header block → `person`/`headline`; a labelled "Skills/Technologies" segment → skill lists; experience blocks (title @ company · dates) → `experience[]` with bullet lines → `achievementsTasks[]`. Every produced node gets a synthetic stable `id` (`imp_exp_0`, `imp_exp_0_b1`, …). |
| Output validation | reuse **`crates/core::MasterCv::from_json` (parse-don't-validate)** + the existing **`tools/fake-data/validate.js`** schema check at the boundary (L3) | One source of truth for "is this a valid master CV". The importer emits a `MasterCv` and serialises it; the boundary test asserts it validates against `master-cv.schema.json`. |

### Architecture placement (one-way graph, preserved)

```
crates/core  (MasterCv types — unchanged, depended-upon)
      ▲
crates/cvimport  (NEW: pdf-extract, zip, quick-xml; dev-dep docx-rs)
      ▲
apps/desktop/src-tauri (aa-desktop)  → new `import_resume(bytes, kind)` command
      ▲
apps/desktop/src (React)  → onboarding gets a second import option alongside JSON
```

`cvimport` depends on `core` only (for `MasterCv`/`Person`/`Experience`/`Achievement` and
serialisation). It MUST NOT depend on `jobparse`, `aa-desktop`, or the render path.

### Invariant held

Per I1 + the constraint: imported data produces a **new** master-CV document for the user to
review; it never mutates an existing loaded master CV. The Tauri command returns the parsed
`MasterCv` JSON to the UI for confirmation; only an explicit user import action installs it as
the session master CV (reusing the existing `import_master_cv` validation path).

## Residual risks

- **R3a (accepted, slice-scoped):** real-world résumé layouts (multi-column, tables, graphics)
  defeat flat-text heuristics. This slice's acceptance bar is the **synthetic personas rendered
  to PDF/DOCX**, mirroring R6's posture for JD parsing. Robustness on arbitrary real résumés is
  out of scope and is the natural home for the later LLM layer (item #3, evidence-bounded).
- **R3b:** `pdf-extract` line-joins. Mitigated by segmenting on cue tokens rather than newlines,
  and proven by the round-trip tests asserting recovered name/titles/skills/experience.
