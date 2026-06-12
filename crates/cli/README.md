# aa-cli — `applicant-advocate`

The command-line front end to the Applicant Advocate engine. Tailors a CV and
drafts a cover letter (two PDFs) from a master-CV JSON + a plain-text job
description — **fully offline, deterministic, and evidence-guarded** (nothing is
invented; export is blocked if any rendered claim lacks backing in the master CV).

## Usage

```bash
applicant-advocate --cv <master-cv.json> --jd <job.txt> [--out <dir>] [--template classic|compact]
```

- `--cv` — master CV JSON (schema: `doc/schemas/master-cv.schema.json`)
- `--jd` — job description as a plain-text file
- `--out` — output directory (default: current dir) → `cv.pdf`, `cover-letter.pdf`
- `--template` — `classic` (two-column, default) or `compact` (single-column, ATS-friendly)

It prints a fit/coverage summary (must-have & nice-to-have coverage, gaps).

## How it stays self-contained

The renderer (`aa-core`) shells out to `typst` and reads templates + fonts. So the
binary alone is not enough — it needs those resources. In a **release bundle**
(built by `scripts/package-cli.sh`) the `typst` binary, `templates/`, and `fonts/`
sit next to the executable; the CLI detects this layout via `std::env::current_exe()`
and points the renderer at them (`AA_TYPST_BIN`, `AA_FONT_PATH`, and a runtime
`CliRenderer` root). In a dev checkout it falls back to the repo-rooted renderer.

## Build a release

```bash
scripts/package-cli.sh 0.1.0   # → dist/applicant-advocate-0.1.0-linux-x86_64.tar.gz (+ .sha256)
```

Produces a statically-linked (musl) binary bundled with `typst`, templates, fonts,
and a synthetic sample — runs on any Linux x86_64 with no Rust/Node/typst installed.
