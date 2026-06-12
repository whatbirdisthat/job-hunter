# packages/

The deterministic core engine, scaffolded by `/ideator` + FOUNDRY. Planned packages (names indicative):

- `parse/` — job-ad parsing & normalization → Normalized Job JSON.
- `cv/` — master-CV loading, evidence graph, schema validation, PDF/DOCX résumé import.
- `match/` — requirement classification, skill normalization, fit scoring, gap analysis, bullet ranking.
- `generate/` — tailored-CV assembly (Typst), cover-letter templating, optional Applicant Advocate adapter.

All logic here is deterministic-first and unit-tested against `fixtures/`. See `doc/ARCHITECTURE.md`.

This directory is a placeholder until the build phase begins.
