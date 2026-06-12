# fake-data — synthetic data generator

Deterministic, zero-dependency generator of **100% fake** data so this public repo can be built and
tested with **no PII**. This is the testing backbone: all fixtures, demos, and CI use its output.

Everything it emits is invented — names are drawn from synthetic pools, emails use the reserved
`example.com` domain, and all contact `name`/`email` reference fields are left empty.

## Usage

```bash
node tools/fake-data/generate.js [--seed 42] [--personas 4] [--jobs 6] [--out fixtures]
```

- **Deterministic:** the same `--seed` produces byte-identical output (PRNG: mulberry32). This keeps
  fixtures stable in version control and CI.
- **Personas** are written to `fixtures/personas/persona-NNN.cv.json`, conforming to
  `doc/schemas/master-cv.schema.json`. Archetypes cycle through: backend-engineer, frontend-engineer,
  product-manager, career-changer.
- **Jobs** are written to `fixtures/jobs/job-<source>-NNN.json` in the Normalized Job JSON shape from
  the design brief, alternating LinkedIn- and Seek-style sources.
- `fixtures/manifest.json` lists what was generated.

## Validation

```bash
node tools/fake-data/validate.js fixtures/personas/*.cv.json
```

A focused, zero-dependency structural validator for the master-CV invariants (required fields,
proficiency 1–5, evidence ids, employment-type enum). A full JSON-Schema (ajv) pass can be added by
FOUNDRY when the engine package is scaffolded.

## Why it tunes to realism without copying anything

The persona archetypes and job vocabulary were shaped to *structurally* resemble a real CV (skill
categories, dates as `MMM YYYY`, emphasised first bullet, monospace micro-achievements) so the
generated documents exercise the template and engine realistically — **without** importing any real
person's data.
