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
| AI | **Deterministic-first.** The "Applicant Advocate" is an *optional*, feature-flagged rewrite/draft layer over **local Ollama** or a user-supplied API key, bound to a cited CV-evidence id. **Realized in item #3** (`crates/advocate`, `aa-advocate`, depends on `aa-core` only): the outbound `RewriteRequest` type physically cannot carry `Person` PII (structural redaction — its JSON keys are exactly `{evidence_id, evidence_text, requirement, kind}`); the rewrite cites an evidence id and the EXISTING ledger `guard` (against the immutable master CV) NAMES + BLOCKS any cited id that resolves NOWHERE. Precisely: the **EVIDENCE-ID is guarded by construction** (a dangling/absent id is blocked — proven by the stub adversarial test), and the stub/CI path is fully guarded; **TEXT-FAITHFULNESS for live models is a documented residual** (R-ADV-RES-1: the live adapters stamp the requested id back verbatim per R-ADV-RES-2, so the id-guard cannot catch a hallucinated rewrite under a valid id — to be closed when the live adapters are activated; see `doc/spec/item-3-advocate-llm.md` "Residual risks"). Default OFF; default cargo features compile the trait + a deterministic `StubProvider` with **no network dependency**, so the `--workspace` CI gate carries no network code — the live Ollama/BYO-key HTTP adapters sit behind `--features live-http` (BYO-key endpoint is `https://`-only with a `rustls` TLS backend; the api_key is redacted in `Debug`). Flag-ON-but-unreachable surfaces an explicit error, never a silent fallback. |
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
