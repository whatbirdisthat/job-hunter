# IDEA brief — Applicant Advocate

- **TITLE:** Applicant Advocate (product/repo: `job-hunter`)
- **SLUG:** `applicant-advocate`
- **DATE:** 2026-06-12

## PROBLEM
AU mid-career tech/knowledge workers applying through **Seek** and **LinkedIn AU** must hand-tailor a
CV and cover letter for every role — hours per application — or send generic ones that get filtered
out. Existing tools are **cloud SaaS** that (a) take sensitive career data off-device and (b) optimise
for **ATS keyword-gaming** rather than honest fit. No local-first, evidence-grounded tool tailors
*honestly* and produces a *polished* result.

## ACTORS
- **Primary:** an **AU mid-career tech/knowledge worker** (developer, PM, designer, consultant)
  job-hunting on Seek + LinkedIn AU, comfortable running a desktop app.
- **Secondary (later phases):** career-changers; high-volume applicants.

## IN-SCOPE (v1 — the first slice, a thin *full Tauri vertical*)
- Tauri desktop app: **Rust core + React/TS UI**, encrypted local store (SQLite + SQLCipher).
- Import a **master-CV JSON** (canonical schema `doc/schemas/master-cv.schema.json`).
- Paste a **single JD** → deterministic parse/normalize → Normalized Job JSON.
- **Deterministic fit-scoring** + coverage/gap report + evidence map.
- **Tailored CV** assembled by reorder/select (never invent) → rendered to PDF via **embedded Typst**
  (`templates/cv/classic.typ`).
- **Templated (deterministic) draft cover letter.**
- Human review (approve/reject bullets) → **export PDF**.

## OUT-OF-SCOPE (v1)
- PDF/DOCX résumé import (JSON import only for slice 1).
- The LLM **Applicant Advocate** rewrite layer (Ollama / BYO-key) — flagged, a later slice.
- Browser **capture extension** + email ingestion (paste only for slice 1).
- Application **tracker / CRM / follow-up call sheet** (phase 2).
- Any LinkedIn/Seek **automated login or scraping** — never.

## CONSTRAINTS
- **Platform:** cross-platform desktop (Tauri); Rust core, React/TS frontend.
- **Privacy:** no PII off-device; no PII in the public repo; redact before any future LLM call.
- **Compliance:** compliant capture only (no auto-login/scrape/CAPTCHA bypass); no fabrication or
  ATS keyword-stuffing.
- **Performance:** JD → tailored CV PDF + cover letter in **< 60 s**, fully offline, on a typical laptop.
- **Rendering:** deterministic **Typst** (embedded crate), reuse the classic template.
- **Data integrity:** master CV is the **immutable source-of-truth**; every output claim traces to an
  evidence id.

## SUCCESS-METRIC (testable)
Given a master-CV JSON + a pasted JD, the app produces a tailored CV PDF + draft cover letter in
**< 60 s, fully offline**, where:
1. **100% of rendered bullets map to an evidence id** present in the master CV (0 fabricated claims —
   automatable check), and
2. a **coverage report** lists must-have / nice-to-have requirements with covered/uncovered status.

**Acceptance:** a synthetic persona + synthetic JD fixture yields a valid PDF and a coverage report;
an injected unsupported claim is **blocked/flagged** by the evidence guard.

## PRICE-BAND
**$0 — free, open-source, local-first.** No revenue model (mission/portfolio motive). Value accrues to
the user: hours saved + privacy + honest output. Monetisation is explicitly out of scope (optional
future donations / hosted-convenience are *not* in this idea).

## LANGUAGE / STACK
**Rust** core (FOUNDRY `handler-rust`) + **React/TS** via **Tauri** (`handler-react`) + **embedded
Typst**; SQLite + SQLCipher. PDF assets verified locally.

## WILD-CARD
The **evidence ledger** — every line of output is click-traceable to the master-CV bullet *and* the JD
requirement it answers — is simultaneously the integrity mechanism and the **headline brand wedge**
("honest, anti-gaming"). Ship it as a *visible feature*, not just an internal guard.

## WEDGE (why this / now / you)
Lead with **honest, anti-gaming integrity**: a tool that refuses to fabricate or keyword-stuff, keeps
your data on your machine, and proves every claim. Differentiated from cloud ATS-optimisers (Teal,
Rezi, Jobscan, Kickresume) and thin OSS scripts by the **combination**: deterministic fit-scoring +
evidence ledger + typeset PDF quality + Seek/AU focus + a path to a full job-application OS.
