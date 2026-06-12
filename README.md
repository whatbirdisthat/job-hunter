# job-hunter — the Applicant Advocate

[![CI](https://github.com/whatbirdisthat/job-hunter/actions/workflows/ci.yml/badge.svg)](https://github.com/whatbirdisthat/job-hunter/actions/workflows/ci.yml)

> Cut through the noise. A privacy-first, local-first assistant that turns one master CV into a
> **tailored CV** and a **draft cover letter** for every job you apply to — every claim traceable
> back to evidence you actually wrote.

This is **free, open-source, and local-first by design**. Your career data never leaves your
machine. There is no server to send your résumé to, no account to create, and **no PII in this
repository** — the project is developed and tested entirely against synthetic data.

## Why this exists

Good people get lost in the noise of mass applications and ATS keyword games. The **Applicant
Advocate** does the opposite of gaming the system: it reads a real job ad, scores how well *your real
experience* fits it, and assembles an honest, well-targeted application that puts your strongest
relevant evidence first. It never invents experience, metrics, or tenure.

## How it works (the value loop)

1. **Plug in your CV** — import a master-CV JSON, or drop in an existing PDF/DOCX résumé to be parsed
   into the canonical schema (`doc/schemas/master-cv.schema.json`).
2. **Bring in a job** — paste a job description, clip it with the capture extension, or import a
   saved-search email. (No automated logins or scraping — compliant, account-safe.)
3. **See the fit** — a deterministic engine matches the job's requirements to evidence in your CV and
   reports coverage and gaps.
4. **Get your application** — a tailored CV (rendered to PDF via Typst, reusing the clean classic
   look) plus a draft cover letter. An optional **Applicant Advocate** AI layer (local Ollama or your
   own API key) can sharpen the wording — bounded strictly to your evidence.
5. **Review and export** — nothing is submitted for you; you stay in control.

## Project status

**Foundation stage.** This repo currently contains the canonical data schema, the PDF design as a
Typst template, a synthetic fake-data generator, and the architecture/privacy docs. The application
itself (desktop app, parsing/scoring engine, capture extension) is specified in
[`doc/ARCHITECTURE.md`](doc/ARCHITECTURE.md) and [`doc/001-design-brief.md`](doc/001-design-brief.md)
and is built next.

## Repository layout

| Path | What it is |
|------|------------|
| `doc/001-design-brief.md` | The full product brief |
| `doc/ARCHITECTURE.md` | The local-first architecture + v1 vertical slice |
| `doc/PRIVACY.md` | The privacy model and the no-PII guarantee |
| `doc/schemas/master-cv.schema.json` | Canonical, immutable Master CV schema (source-of-truth) |
| `doc/design/pdf-look.md` | Design tokens for the CV's "finished look" |
| `templates/cv/classic.typ` | The CV rendered to PDF (Typst) |
| `tools/fake-data/` | Deterministic synthetic-persona + job-ad generator |
| `fixtures/` | 100% synthetic test data (the only CV-shaped data allowed in git) |
| `apps/`, `packages/`, `extension/` | Scaffolded next (desktop app, engine, capture extension) |

## Quick start (foundation)

```bash
# Generate synthetic fixtures (deterministic; same seed -> identical output)
node tools/fake-data/generate.js --seed 42 --personas 4 --jobs 6 --out fixtures

# Validate them against the master-CV schema invariants
node tools/fake-data/validate.js fixtures/personas/*.cv.json

# Render a synthetic persona to a PDF using the classic template
typst compile templates/cv/classic.typ out.pdf \
  --input data=fixtures/personas/persona-001.cv.json --root .
```

## Privacy guarantee

- **No PII in this repository, ever.** A `.gitignore` PII firewall blocks real CV data, secrets, and
  local databases; CI runs only on synthetic fixtures.
- **Local-first.** Real career data lives only on your device (and in the app's encrypted local
  store at runtime). See [`doc/PRIVACY.md`](doc/PRIVACY.md).

## License

Intended to be permissive open-source (free to use). Final license is being confirmed — see
[`LICENSE`](LICENSE).
