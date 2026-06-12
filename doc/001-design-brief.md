Yes — you can build this, but **do not optimize for “gaming recruiter AI” or bypassing platform rules**. Optimize for **accurate fit-matching, tailored application materials, compliance, auditability, and human follow-up orchestration**. That will still materially improve outcomes.

Here’s a **practical agent-plan** you can hand to an agentic coding harness.

---

# Goal

Build a system that:

1. Connects to **LinkedIn** and **Seek** job feeds.
2. Ingests job ads / PDs into a normalized schema.
3. Uses a **master CV JSON** as source-of-truth.
4. Produces:
   - a **tailored CV draft**
   - a **targeted cover-letter draft**
   - a **fit analysis**
   - an **application packet**
5. Keeps as much logic as possible **deterministic and client-side**.
6. In phase two, adds:
   - LLM-assisted prioritization
   - recruiter/company follow-up workflows
   - a “daily call sheet”
   - application tracking and reminders

---

# First principles

Design around these constraints:

- **Terms-of-service compliance matters.** LinkedIn especially is sensitive around scraping/automation. Prefer:
  - official exports/APIs where available
  - email feed ingestion
  - browser-assisted user-driven capture
  - saved-search notification parsing
  - manual “clip this job” extension flow
- **Deterministic first, LLM second.**
  - Deterministic extraction, scoring, redaction, keyword mapping, and section selection
  - LLM only for summarization/rewrite/draft generation where needed
- **Source-of-truth CV must stay canonical.**
  - Never mutate the full CV JSON
  - Generate job-specific views
- **Every generated claim must be traceable** to evidence in the master CV JSON.
- **Human-in-the-loop required** before submission.

---

# Product framing

## Core product
A **job application operating system**:
- job intake
- fit scoring
- CV tailoring
- cover letter drafting
- application tracking
- follow-up orchestration

## Non-goals for v1
- Fully autonomous application submission
- CAPTCHA bypass
- anti-bot evasion
- fake experience inflation
- fabricating achievements
- deceptive ATS stuffing

---

# Recommended architecture

## High-level components

1. **Acquisition Layer**
   - browser extension
   - email ingestion
   - RSS / saved-search parsers
   - manual URL import
   - optional Playwright-assisted user session capture

2. **Parsing & Normalization Layer**
   - extract title, company, location, salary, responsibilities, requirements, keywords
   - normalize into a common Job schema

3. **CV Knowledge Layer**
   - canonical master CV JSON
   - evidence graph: skills, projects, roles, achievements, dates, domains
   - claim-to-evidence mapping

4. **Deterministic Matching Engine**
   - requirement extraction
   - keyword clustering
   - must-have/nice-to-have classification
   - coverage scoring
   - gap analysis
   - section ranking for CV inclusion

5. **Document Generation Layer**
   - tailored CV assembler
   - cover letter drafter
   - recruiter outreach draft generator
   - follow-up note generator

6. **Review UI**
   - compare job requirements vs CV evidence
   - approve/reject each included bullet
   - edit generated documents
   - track applications and follow-ups

7. **Workflow / Agent Layer**
   - orchestration tasks
   - daily digest
   - reminders
   - recruiter call sheet creation

8. **Storage Layer**
   - jobs
   - normalized job requirements
   - applications
   - generated documents
   - recruiter contacts
   - audit logs

---

# Recommended tech stack

## If building fast
- **Frontend**: Next.js
- **Desktop/local-first option**: Tauri or Electron
- **Backend/API**: Node/TypeScript
- **DB**: Postgres + pgvector optional
- **Queue/workflows**: Temporal or Inngest
- **Browser automation**: Playwright
- **Extension**: Manifest V3 extension
- **Document generation**:
  - JSON -> Markdown -> DOCX/PDF
  - libraries for DOCX templating
- **Search/indexing**:
  - deterministic text indexing first
  - embeddings optional in phase two

## For local-first/privacy-heavy
- Tauri desktop app
- SQLite + SQLCipher
- local document rendering
- optional local LLM integration later

---

# Compliance-safe acquisition strategy

This is important.

## LinkedIn
Preferred order:
1. **User-driven browser extension capture**
   - user opens job page
   - extension extracts DOM
   - sends normalized content locally
2. **Saved-job / notification email ingestion**
3. **Manual copy/paste import**
4. **Playwright with explicit user session and manual approval gates**

Avoid building the product around unattended scraping.

## Seek
Same pattern:
1. browser extension page capture
2. saved-search email parsing
3. manual URL import / copy-paste
4. cautious browser-assisted collection if needed

## Best v1 approach
Build:
- **Chrome extension**
- **“Import this job” button**
- **Gmail/Outlook forwarding parser**
- **paste-a-job-description fallback**

That gets you 80% of value with much lower operational risk.

---

# Canonical data models

## Master CV JSON
Your master CV should look roughly like:

```json
{
  "person": {
    "name": "",
    "location": "",
    "email": "",
    "phone": "",
    "linkedin": "",
    "website": ""
  },
  "headline": "",
  "summary_variants": [],
  "skills": [
    {
      "name": "Python",
      "aliases": ["Py"],
      "category": "language",
      "proficiency": "advanced",
      "evidence_ids": ["exp_12", "proj_3"]
    }
  ],
  "experience": [
    {
      "id": "exp_12",
      "company": "",
      "title": "",
      "start_date": "",
      "end_date": "",
      "location": "",
      "domain": "",
      "bullets": [
        {
          "id": "exp_12_b1",
          "text": "",
          "tags": ["leadership", "automation", "aws"],
          "metrics": ["30% reduction"],
          "evidence_strength": 0.95
        }
      ]
    }
  ],
  "projects": [],
  "education": [],
  "certifications": [],
  "awards": [],
  "preferences": {
    "role_types": [],
    "industries": [],
    "locations": []
  }
}
```

## Normalized Job JSON
```json
{
  "source": "linkedin",
  "source_url": "",
  "captured_at": "",
  "job_id": "",
  "company": "",
  "title": "",
  "location": "",
  "employment_type": "",
  "work_model": "remote|hybrid|onsite|unknown",
  "salary": {
    "min": null,
    "max": null,
    "currency": "AUD"
  },
  "description_raw": "",
  "description_clean": "",
  "responsibilities": [],
  "requirements": {
    "must_have": [],
    "nice_to_have": [],
    "tools": [],
    "domains": [],
    "seniority_signals": []
  },
  "keywords": [],
  "application_method": "",
  "recruiter": {
    "name": "",
    "title": "",
    "profile_url": ""
  }
}
```

## Tailoring Output JSON
```json
{
  "job_id": "",
  "fit_score": 0.82,
  "coverage": {
    "must_have_covered": 9,
    "must_have_total": 11,
    "nice_to_have_covered": 4,
    "nice_to_have_total": 8
  },
  "matched_evidence": [
    {
      "requirement": "stakeholder management",
      "evidence_ids": ["exp_3_b2", "exp_8_b1"]
    }
  ],
  "gaps": [],
  "selected_cv_sections": [],
  "selected_bullets": [],
  "tailored_summary": "",
  "cover_letter_points": [],
  "risk_flags": []
}
```

---

# Deterministic engine design

This is the heart of the product.

## Deterministic steps

### 1. Job parsing
Extract:
- title
- company
- location
- required skills
- responsibilities
- years-of-experience patterns
- certification requirements
- domain vocabulary

Use:
- CSS selectors for known sources
- fallback readability extraction
- rule-based heading segmentation
- regex/entity extraction

### 2. Requirement classification
Rules to split:
- **must-have**
  - “required”
  - “must have”
  - “essential”
  - “you will need”
- **nice-to-have**
  - “preferred”
  - “bonus”
  - “desirable”
  - “nice to have”

### 3. Skill normalization
Map synonyms:
- JS -> JavaScript
- TS -> TypeScript
- stakeholder engagement -> stakeholder management
- CI/CD -> continuous integration / delivery

Use a maintained taxonomy file.

### 4. Evidence matching
Match each job requirement to:
- skills
- experience bullets
- projects
- certifications

Score by:
- exact skill match
- alias match
- co-occurrence with role title/domain
- recency
- strength of metrics
- seniority alignment

### 5. CV bullet selection
Choose bullets using deterministic ranking:
- relevance to must-haves
- measurable outcomes
- recency
- title/domain alignment
- diversity across competencies

### 6. Summary tailoring
Prefer templated deterministic summary assembly first:
- role title alignment
- years of experience bracket
- domain emphasis
- top 3 matched strengths

### 7. Cover letter drafting
Use structured inputs:
- why this company
- why this role
- 2–3 strongest evidence-backed themes
- specific business value claims grounded in CV evidence

### 8. Risk controls
Flag:
- unverified claims
- repeated keywords with no evidence
- missing must-haves
- stale technologies
- overlong documents
- inconsistent dates/titles

---

# Where LLMs should and should not be used

## Deterministic / local-first
Use deterministic logic for:
- parsing known DOMs
- normalizing skills
- classification of must-have/nice-to-have
- evidence retrieval
- fit scoring
- bullet ranking
- recruiter follow-up scheduling
- reminders and call sheet generation

## LLM optional
Use LLMs for:
- rewriting selected bullets more crisply
- generating cover letter drafts
- summarizing company/job angle
- producing recruiter outreach variants
- suggesting missing evidence to add manually

## Guardrails
LLM output must:
- only use evidence passed in context
- cite source evidence IDs internally
- never invent metrics
- never claim experience absent from master CV

---

# Phased rollout plan

## Phase 0 — Discovery / architecture spike
**Goal:** validate acquisition, schemas, and tailoring quality.

### Deliverables
- source compliance review
- sample imports from LinkedIn and Seek
- canonical CV schema
- normalized job schema
- fit scoring rubric
- UX wireframes

### Tasks
- capture 20 representative job ads
- define selectors / extraction strategies
- create skill taxonomy and synonym map
- define scoring weights
- build 3 sample tailored CV outputs

### Exit criteria
- can ingest jobs from both sources via at least one compliant path
- can generate a tailored CV packet for 10 sample jobs
- quality is acceptable to human reviewer

---

## Phase 1 — Deterministic MVP
**Goal:** build reliable job capture + CV tailoring + draft cover letters.

### User flow
1. user imports a job
2. system parses and normalizes it
3. system scores fit against master CV JSON
4. system generates:
   - tailored CV draft
   - cover letter draft
   - requirement coverage report
5. user edits and exports
6. user marks application submitted

### Features
- browser extension capture
- job inbox
- normalized parsing pipeline
- master CV JSON manager
- deterministic fit scoring
- tailored CV builder
- cover letter draft generator
- export to PDF/DOCX
- application tracker

### Agent prompt for build
Give this to the coding harness:

---

## Agent build brief — Phase 1

**Objective:** Build a local-first web app that ingests job descriptions from LinkedIn and Seek via browser-extension capture or pasted text, normalizes them, scores them against a canonical CV JSON, and outputs a tailored CV draft, evidence map, and cover letter draft.

### Requirements
- Tech stack: Next.js + TypeScript + Postgres or SQLite if local-first
- Build Chrome extension for job page capture
- Define canonical schemas:
  - Master CV JSON
  - Normalized Job JSON
  - Tailoring Result JSON
- Implement deterministic parsers for:
  - job title
  - company
  - location
  - responsibilities
  - must-have / nice-to-have requirements
  - skills / tools / domain terms
- Implement scoring engine:
  - requirement matching
  - evidence selection
  - recency weighting
  - metrics weighting
  - seniority weighting
- Build CV assembler:
  - summary section
  - skill ordering
  - bullet selection and ordering
  - optional role-specific section variants
- Build cover letter generator using templates first, with optional LLM rewrite adapter behind a feature flag
- Build UI:
  - job inbox
  - job detail
  - match analysis
  - document preview
  - export
  - application status tracker
- Keep all tailoring decisions auditable:
  - every selected bullet must link back to evidence IDs in master CV JSON
- Do not implement autonomous application submission
- Do not include anti-bot or CAPTCHA bypass features
- Add unit tests for:
  - parser rules
  - skill normalization
  - fit scoring
  - bullet ranking
- Add fixture-based tests using 10 sample job descriptions

### Output expected
- architecture doc
- schema definitions
- extension scaffold
- MVP application
- tests
- sample fixtures
- README with local setup and privacy model

---

## Phase 2 — Assisted intelligence and workflow ops
**Goal:** add LLM-assisted prioritization and follow-up system.

### Features
- LLM-assisted “why this role” summarization
- recruiter/company research summaries
- follow-up recommendation engine
- “daily call sheet”
- suggested messaging:
  - recruiter follow-up
  - hiring manager outreach
  - post-application nudge
- application aging logic:
  - day 0 submitted
  - day 3–5 follow-up suggestion
  - day 7–10 second follow-up
  - archive / deprioritize rules

### Daily call sheet contents
For each day:
- company
- role
- application date
- recommended follow-up window
- recruiter/contact
- suggested channel
- draft message
- next action
- confidence / priority score

### Agent build brief — Phase 2

**Objective:** Extend the MVP with assisted workflow features that prioritize jobs, track submitted applications, and generate a daily follow-up sheet for recruiters and companies.

### Requirements
- Add application lifecycle model:
  - discovered
  - tailored
  - applied
  - follow-up due
  - interview
  - closed
- Add scheduler for follow-up recommendations
- Add recruiter/contact entity model
- Add daily digest page and exportable “call sheet”
- Implement rules engine for follow-ups:
  - based on submission date
  - based on role priority
  - based on source and response history
- Add optional LLM adapters for:
  - recruiter outreach drafts
  - hiring manager notes
  - role prioritization summaries
  - company-specific talking points
- Add CRM-like notes:
  - contacted
  - replied
  - voicemail left
  - next step
- Add deduplication for repeated job ads across sources
- Add event timeline per application
- Add reminders via email/calendar/web notifications
- Build prompts so that generated outreach stays factual and evidence-based
- Add feature flags so all LLM features can be disabled cleanly

### Output expected
- migration scripts
- workflow engine
- call-sheet UI
- outreach draft generation
- scheduler/reminder system
- updated tests and docs

---

## Phase 3 — Advanced automation
**Goal:** improve throughput without losing trust/compliance.

### Possible features
- semantic job clustering
- “best jobs today” ranking
- company watchlists
- recruiter relationship graph
- network/referral tracker
- interview prep pack generation
- per-role CV strategy memory
- local embeddings for private similarity search
- learned ranking from past outcomes

---

# Suggested scoring model

A simple deterministic formula:

```text
fit_score =
  0.40 * must_have_coverage +
  0.20 * nice_to_have_coverage +
  0.15 * title_alignment +
  0.10 * domain_alignment +
  0.10 * seniority_alignment +
  0.05 * recency_strength
  - penalties
```

Penalties:
- unsupported claims
- missing critical requirement
- location mismatch
- visa/work authorization mismatch if relevant
- salary mismatch if specified

---

# CV tailoring strategy

## CV generation rules
- Keep 1 master CV JSON only.
- Generate job-specific CVs as views.
- Reorder, do not invent.
- Condense low-relevance bullets.
- Surface metric-rich bullets.
- Match terminology used in the job ad when truthful.
- Put strongest aligned evidence on page 1.

## Cover letter strategy
The best letters are usually:
- short
- role-specific
- evidence-backed
- company-aware
- easy to scan

Template:
1. why this role/company
2. 2–3 matched strengths with proof
3. close with relevance and enthusiasm

---

# Extra advantageous features

Yes — there are several high-value features worth adding.

## 1. Evidence ledger
Every line in the tailored CV and cover letter should map back to:
- CV evidence ID
- original bullet
- job requirement matched

This is hugely useful for trust and editing.

## 2. ATS readability checker
Check for:
- tables/columns
- icons
- headers/footers
- unusual fonts
- image-only content
- parse order issues

## 3. Keyword coverage panel
Show:
- must-have keywords found
- missing keywords
- where each keyword appears in CV

Not stuffing — just visibility.

## 4. Gap-aware rewrite suggestions
Instead of faking fit:
- suggest adjacent true experience to emphasize
- suggest portfolio/project proof to add
- suggest one-line clarification to address a gap

## 5. Application de-duplication
Same role may appear:
- on LinkedIn
- on Seek
- on company site
- via recruiter posting

Canonicalize postings.

## 6. Job freshness & urgency score
Rank by:
- posted date
- applicant volume hints
- direct employer vs recruiter
- fit score
- salary clarity
- ease of application

## 7. Company dossier
Lightweight brief:
- what company does
- likely pain points from role
- recent news/manual notes
- talking points for outreach/interviews

## 8. Outreach CRM
Track:
- recruiter names
- outreach attempts
- response outcomes
- referral sources
- next follow-up date

## 9. Interview prep auto-pack
Once status changes to interview:
- role summary
- likely competency themes
- STAR stories from CV evidence
- company-specific questions to ask

## 10. Version/performance analytics
Track:
- which CV variants used
- which cover-letter styles used
- interview conversion by variant
- source quality by platform/search

This becomes your feedback loop.

---

# Important operational risks

## 1. Platform risk
LinkedIn scraping/automation can create account risk.

Mitigation:
- user-driven capture
- emails and manual import
- minimal automation
- no unattended scraping as v1 dependency

## 2. Hallucination risk
Mitigation:
- evidence-only generation
- deterministic first
- audit trail
- explicit unsupported-claim blocker

## 3. Privacy risk
You’re storing highly sensitive career data.

Mitigation:
- local-first where possible
- encryption at rest
- secrets isolation
- redact before LLM calls
- selective field sharing only

## 4. Over-optimization risk
If you overfit to ATS keywords, materials get worse for humans.

Mitigation:
- readable-first
- proof-backed wording
- score both ATS coverage and human clarity

---

# Recommended roadmap by week

## Week 1
- schemas
- sample fixtures
- parser spike
- extension spike
- fit rubric

## Week 2
- ingestion pipeline
- normalization
- master CV manager
- deterministic matcher

## Week 3
- CV assembler
- cover letter templating
- review UI

## Week 4
- export
- tracking
- tests
- end-to-end polish

## Week 5–6
- follow-up scheduler
- recruiter/contact CRM
- call sheet
- optional LLM adapters

---

# What to hand to a coding agent

Use this as the top-level instruction:

---

## Full agentic harness brief

Build a privacy-conscious, local-first job application assistant that helps a user tailor applications from imported job ads. The system must ingest job ads from LinkedIn and Seek through compliant, user-driven acquisition methods such as browser extension capture, email ingestion, manual paste, and optionally browser-assisted import with explicit user interaction.

The system must maintain a canonical master CV JSON as source-of-truth and never mutate it during tailoring. For each imported job, the system must normalize the job description into structured requirements, deterministically match those requirements against evidence in the CV JSON, compute a fit score, select the most relevant bullets and sections, and generate:
1. a tailored CV draft,
2. an evidence map,
3. a short cover letter draft,
4. an application tracking record.

Prioritize deterministic and client-side processing wherever possible. LLM usage must be optional, feature-flagged, and restricted to rewriting/summarization tasks using only evidence explicitly provided in context. The system must never fabricate claims, metrics, tenure, or qualifications.

Phase 1 should deliver ingestion, normalization, deterministic fit scoring, CV tailoring, draft cover letters, export, and application tracking.

Phase 2 should add recruiter/contact tracking, follow-up scheduling, daily call sheet generation, and optional LLM-assisted outreach/message drafting.

Non-goals include autonomous application submission, anti-bot evasion, CAPTCHA bypass, deceptive ATS stuffing, and unsupported claim generation.

Produce:
- architecture documentation
- data schemas
- extension/app code
- tests and fixtures
- privacy and compliance notes
- phased rollout plan in repository docs

---

