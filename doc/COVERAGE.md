# Coverage policy — slice 1

**Floor: 100% of reachable code.** Measured with `cargo llvm-cov --workspace --all-targets`.
Line coverage of all *reachable* statements is **100%**. The residual sub-line *region*
misses (raw line metric reads 99.35%) are enumerated below, each a justified pragma per the
FOUNDRY test contract ("the only path below the floor is an explicit pragma with a stated
reason"). They fall into three classes, all **unreachable on any valid input**:

## P-COV-1 — Infallible serde-serialize error arms
`MasterCv::to_json`, `NormalizedJob::to_json` (core `types.rs:142`, `job.rs:39`),
`render_cover_letter` serialize (`render.rs:146`), and the desktop `seam` serialize/deserialize
(`lib.rs:49,50`). `serde_json::to_string` of an in-memory struct whose fields are only
`String`/`Vec`/`Option`/number **cannot fail** (no maps with non-string keys, no custom
`Serialize` that errors). The `.map_err(...)` closure is dead by construction. Kept because the
methods expose a `Result` API for forward-compatibility (a future field could be fallible).

## P-COV-2 — Defensive filesystem error closures
`CliRenderer::compile` write/read error arms and `repo_root`'s `canonicalize` fallback
(`render.rs:70,108,123,134`). These fire only on transient OS I/O failures (disk full,
permission race) that cannot be triggered deterministically offline without root-level fault
injection. The *spawn-failure* and *typst-compile-failure* arms ARE covered
(`cli_renderer_reports_typst_compile_failure`, `cli_renderer_errors_when_root_missing`).

## P-COV-3 — Feature-gated §H embedded renderer
The `embedded` module in `render.rs` (behind `--features embedded-typst`) is **not compiled**
under default features and so is not in the default coverage set. It is deferred pending
DISCUSS-RENDER (the `time 0.3.48` ↔ typst coherence blocker). When the feature compiles, its
tests run under the same five-level contract.

## What IS covered to 100% (reachable)
Every line of: §A normalize/match, §B coverage, §C fit, §D ranking/summary, §E ledger guard
(incl. the non-vacuous dangling-id test), §F jobparse (all cues + oracle), §H view assembly +
the CLI render path, the seam, and every command happy/unhappy path. All error *values* that a
caller can actually observe (parse failures, NoMasterCv/NoJob, ledger-blocked, render-failed,
typst-compile-failed) are exercised.

---

# Coverage policy — item #2 (`crates/cvimport`, résumé import)

**Same floor: 100% of reachable code** (`cargo llvm-cov --workspace --all-targets`,
`--fail-under-lines 99`). Workspace line coverage with item #2 is **99.32%** (above the floor).
The residual sub-line region misses in `crates/cvimport` are the enumerated pragmas below — each
unreachable on any valid input, mirroring the slice-1 P-COV-1/P-COV-2 classes. Every observable
`ImportError` value (UnsupportedKind, Extract, Decode for a missing `word/document.xml` AND for
ill-formed XML, Empty) IS exercised (L1/L2 tests).

## P-COV-cvimport-1 — Infallible text-decode arm
`extract/docx.rs` `BytesText::decode().map_err(...)`. `Reader::from_str` guarantees valid UTF-8,
so the decode of a `w:t` run cannot fail. The `.map_err` closure is dead by construction (same
class as slice-1 P-COV-1). Kept so the function exposes a total error surface.

## P-COV-cvimport-2 — Defensive zip-read I/O arm
`extract/docx.rs` `entry.take(MAX+1).read_to_end(&mut buf).map_err(...)`. Reading a
successfully-*opened* in-memory zip entry into a buffer fails only on a transient
OS/decompression I/O fault that cannot be triggered deterministically offline (same class as
slice-1 P-COV-2). The read is **bounded** by `MAX_DOCUMENT_XML_BYTES` (32 MiB) against a
decompression bomb (item #2 Finding 2); the oversized-document arm IS covered
(`oversized_document_xml_returns_decode_error_without_oom`), as is the explicit UTF-8
conversion arm (`invalid_utf8_document_xml_returns_decode_error`), the
*missing-`word/document.xml`* arm and the *ill-formed-XML* `read_event` arm
(`valid_zip_missing_document_part_returns_decode_error`, `malformed_document_xml_returns_decode_error`).

## P-COV-cvimport-3 — Unreachable date-range `None` propagation
`segment.rs` `parse_job_line`: `parse_date_range(&range)?`. `range` always begins at a month
token (sliced from the month position), so `parse_date_range`'s empty-start `None` arm is
unreachable from this call site. That `None` arm itself IS covered by a direct unit test
(`parse_date_range_rejects_empty_start`). Kept as `?` for a total, honest contract.

## What IS covered to 100% (reachable) — cvimport
PDF extract (`extract/pdf.rs`), the extract dispatch (`extract/mod.rs`), the whole `import_resume`
pipeline (`lib.rs`), every `map.rs` field mapping + synthetic-id assignment, and the segmenter's
header/skills/experience cue paths incl. the edge arms (blank-skip loops, label-at-EOF, glued PDF
bullets, business-line-absent, bullet-before-job-line). Item #2 review additions now also cover:
the UTF-8-safe case-insensitive month search (`split_on_date_range`/`parse_job_line` over titles
whose lowercase changes byte length — `ẞ`, `İ` — Finding 1, L1 + L2 + L4); the
**separators-only skills line** FALSE arm of `if !skills.is_empty()`
(`skills_label_followed_by_separators_only_pushes_no_bucket`, Finding 4 — no pragma needed); and
**consecutive job lines** not consuming the next role as a business name
(`consecutive_job_lines_do_not_drop_a_block`, Finding 5). The shared perf gate
(`tests/perf_gate.rs`) is unit-tested in `tests/perf_gate_l1.rs` — including a provable 100×
regression failure (non-vacuity). All five test levels L1–L5 pass and each records a perf sample;
the L5 STORYs are perf-delta gated against TRACKED baselines under `doc/perf/`
(`cvimport-import-story-baseline.txt`, `desktop-story-baseline.txt`) — never the gitignored
`target/` (Finding 3).
