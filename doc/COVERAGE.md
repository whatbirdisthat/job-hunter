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
