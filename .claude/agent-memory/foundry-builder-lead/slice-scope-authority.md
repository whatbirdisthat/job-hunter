---
name: slice-scope-authority
description: Which doc wins on slice scope when ARCHITECTURE.md and the IDEA package disagree
metadata:
  type: project
---

When `doc/ARCHITECTURE.md` and the IDEA package (`doc/idea/applicant-advocate/`) disagree on **slice
scope**, the **IDEA package supersedes** (per handoff.md). Concretely: ARCHITECTURE.md lists "JSON
**or** PDF/DOCX import" in v1, but **slice 1 is JSON import only** — PDF/DOCX is later item R3.

**Why:** the IDEA package is the refined, build-ready handoff; ARCHITECTURE.md is the older foundation
doc it overrides.

**How to apply:** when planning, treat `doc/idea/applicant-advocate/first-slice.md` §A–H as the binding
contract and ARCHITECTURE.md as background. Slice-1 out-of-scope (never build in this cycle): PDF/DOCX
import, the LLM Applicant Advocate layer, the capture extension, the tracker/CRM, and any LinkedIn/Seek
automation. See [[crate-dependency-rule]].
