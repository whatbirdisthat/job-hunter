Applicant Advocate — CLI (v0.1.0, Linux x86_64)
================================================

Tailor a CV and draft a cover letter (two PDFs) from your master CV + a job
description — fully offline, on your machine. Nothing is uploaded; every claim in
the output is checked against your CV, so nothing is invented.

This bundle is self-contained: it needs NO Rust, NO Node.js, NO typst, and no
system fonts installed. The binary is statically linked and ships its own typst
renderer, templates, and fonts.

QUICK START
-----------
From inside this folder:

    ./applicant-advocate --cv samples/sample-cv.json --jd samples/sample-job.txt --out ./out

That writes:
    out/cv.pdf              the tailored CV
    out/cover-letter.pdf    the draft cover letter

USE YOUR OWN DATA
-----------------
  --cv <file>      Your master CV as JSON (the canonical schema — see the project
                   repo: doc/schemas/master-cv.schema.json). Start from
                   samples/sample-cv.json and replace the contents with your own.
  --jd <file>      The job description, saved as a plain-text file (copy/paste the
                   posting into a .txt).
  --out <dir>      Where to write the PDFs (default: current directory).
  --template <n>   'classic' (two-column, default) or 'compact' (single-column,
                   more ATS-friendly).
  --help           Full usage.

WHAT IT DOES
------------
  1. Reads your master CV (the immutable source of truth).
  2. Parses the job description into structured requirements.
  3. Scores fit and reports must-have / nice-to-have coverage and gaps.
  4. Selects and reorders your strongest matching evidence (never invents).
  5. Renders a tailored CV PDF + a draft cover letter, each line traceable to your
     own CV. Export is blocked if any claim lacks backing evidence.

PRIVACY
-------
Everything runs locally. No network calls, no telemetry, no accounts.

This is free, open-source software (MIT). Source, schema, and issues:
https://github.com/whatbirdisthat/job-hunter
