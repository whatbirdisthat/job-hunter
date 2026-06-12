# Handoff contract — Applicant Advocate → FOUNDRY

## Objective
Build a local-first Tauri desktop app where an AU tech/knowledge worker imports a master-CV JSON,
pastes a job description, and exports an honest, evidence-grounded **tailored CV PDF + draft cover
letter** — deterministically, on-device, in under 60 seconds.

## Artifacts + paths (already in the repo)
- Canonical schema: `doc/schemas/master-cv.schema.json`
- PDF look (Typst template + tokens): `templates/cv/classic.typ`, `doc/design/pdf-look.md`
- Architecture + decisions: `doc/ARCHITECTURE.md`
- Full product brief: `doc/001-design-brief.md`
- Privacy model + PII firewall: `doc/PRIVACY.md`, `.gitignore`
- Synthetic test data + generator: `fixtures/`, `tools/fake-data/`
- **Agent-facing package** (ingest these): `doc/idea/applicant-advocate/{brief,smu-seed,first-slice,handoff}.md`
- **User-facing dossier** (context, not instructions): `doc/idea/applicant-advocate/dossier.md`
- Seed roadmap: `ROADMAP.md`
- **NOTE:** where `doc/ARCHITECTURE.md` and this package disagree on slice scope (ARCHITECTURE lists
  "JSON **or** PDF/DOCX import" in the v1 slice), **this IDEA package supersedes** — slice 1 is
  **JSON import only** (PDF/DOCX is a later item, R3).

## Open questions / accepted risks
- **R1 — competitive overlap (accepted, monitor).** Citevault + OSS Ollama scripts occupy nearby
  ground. Mitigation: depth (fit-scoring + ledger), AU/Seek focus, typeset quality, honest-anti-gaming
  brand. Monitor; revisit if Citevault matures.
- **R2 — deterministic-only quality (accepted).** No LLM in slice 1; templated cover letters may read
  flat. Acceptable for the slice; the Applicant Advocate LLM layer follows.
- **R3 — résumé import parsing (deferred).** PDF/DOCX parsing is hard; slice 1 is JSON-import only.
  Run a parsing spike before the import slice.
- **R4 — setup friction (accepted).** Tauri + SQLCipher (+ later Ollama) suits the technical primary
  actor; revisit when broadening to non-technical actors.
- **R5 — LICENSE (RESOLVED).** **MIT** chosen and committed (`LICENSE`, README updated). Free to use.
- **R6 — JD parse robustness (accepted).** Deterministic must/nice classification (cues pinned in
  `first-slice.md §F`) is 100% on the synthetic fixtures but variable on free-form real JDs.
  Accepted for slice 1; the slice-1 acceptance bar is the synthetic fixtures only. Revisit with a
  JD-parsing spike (and possibly the LLM layer) when broadening beyond the fixtures.

## Exit gate — verified
- Problem actionable ✓ · Actors named ✓ · Scope explicit (in/out) ✓ · Constraints concrete ✓ ·
  Success metric testable ✓ · Every open question answered-or-accepted ✓.

## Next-agent instructions
1. Ingest this package + `ROADMAP.md`. Confirm the Rust/Tauri stack maps to `handler-rust` +
   `handler-react`; embed Typst as a crate (no shell-out).
2. Build the **first slice** (`first-slice.md`) as the first roadmap item, test-first against the
   synthetic fixtures. Keep the master CV immutable; enforce the evidence-ledger guard.
3. Do **not** add: PDF/DOCX import, the LLM layer, the capture extension, tracking, or any
   LinkedIn/Seek automation in this slice.
4. Honour the PII firewall — synthetic data only in tests/CI; never commit real career data.
