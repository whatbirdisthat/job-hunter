# ROADMAP — job-hunter (Applicant Advocate)

Seeded from the IDEA package at `doc/idea/applicant-advocate/`. FOUNDRY ingests this; items are pulled
through the spec → test → implement → story conveyor. Build top-down.

## DONE
### 1. First slice — JD → tailored CV + cover-letter PDF (full Tauri vertical) ✅ PR #1 merged
The thin end-to-end vertical: import master-CV JSON → paste JD → deterministic fit/coverage →
select & reorder evidence (review UI) → render tailored CV PDF + templated cover letter → export.
Rust core + jobparse + Tauri/React UI; §A–H algorithms; evidence-ledger guard. CliRenderer behind a
seam (R7). 100%-of-reachable coverage; acceptance green on synthetic fixtures.

### 2. PDF/DOCX résumé import → master-CV schema ✅ (item-2-resume-import — PR open, awaiting merge)
Deterministic onboarding path R3: parse an existing PDF/DOCX résumé into a NEW canonical master-CV
document the user reviews (never mutates a loaded one; I1). New crate `crates/cvimport` (depends on
`crates/core` only): PDF via `pdf-extract`, DOCX via `zip`+`quick-xml` (read) / `docx-rs` (dev-only
synthetic fixtures), hand-written deterministic cue-token segmenter → person/skills/experience+
achievements with synthetic stable ids. Output validated against `master-cv.schema.json`. Wired as a
Tauri `import_resume` command + a second onboarding import option in the React UI. No LLM. Spike:
`doc/idea/applicant-advocate/spike-resume-import.md`. EARS R-CVI-1..10; L1–L5 + perf-delta gate;
100%-of-reachable coverage (P-COV-cvimport-1/2/3); PII-free synthetic fixtures only. Adversarial
review PASS after one BLOCK round (UTF-8 panic + DOCX decompression-bomb cap + non-vacuous perf gate).

## TODO (being built in order, one PR per item)

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
