---
name: typst-embedding-blocker
description: Embedded typst-as-lib/typst crate will not compile in this env (time 0.3.48 coherence bug); CLI typst works fine. Render path made swappable.
metadata:
  type: project
---

The §H requirement is "embedded Typst render via the `typst`/`typst-as-lib` crate, NO shell-out in the app path." This is **uncompilable in this environment**.

**Why:** every available `typst-library` (0.13.1 and 0.14.2) transitively requires `time` with the `parsing` feature. The crate mirror (`fleetmirror`) only vendors `time 0.3.48`, whose generated `impl From<HourBase> for <HourBase as ModifierValue>::Type` triggers a rustc 1.96 coherence false-positive (E0119) against typst's `Numeric` blanket impls in `typst-library/src/visualize/{paint,image}.rs` and `layout/rel.rs`. No compatible older `time` (≤0.3.40) is vendored, and `time`'s parsing feature cannot be disabled (typst calls `PrimitiveDateTime::parse`). `typst-as-lib` 0.14.4→typst 0.13, 0.15.5→typst 0.14: BOTH hit it.

**The Typst binary itself works** — `typst compile` renders both `templates/cv/classic.typ` and `templates/letter/classic-letter.typ` to valid PDFs (the CI `foundation` smoke proves the CV path).

**How to apply:** Do NOT keep retrying crate-version permutations — it is a deterministic env blocker (P1-18 fail-fast). The render path in `crates/core/src/render.rs` is behind a `Renderer` seam: a `CliRenderer` (subprocess `typst compile`) works today; an `EmbeddedRenderer` (the §H contract) is feature-gated `embedded-typst` and compiles the instant a compatible `time` is vendored. Surfaced as DISCUSS-RENDER to FOUNDER: either (a) vendor a compatible `time` into the mirror to honour §H embedded verbatim, or (b) accept the CLI render path for slice 1 (a documented, reversible deviation from "no shell-out"). Recommendation: (a). Everything else (§A–§F engine, §E ledger guard, seam, commands) is green at 100% coverage independent of this.
