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

### 3. Applicant Advocate LLM layer ✅ (item-3-advocate-llm — PR open, awaiting merge)
Optional, feature-flagged, evidence-bounded rewrite/draft layer; OFF by default — the deterministic
path (items 1–2) remains the product without it. New crate `crates/advocate` (depends on `crates/core`
only, one-way graph): an `AdvocateProvider` trait with a deterministic `StubProvider` (always compiled,
the CI/test surface) and feature-gated (`live-http`) `OllamaProvider` (loopback `http://localhost:11434`)
+ generic BYO-key `HttpKeyProvider` (TLS via `ureq/rustls`; rejects non-`https://` endpoints; manual
redacting `Debug`). Redaction is STRUCTURAL by type: the `RewriteRequest` carries only
`{evidence_id, evidence_text, requirement, kind}` — no `Person` block can reach the prompt. Output
re-enters the EXISTING §E evidence-ledger guard against the IMMUTABLE master CV: a rewrite citing a
dangling/absent evidence id is BLOCKED at export (non-vacuous adversarial test — stub fabricates →
export blocked & named; honest twin passes). Surfaces: CV-bullet rewrite + cover-letter strength
drafting behind a clear opt-in React toggle (OFF by default) + an "AI was used" badge. EARS R-ADV-1..13;
L1–L5 + STORY perf-delta gate (new tracked baseline); 100%-of-reachable coverage; no live model in CI
(`ureq` excluded from the default/CI build by construction). Adversarial review PASS after one
NEEDS_REVISION round (TLS+scheme guard, Debug redaction, honest faithfulness-limitation disclosure,
free-text-PII residual doc, cover-letter single-rewrite). Documented residuals deferred to the
adapter-wiring slice: R-ADV-RES-1 text-faithfulness for live models, R-ADV-RES-2 cited-id parsing,
R-ADV-RES-3 free-text PII in evidence.

### 4. Capture extension (MV3) + email saved-search ingestion ✅ (item-4-capture-extension — PR open, awaiting merge)
Two compliant, USER-DRIVEN acquisition paths feeding the SAME strict Normalized Job JSON
(`doc/schemas/normalized-job.schema.json`) — the first non-Rust (TypeScript) slice. Zero npm deps.
Deterministic DOM→job and email→job logic lives as PURE `.mjs` cores in `packages/capture-core`
(§F ported VERBATIM from `crates/jobparse`, byte-faithful on all normal input; one documented,
tested unicode divergence-family that favours never-panic/no-corruption over the Rust oracle's
panic/corrupt on length-changing `to_lowercase`). `extension/` holds only thin MV3 wiring.
- **dom-extract core** (`htmlString → NormalizedJob`): zero-dep tolerant HTML→text then §F. The
  content script reads the active tab's `outerHTML` on the user's click and calls the core.
- **email-extract core** (`emlString → NormalizedJob[]`): zero-dep `.eml` (multipart/alternative,
  QP+base64, UTF-8) → HTML→text → §F per posting (deterministic §F-sentence split).
- **Handoff:** downloadable `.json` (the user imports via the existing path; byte-compatible with
  `CoreJob::from_json`, no Rust change). Localhost handoff documented-as-deferred (needs new Rust
  surface + security model — out of scope, see DISCUSS-HANDOFF).
- **COMPLIANCE (non-negotiable):** user-driven capture ONLY. MV3 manifest permissions are exactly
  `[activeTab, scripting, downloads]` — NO `host_permissions`, NO auto-injected `content_scripts`;
  injection is programmatic + gesture-gated. No automated login/navigation/scraping/anti-bot evasion;
  prohibitions stated verbatim in `manifest.json` + `extension/README.md`.
- New zero-dep normalized-job validator (`packages/capture-core/src/validate-job.mjs` +
  `tools/fake-data/validate-job.js` CLI). L1–L5 (perf-delta gated STORY); 100%-of-reachable coverage.
  New BLOCKING CI job `capture-core` runs on `node --test` with NO npm install (honest gate despite
  issue #2); `foundation`/`pii-guard`/`rust-workspace` unchanged. Synthetic PII-free fixtures only.
  EARS R-EXT-* / R-EML-* / R-JOB-*; spec `doc/spec/item-4-capture-extension.md`. Adversarial review
  PASS after two NEEDS_REVISION rounds (two HIGH port-fidelity divergences found by differential
  fuzzing + a fidelity-overclaim corrected). Residuals deferred: real LinkedIn/Seek DOM fidelity
  (DISCUSS-DOM, synthetic-fixtures bar per R6) and localhost handoff (DISCUSS-HANDOFF).

## TODO (being built in order, one PR per item)

### 5. Application tracker / CRM
Application lifecycle, follow-up scheduler, daily call sheet, recruiter/contact CRM. On-device.

### 6. More CV templates + ATS-readability + keyword-coverage panels
Additional Typst templates; ATS-readability checker; keyword-coverage visibility panel (not stuffing).

## Constraints (apply to every item)
- No PII in the repo; synthetic data only in tests/CI (PII firewall).
- Master CV is immutable; every output claim traces to an evidence id.
- Compliant capture only — never automate LinkedIn/Seek login or scraping.
- Deterministic-first; LLM features optional, flagged, and redacted before any cloud call.
