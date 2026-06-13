# PR Review — ROADMAP item #10 (DOCX output), branch `item-10-docx-output`

**Reviewer roles:** CORRECTNESS (primary) + REGRESSION + ARCHITECTURE + LICENSING
**Commit reviewed:** `590de6f` — feat(docx): .docx export for CV + cover letter (item 10)
**Verdict: PASS**

Reviewed adversarially: assumed wrong until it failed to break. Ran every gate myself
(fmt, clippy -D warnings, full test suite, coverage with the 99 floor) and built throwaway
probes to attack the determinism claim empirically. All seven item-#10 contracts hold.

---

## Gate results (run, not taken on faith)

| Gate | Command | Result |
|------|---------|--------|
| fmt | `cargo fmt --all --check` | **exit 0** |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0**, no warnings |
| tests | `cargo test --workspace --all-targets` | **exit 0** — 36 test binaries, **0 failed**; docx L1 (10) + docx L2 (12) + CLI docx story (1) all green |
| coverage | `cargo llvm-cov --workspace --all-targets --ignore-filename-regex 'crates/cli/' --summary-only --fail-under-lines 99` | **exit 0** |

Coverage rows:
```
crates/docx/src/lib.rs   Lines 99.34% (2 missed)   Regions 98.91% (6 missed)
TOTAL                    Lines 99.34% (36 missed)
```
The 2 missed lines / residual regions are the documented `pack()` `.map_err` closure
(P-COV-2 class). Floor of 99 is met; exit 0.

---

## Contract verification

**1. Crate graph (architecture) — PASS.** `crates/docx/Cargo.toml`: runtime deps are
`aa-core` (path) + `docx-rs = "0.4"` only; `cargo tree -p aa-core | grep -i docx` is EMPTY;
`crates/core/Cargo.toml` has no docx edge. cvimport's `docx-rs` is a **dev-dep**
(`crates/cvimport/Cargo.toml:25`, under `[dev-dependencies]`) and is **untouched** by this
diff. One-way graph intact.

**2. Shared heading contract (anti-drift) — PASS, attacked hard.**
- All four skill labels in `skill_sections` (`lib.rs:36-43`) — "Languages", "Skills",
  "Tools & Technologies", "Platforms & Services" — and "Experience" are members of
  `heading_vocabulary()` (`render.rs:101-111`). Verified literal-by-literal.
- They match `templates/cv/classic.typ:162-170` `skillBlock(...)` calls **exactly**
  (same strings, same order).
- The `debug_assert!` (lib.rs:107, 122) IS compiled out in release — but the **non-debug**
  `#[test] every_skill_label_is_in_the_heading_vocabulary` (lib.rs:249) and L2
  `every_cv_heading_is_in_the_shared_vocabulary` use plain `assert!` over BOTH templates,
  so a release-build drift fails CI regardless of profile. Drift vector closed.

**3. Sample watermark parity (item 8b) — PASS.**
- `cv_docx`/`cover_letter_docx` emit `aa_core::SAMPLE_WATERMARK` as the FIRST paragraph iff
  `watermark` (lib.rs:86-88, 192-194); L2 (f) pins on⇔present / off⇔absent for both docs.
- CLI shares ONE `decide()` call (main.rs:199) and one `watermark` bool across formats;
  filenames use `cv_filename_ext`/`cover_letter_filename_ext` with the SAME `watermark`
  flag, only the extension differs (main.rs:248,253,262,267) → `cv.SAMPLE.docx` /
  `cover-letter.SAMPLE.docx`. The old `cv_filename(true)=="cv.SAMPLE.pdf"` behaviour is
  preserved (samples.rs:136 + `static_pdf_helpers_agree_with_ext_helpers` test) and the
  pre-existing `filenames_switch_on_sampleness` test survives. No PDF-path change.

**4. Evidence ledger — PASS.** Both `guard()` calls (main.rs:214 CV, 228 cover-letter against
the immutable master) run BEFORE `create_dir_all` (231) and BEFORE the format-branched
render/write (239/258). The docx branch is unreachable if either guard errs (`?` propagates).
Format-independent; docx path does not bypass it. The cover-letter docx writes
`[evidence: <id>]` per strength (lib.rs:205); L2 (e) asserts every id present.

**5. No PII / determinism — PASS, and the determinism claim is HONEST (empirically proven).**
- Fixtures synthetic; persona-001 email is `devin-voss@example.com` (reserved). No
  real-looking emails anywhere in the diff. No committed binaries.
- I attacked the "length-deterministic not byte-deterministic" note as a potential masked
  bug. Built probes:
  - Same-process two calls → **byte-identical**.
  - 4 fresh processes → **identical FNV hash** every run (byte-identical cross-process).
  - 8 concurrent threads in one process → `all_byte_eq=false`, `all_len_eq=true`.
  - Diffed two diverging outputs: the ONLY difference is `w14:paraId="00000002"` vs
    `"00000001"` — a docx-rs **process-global, fixed-width auto-id counter**, exactly as the
    note states. All 8 outputs contain the authored content (name, skills, achievement,
    "Experience"). The length-equality invariant is legitimate (the divergence is upstream
    library numbering, not our content) — **not a cop-out, not masking a content bug.**

**6. Coverage / pragma honesty — PASS.** The `pack()` `.map_err` closure (lib.rs:53-62) over an
in-memory `Cursor<Vec<u8>>` is genuinely unreachable for I/O error (only OOM, which aborts).
Mapping to `CoreError::Render` is correct — it matches the typst render path's error arm and
gives `pack` a total `Result`. Consistent with cvimport's equivalent infallible-IO arm
(`extract/docx.rs:36`, P-COV-cvimport-2), which maps to `ImportError::Decode` — each maps to
its own crate's domain error, which is the right call (docx authoring is a render concern;
docx reading is a decode concern). doc/COVERAGE.md documents it under the same P-COV-2 class,
no new pragma id.

**7. Licensing — PASS.** docx-rs is MIT, noted in `crates/docx/Cargo.toml:7` and the commit
body. Cargo.lock delta: the ONLY new `[[package]]` stanza is `aa-docx` itself; `docx-rs` and
`zip 2.4.2` were already vendored (via cvimport) — confirmed by inspecting the lock diff (the
new lines are dependency *edges*, not new package entries). No new transitive licence concern.

---

## Regression lens
No previously-passing test fails (whole suite green). PDF path is byte-for-byte unchanged
(`--format pdf` is the default; the PDF render/write block at main.rs:239-256 is untouched
logic guarded by `wants_pdf()`). No behaviour change to item 8b / item 9.

---

## Non-gating observations (LOW / SUGGESTION)

- **LOW** `crates/docx/src/lib.rs:18-20, 74-75` — `template` is accepted but Classic and
  Compact emit an identical heading set (DOCX is linear). Documented as deliberate
  ("honoured for API parity"). Honest, but the parameter is currently inert; if it stays
  inert long-term consider a doc-comment `# Note` rather than relying on readers reaching
  line 18. Not a defect.
- **SUGGESTION** The determinism note (module_l2.rs:259-271) is excellent. Consider adding a
  one-line pointer to it from doc/COVERAGE.md or a DISCUSS note so the empirical finding
  isn't only discoverable inside a test file.
- **SUGGESTION** `document_text`/`document_xml_text` tag-stripping helper is duplicated
  between `lib.rs` (inline tests) and `tests/module_l2.rs`. Intentional (keeps private-arm
  tests inline), noted in a comment. Fine; a shared test-support module would remove the
  copy if it grows.

No CRITICAL, HIGH, or MEDIUM findings. Nothing to revise.
