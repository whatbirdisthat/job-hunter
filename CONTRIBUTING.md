# Contributing

Thanks for helping build the Applicant Advocate. The single most important rule first:

## 🔒 Never commit PII

This is a public repository. **Do not commit real CV data, real names, real contact details,
credentials, local databases, or screenshots containing any of the above.**

- Generate test data with `node tools/fake-data/generate.js`.
- The only CV-shaped data allowed in git lives under `fixtures/` and must be 100% synthetic.
- The `.gitignore` PII firewall is your safety net — do not weaken it.
- Run a PII/secret sweep before committing (SENTINEL `pii-audit` / `secret-scan`, or grep for obvious
  patterns). See `doc/PRIVACY.md`.

## Project shape

- **Local-first desktop** (Tauri: Rust shell + React/TS frontend), SQLite + SQLCipher.
- **Deterministic-first**: parsing, normalization, scoring, and selection are deterministic and
  testable. LLM features are optional, feature-flagged, and evidence-bounded.
- **Master CV is immutable** — tailoring produces views, never mutations.
- See `doc/ARCHITECTURE.md` for the layers and the v1 vertical slice.

## Working with the foundation

```bash
node tools/fake-data/generate.js --seed 42 --personas 4 --jobs 6   # synthetic fixtures
node tools/fake-data/validate.js fixtures/personas/*.cv.json        # schema invariants
typst compile templates/cv/classic.typ out.pdf \
  --input data=fixtures/personas/persona-001.cv.json --root .       # render a CV PDF
```

## Principles

- Honest applications only: never fabricate experience, metrics, or tenure.
- Compliant acquisition only: no automated logins, scraping, anti-bot evasion, or CAPTCHA bypass.
- Every generated claim must trace to an evidence id in the master CV.
