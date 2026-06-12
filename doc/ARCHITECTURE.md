# Architecture

job-hunter is a **local-first desktop application**. Everything — the master CV, imported jobs,
generated documents, and the encrypted database — stays on the user's device. There is no backend
service that receives personal data.

The long-form product brief is `001-design-brief.md`; this document records the **decisions that
override it** and the concrete v1 plan.

## Decisions (override the brief where they differ)

| Area | Decision |
|------|----------|
| Shape | **Local-first desktop (Tauri)** — Rust shell + web frontend (React/TS), **SQLite + SQLCipher** encrypted at rest. |
| Acquisition | **Compliant capture only** — MV3 "clip this job" extension · paste-a-JD · saved-search email parsing. **No automated login/scraping.** |
| PDF | **Typst templates** (`templates/cv/`), deterministic, no headless browser. The DW_CV CSS is design-reference only. |
| AI | **Deterministic-first.** The "Applicant Advocate" is an *optional*, feature-flagged rewrite/draft layer over **local Ollama** or a user-supplied API key, bounded strictly to CV evidence. |
| Data | Master CV is the **immutable source-of-truth**; tailoring produces views, never mutations. |

## Layers

1. **Acquisition** — extension DOM clip · paste · email parser → raw job text. No login automation.
2. **Parsing / Normalization** — deterministic extraction → **Normalized Job JSON** (title, company,
   location, salary, responsibilities, must/nice requirements, tools, keywords).
3. **CV Knowledge** — master CV JSON (`doc/schemas/master-cv.schema.json`) + evidence graph +
   claim→evidence mapping. Every achievement and skill carries a stable `id`.
4. **Deterministic Matching Engine** — requirement classification, skill normalization (taxonomy /
   synonym map), evidence matching, fit scoring (brief's weighted formula), gap analysis, bullet
   ranking, section selection.
5. **Document Generation** — Typst-rendered tailored CV + templated cover letter. Optional Applicant
   Advocate rewrite (evidence-bounded; cites evidence ids internally; never invents).
6. **Review UI** — requirement↔evidence comparison, approve/reject each bullet, edit drafts, export.
7. **Workflow** (phase 2) — application tracker, follow-up scheduler, daily call sheet, recruiter CRM.
8. **Storage** — SQLite + SQLCipher, on-device only.

## v1 vertical slice — **JD → tailored CV + cover-letter PDF**

The first shippable slice proves the core value loop end to end:

1. Onboard master CV — **import JSON** and validate. (PDF/DOCX résumé parsing is a *later* slice —
   see `doc/idea/applicant-advocate/`, which supersedes this list for slice scope.)
2. Ingest one JD — paste or extension clip → Normalized Job JSON.
3. Deterministic fit-score + coverage report + evidence map.
4. Select/reorder evidence — reorder only, never invent; surface metric-rich bullets; strongest first.
5. Render tailored CV PDF via Typst + draft cover letter (template first; optional Advocate rewrite).
6. Human review → export. (Application tracking/submission is out of v1 scope.)

## Guardrails (non-negotiable)

- Evidence-only generation; unsupported-claim blocker; full audit trail (evidence ledger).
- No autonomous submission, no anti-bot/CAPTCHA bypass, no ATS keyword stuffing, no fabrication.
- All LLM features feature-flagged and fully disablable; redact before any cloud call.

## Testing

- Unit: parser rules, skill normalization, fit scoring, bullet ranking, schema validation, Typst render.
- Fixture-based: synthetic personas + ≥10 synthetic job ads from `tools/fake-data`. **No real data in CI.**

## Render path — DISCUSS-RENDER resolution (accepted risk R7, reversible)

The first-slice contract (§H) specifies **embedded Typst, no shell-out** (a "no system dependency"
property). The embedded `typst` crate is **currently uncompilable in our build environment**: it
transitively requires `time ≥ 0.3.49`, but the crate mirror tops out at `time 0.3.48` (verified).
Resolution: the render path sits behind a `Renderer` seam (`crates/core/src/render.rs`):
- **`CliRenderer`** (default today) — invokes the `typst` binary; deterministic; meets the < 60 s budget.
- **`EmbeddedRenderer`** (feature `embedded-typst`) — the §H contract verbatim; compiles unchanged the
  moment a compatible `time` is available in the mirror.

This is an **accepted, fully reversible slice-1 deviation** (option b): flip the feature when the
mirror gains `time ≥ 0.3.49` — zero code change. The only slice-1 cost is a runtime dependency on the
`typst` binary, at odds with §H's bundled intent; behaviour, determinism, and the budget are unaffected.

## Build handoff

This foundation is handed to `/ideator` to refine into a build-ready package, then to FOUNDRY to
scaffold `apps/desktop` (Tauri), `packages/` (the engine), and `extension/` (MV3 capture).
