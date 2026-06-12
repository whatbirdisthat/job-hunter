# SMU-seed — Applicant Advocate

A subject-matter-understanding seed for FOUNDRY's builder-lead to expand into the full SMU.

## What the product is
A **local-first, privacy-absolute desktop app** that turns one canonical master CV into an **honest,
tailored CV + draft cover letter** for a specific job — every claim traceable to evidence the user
actually wrote. It cuts through application noise without gaming ATS systems.

## Who it's for
AU mid-career tech/knowledge workers job-hunting on Seek + LinkedIn AU (primary). Later:
career-changers, high-volume applicants.

## The problem (domain framing)
Tailoring per-application is slow and most people do it badly or generically; cloud tools exfiltrate
sensitive career data and optimise for keyword-gaming. The gap: honest, local, polished tailoring.

## Core domain concepts / terms
- **Master CV** — the immutable, canonical source-of-truth (`doc/schemas/master-cv.schema.json`).
- **Evidence** — an id'd skill / experience / achievement bullet; the atom of traceability.
- **Normalized Job** — a parsed JD: title, company, location, salary, responsibilities, requirements.
- **Requirement** — a job need classified **must-have** vs **nice-to-have**.
- **Fit Score / Coverage** — deterministic match of requirements to evidence; coverage + gaps.
- **Evidence Ledger / Map** — the mapping from each output line → master-CV evidence id + the JD
  requirement it answers. The integrity backbone *and* the brand.
- **Tailored View** — a job-specific selection/ordering over the master CV (a view, never a mutation).
- **Applicant Advocate** — the *optional, later* LLM layer (Ollama / BYO-key) that rewrites within
  evidence bounds; never invents.

## Design values (tie-breakers, in order)
1. **Privacy > convenience** — on-device always; no PII off the machine or in the repo.
2. **Honesty/traceability > polish** — never fabricate; every claim proven.
3. **Deterministic > LLM** — deterministic engine is the product; LLM is an optional accelerant.
4. **Local > cloud** — no backend that receives personal data.

## Hard constraints
Cross-platform Tauri (Rust core + React/TS). Compliant capture only (no auto-login/scrape). < 60 s,
fully offline, per tailoring. Embedded Typst rendering. Master CV immutable; output claims traceable.

## What success / failure looks like
- **Success:** from JSON master CV + pasted JD, a valid tailored CV PDF + cover letter in < 60 s
  offline, 0 fabricated claims, a coverage report — and the user trusts it enough to send.
- **Failure:** any claim not backed by evidence reaches the output; data leaves the device; output is
  slow, ugly, or generic enough that the user rewrites it by hand anyway.
