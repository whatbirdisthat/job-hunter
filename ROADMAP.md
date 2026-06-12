# ROADMAP — job-hunter (Applicant Advocate)

Seeded from the IDEA package at `doc/idea/applicant-advocate/`. FOUNDRY ingests this; items are pulled
through the spec → test → implement → story conveyor. Build top-down.

## IN PROGRESS
_(none yet — run `/foundry` to pull the first item)_

## TODO

### 1. First slice — JD → tailored CV + cover-letter PDF (full Tauri vertical) · PRIORITY: NOW
The thin end-to-end vertical. Import master-CV JSON → paste a JD → deterministic fit/coverage →
select & reorder evidence (review UI) → render tailored CV PDF (embedded Typst) + templated cover
letter → export. **Stack:** Rust core (`handler-rust`) + React/TS Tauri UI (`handler-react`).
**Spec:** `doc/idea/applicant-advocate/first-slice.md`. **Acceptance:** `< 60 s` offline; 0 fabricated
claims (evidence-ledger guard); coverage report; passes on synthetic fixtures.
- Out of scope here: PDF/DOCX import, LLM layer, capture extension, tracking, any platform automation.

## LATER (phase order indicative — not for this cycle)
2. **PDF/DOCX résumé import** → master-CV schema (run a parsing spike first; R3).
3. **Applicant Advocate LLM layer** — optional, feature-flagged, evidence-bounded (Ollama / BYO-key).
4. **Capture extension** (MV3 "clip this job") + email saved-search ingestion.
5. **Application tracker / CRM** — lifecycle, follow-up scheduler, daily call sheet.
6. **More CV templates** + ATS-readability + keyword-coverage panels.

## Constraints (apply to every item)
- No PII in the repo; synthetic data only in tests/CI (PII firewall).
- Master CV is immutable; every output claim traces to an evidence id.
- Compliant capture only — never automate LinkedIn/Seek login or scraping.
- Deterministic-first; LLM features optional, flagged, and redacted before any cloud call.
