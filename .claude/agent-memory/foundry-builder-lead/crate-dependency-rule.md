---
name: crate-dependency-rule
description: The one-way crate dependency rule for the job-hunter Rust workspace (core/jobparse/desktop)
metadata:
  type: project
---

The job-hunter Rust workspace dependency rule (decided in FOUNDRY_PLAN.md for slice 1):

- `crates/core` MUST NOT depend on `crates/jobparse`.
- `crates/jobparse` MUST NOT depend on `crates/core`.
- They are siblings; **`apps/desktop` is the only crate that depends on both** and wires the seam
  (jobparse output → core input).
- The shared contract is **data, not code**: the master-CV JSON Schema (`doc/schemas/master-cv.schema.json`)
  and the Normalized-Job shape. The **tailored view is a schema-conformant master-CV document** (first-slice
  §H) — never a bespoke object — so `templates/cv/classic.typ` renders it unchanged.

**Why:** keeps the core↔jobparse seam a pure value boundary, prevents engine/parser coupling, and lets
the same Typst template render both CLI fixtures and the embedded tailored view.

**How to apply:** when a task seems to need a cross-crate import between core and jobparse, route it
through the shared schema/value contract instead. OPEN (DISCUSS D1): whether NormalizedJob is a tiny
shared type owned by core, or a separate `normalized-job.schema.json` (recommended) — resolve before
building `crates/jobparse`. See [[handler-gaps]], [[slice-scope-authority]].
