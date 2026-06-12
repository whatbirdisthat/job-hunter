# ROADMAP — job-hunter (Applicant Advocate)

Seeded from the IDEA package at `doc/idea/applicant-advocate/`. FOUNDRY ingests this; items are pulled
through the spec → test → implement → story conveyor. Build top-down.

## DONE
### 1. First slice — JD → tailored CV + cover-letter PDF (full Tauri vertical) ✅ PR #1 merged
The thin end-to-end vertical: import master-CV JSON → paste JD → deterministic fit/coverage →
select & reorder evidence (review UI) → render tailored CV PDF + templated cover letter → export.
Rust core + jobparse + Tauri/React UI; §A–H algorithms; evidence-ledger guard. CliRenderer behind a
seam (R7). 100%-of-reachable coverage; acceptance green on synthetic fixtures.

## TODO (being built in order, one PR per item)

### 2. PDF/DOCX résumé import → master-CV schema · PRIORITY: NOW
Parse an existing résumé (PDF/DOCX) into the canonical master-CV schema (deterministic; run a
parsing-strategy spike first; R3). Validate output against `master-cv.schema.json`. No LLM.

### 3. Applicant Advocate LLM layer
Optional, feature-flagged, evidence-bounded rewrite/draft over local Ollama or a user-supplied key.
Redact before any call; never invent; cite evidence ids; fully disablable. Tests use a stub adapter
(no live model in CI).

### 4. Capture extension (MV3) + email saved-search ingestion
"Clip this job" browser extension (DOM → Normalized Job JSON) + saved-search email parser. Compliant
capture only — no automated login or scraping.

### 5. Application tracker / CRM
Application lifecycle, follow-up scheduler, daily call sheet, recruiter/contact CRM. On-device.

### 6. More CV templates + ATS-readability + keyword-coverage panels
Additional Typst templates; ATS-readability checker; keyword-coverage visibility panel (not stuffing).

## Constraints (apply to every item)
- No PII in the repo; synthetic data only in tests/CI (PII firewall).
- Master CV is immutable; every output claim traces to an evidence id.
- Compliant capture only — never automate LinkedIn/Seek login or scraping.
- Deterministic-first; LLM features optional, flagged, and redacted before any cloud call.
