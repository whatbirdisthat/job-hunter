# Coverage policy — slice 1

**Floor: 100% of reachable code.** Measured with `cargo llvm-cov --workspace --all-targets`.
Line coverage of all *reachable* statements is **100%**. The residual sub-line *region*
misses (raw line metric reads 99.35%) are enumerated below, each a justified pragma per the
FOUNDRY test contract ("the only path below the floor is an explicit pragma with a stated
reason"). They fall into three classes, all **unreachable on any valid input**:

## P-COV-1 — Infallible serde-serialize error arms
`MasterCv::to_json`, `NormalizedJob::to_json` (core `types.rs:142`, `job.rs:39`),
the `render_cover_letter` / `render_cover_letter_watermarked` serialize (`render.rs:322,343`; the
second arm is the item-8b watermark variant, same class), and the desktop `seam` serialize/deserialize
(`lib.rs:49,50`). `serde_json::to_string` of an in-memory struct whose fields are only
`String`/`Vec`/`Option`/number **cannot fail** (no maps with non-string keys, no custom
`Serialize` that errors). The `.map_err(...)` closure is dead by construction. Kept because the
methods expose a `Result` API for forward-compatibility (a future field could be fallible).

## P-COV-2 — Defensive filesystem error closures
`CliRenderer::compile_inner` write/read error arms and `repo_root`'s `canonicalize` fallback
(`render.rs:263,265,302,183`; the temp-data-file create + write-data + read-pdf closures and the
canonicalize fallback). These fire only on transient OS I/O failures (disk full,
permission race) that cannot be triggered deterministically offline without root-level fault
injection. The *spawn-failure* and *typst-compile-failure* arms ARE covered
(`cli_renderer_reports_typst_compile_failure`, `cli_renderer_errors_when_root_missing`).

## P-COV-3 — Feature-gated §H embedded renderer
The `embedded` module in `render.rs` (behind `--features embedded-typst`) is **not compiled**
under default features and so is not in the default coverage set. It is deferred pending
DISCUSS-RENDER (the `time 0.3.48` ↔ typst coherence blocker). When the feature compiles, its
tests run under the same five-level contract.

## P-COV-4 — The `applicant-advocate` CLI binary (release tooling)
`crates/cli/src/main.rs` is a thin binary **entrypoint** — argument parsing + filesystem IO that
wires the already-100%-covered engine (`tailor`/`guard`/`render`) into a command-line tool. It is
excluded from the coverage floor via `--ignore-filename-regex 'crates/cli/'` (CI), the way binary
`main`s conventionally are. Its end-to-end behaviour is proven by the standalone bundle smoke
(build → run under a stripped env → two valid PDFs) documented in `crates/cli/README.md` and the
release PR. The renderer overrides it relies on (`CliRenderer::with_typst_bin` /
`with_font_path`) ARE covered in-crate, by `renderer_honours_builder_overrides`.

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

---

# Coverage policy — item 8a (`crates/cvimport`, adaptive JSON miner)

**Same floor: 100% of reachable code** (`cargo llvm-cov --workspace --all-targets
--ignore-filename-regex 'crates/cli/' --fail-under-lines 99`). Item 8a adds one private module
`crates/cvimport/src/mine_json.rs` (the adaptive JSON miner) + its L1 in-module tests, the L2/L4/L5
integration targets, the two L3 cases in `boundary_schema.rs`, and the synthetic input fixtures
under `crates/cvimport/tests/fixtures/json/`.

**`mine_json.rs` is 100.00% — regions, functions AND lines — with ZERO new pragmas.** A pure
`serde_json::Value` walker that takes a parsed `&Value` and returns a struct has no infallible-
serialize or defensive-IO arms to exempt, so every branch is reachable from a real input. The L1
in-module tests carry the burden (every person/experience/skill synonym arm; the dedicated-object
preference + top-level fallback; multi-array disambiguation incl. the empty-higher-priority-array
skip; the jobTitle-required emission rule; achievement newline-split, object-form, and the
**skip-unusable-array-element** arms; numeric-date coercion incl. the float-truncation and u64 arms;
the proficiency honesty default for every out-of-range/non-integer/non-number/absent case; the
**blank-string and non-object skill** drop arms; the Empty gate; the no-mutation guarantee; and the
arbitrary/hostile-input never-panic guarantee). Workspace line coverage with item 8a is **99.36%**
(above the floor).

## What IS covered to 100% (reachable) — cvimport JSON miner
`import_cv_json` (orchestration + Empty gate), `completeness` (every `missing_*` flag true/false +
`is_complete`), `ignored_role_arrays`, and all extractor helpers (`person_source`, `extract_person`,
`extract_experience`, `extract_achievements`, `extract_skills`) plus the primitives (`lc_get`,
`str_field`, `coerce_date`, `split_achievement`, `skill_from`, `proficiency_of`). The L5 STORY
(`tests/mine_json_story_l5.rs`) is perf-delta gated against the NEW TRACKED baseline
`doc/perf/cvimport-jsonmine-story-baseline.txt` (`0.075000`s ≈ 3.4× the ~0.022s observed
steady-state — NOT the import-story baseline, whose render+extract journey is orders of magnitude
slower; sharing it would make the 3×-delta arm vacuous). **No `P-COV-cvimport-mine-N` pragma was
required.**

---

# Coverage policy — item #3 (`crates/advocate`, Applicant Advocate LLM layer)

**Same floor: 100% of reachable code** (`cargo llvm-cov --workspace --all-targets`,
`--fail-under-lines 99`). Workspace line coverage with item #3 is **99.36%** (above the floor).

## `crates/advocate` — 100% lines, NO new pragmas
Every reachable line of the advocate crate is covered by the L1/L2/L3 tests: `redact`/`redact_kind`,
`build_prompt` (BOTH `RewriteKind` arms), `StubProvider::{new, fabricating, rewrite, name}` (honest
AND fabricating modes), `AdvocateConfig::default` (disabled), the `AdvocateError` Display variants,
and the `RewriteKind` serde round-trip. The PII firewall is proven structurally at L3
(`outbound_payload_has_no_master_cv_fields`: the serialized `RewriteRequest` keys are EXACTLY
`{evidence_id, evidence_text, requirement, kind}`).

The **`live-http` adapters** (`src/live.rs`: `OllamaProvider`, `HttpKeyProvider`) are **not compiled
under default features** and so are **not in the default coverage set** — exactly the
**P-COV-3 feature-gated class** (the slice-1 embedded renderer). They are a compile-gated,
network-only surface verified to build + lint clean under `--features live-http`
(`cargo clippy -p aa-advocate --features live-http --all-targets -- -D warnings`), but CI proves the
advocate contract with the deterministic `StubProvider` instead of a live model (NO network in CI by
construction — `ureq` is absent from the default dependency tree). When a live-model integration
harness is added it runs under the same five-level contract; until then the adapters carry no
pragma because they are simply not in the default-feature reachable set.

The **network-free portions** of the adapters DO carry unit tests under `#[cfg(feature="live-http")]`
(also the **P-COV-3 feature-gated class**, exercised by `cargo test -p aa-advocate --features
live-http`, NOT by the default `--workspace` gate): the `HttpKeyProvider` `https://`-only scheme guard
(parse-don't-validate — `http_key_provider_rejects_insecure_endpoint` /
`_accepts_https_endpoint` / `_rejects_schemeless_endpoint`) and the secret-redacting `Debug` impl
(`http_key_provider_debug_redacts_api_key`). These fire BEFORE any HTTP call, so they need no socket.
The only lines still uncovered in `src/live.rs` are the actual send/parse paths that require a live
endpoint — they remain in the P-COV-3 not-in-default-reachable-set class until a live harness exists.

## `apps/desktop/src-tauri/lib.rs` — item-#3 additions, no NEW pragma class
The advocate export path (`prepare_export` / `export_application` / `render_inputs` /
`set_advocate_enabled` / `advocate_enabled` / the `From<AdvocateError>` conv) is covered by the L4
system tests (`tests/advocate_l4.rs`) and the L5 advocate STORY (`tests/story_l5.rs`). All observable
branches are exercised:
- flag ON + honest stub → two valid PDFs, `ai_used == true` (`rewrite_enabled_clean_stub_exports_two_pdfs`);
- **the mandatory adversarial test** — a fabricated CV-bullet cited id is NAMED + BLOCKED
  (`adversarial_stub_fabricates_dangling_id_blocks_export`) with its **non-vacuous twin** (the same
  honest-stub journey PASSES — `non_vacuous_twin_honest_stub_same_journey_passes`);
- the **cover-letter strength** fabrication branch (the `else` adopt-cited-id arm + the letter
  re-guard) is covered by `fabricated_cover_letter_strength_id_blocks_export` (honest for bullets,
  fabricating only for strengths, so the CV guard passes and the letter re-guard fires);
- flag OFF → render inputs byte-identical to the deterministic path
  (`flag_off_is_byte_identical_to_deterministic` — compares the *pre-render* view/letter JSON, NOT
  the PDF bytes, because typst PDFs are not byte-stable across invocations per R-D2);
- flag ON + unreachable provider → explicit `CommandError::Advocate`, **no silent fallback**
  (`unreachable_provider_surfaces_error`).

The residual sub-line region misses in `lib.rs` (raw line metric 98.88%) are the **`?`-operator
error-propagation arms** — the early-return `Err` fragment of `self.provider.rewrite(&req)?` on the
happy path, and the `ok_or(NoMasterCv)/ok_or(NoJob)?` arms reached through `prepare_export`'s callers.
These are the **same P-COV-1 infallible/defensive-error-arm class** as slice-1: the error *values* a
caller can observe (`Advocate`, `ExportBlocked`, `NoMasterCv`, `NoJob`) are ALL exercised by dedicated
tests; the un-hit fragments are the short-circuit paths the happy-path control flow does not take.
No NEW pragma id is introduced — these fall under the existing **P-COV-1** rationale.

## `crates/core/src/tailor.rs` — `requirement_for` is 100%
The item-#3 core helper `requirement_for` (R-ADV-12, top-matching must-have or joined-list fallback)
is covered to 100% by four L1 tests: top-match, no-single-match → joined list, unknown id → joined
list, and empty-job → empty string.

---

# Coverage policy — item #5 (`crates/tracker`, application tracker / CRM)

**Same floor: 100% of reachable code** (`cargo llvm-cov --workspace --all-targets`,
`--fail-under-lines 99`). Workspace line coverage with item #5 is **99.27%** (above the floor).

## `crates/tracker` — 100% lines on ALL FOUR pure cores, NO new pragmas
Every reachable line of the four pure cores is covered to **100% lines** by the L1/L2 tests:
- `lifecycle.rs` — the full `AppState × AppState` matrix is enumerated (legal-or-error per cell,
  the non-vacuous twin proving the table is not trivially satisfiable), `Closed` terminal, and the
  `AppState::parse` happy/bad-string arms;
- `scheduler.rs` — the pinned aging boundaries (day 2/3/5/6/7/10/11), the month/year-boundary +
  same-day `days_since`, the future-date clamp (DISCUSS-FUTUREDATE), and the window constants;
- `callsheet.rs` — every brief field per row, deterministic priority ordering + tie-break,
  template fill, actionable-only filtering, and the clock-injected two-days-differ case. The
  priority/window logic is keyed on `NextAction` (the two actionable variants only), so there is
  **no unreachable non-actionable arm** — every branch is exercised (no pragma);
- `crm.rs` / `date.rs` — contact/note model, the timeline append + linkage resolution, the
  `Channel`/`Outcome` parse arms, and the civil-day-number `Date` arithmetic (incl. leap day).

The residual sub-line **region/branch** misses in `date.rs` (98.75% regions) and `lifecycle.rs`
(97.60% regions) are NOT missed *lines* (both are **100% lines**): they are interior `match`/`if`
fan-out the line metric already counts as covered. No pragma id is introduced — the cores meet the
100%-of-reachable-**lines** floor with zero documented exceptions.

## `apps/desktop/src-tauri` — command-layer tracker additions, existing P-COV-1/P-COV-2 classes
The tracker command surface (`track_application`/`advance_application`/`add_contact`/`link_contact`/
`add_note`/`daily_call_sheet`/`list_applications` + the `From<TransitionError>`/`From<ParseEnumError>`/
`From<StoreError>` convs) and the `TrackerStore`/`JsonFileStore` adapter are covered by the L4 system
tests (`tests/tracker_l4.rs`, `tests/tracker_store_errors_l4.rs`) and the L5 STORY
(`tests/tracker_story_l5.rs`). Every observable error VALUE a caller can hit is exercised:
- the illegal-transition `CommandError::Tracker` (with its non-vacuous legal twin), the bad-enum
  `Channel`/`Outcome`/`AppState` strings, and the unknown-application/contact id errors;
- the persistence round-trip (a second store over the same path loads the same doc), the atomic
  temp+rename durability, the **load-of-corrupt-file `StoreError::Serde`** arm, and the
  **save-to-an-unwritable-path `StoreError::Io`** arm.

The residual misses in `tracker_store.rs` are the **infallible `to_string_pretty`
serialize arm** (P-COV-1: serializing an in-memory struct of `String`/`Vec`/`Option`/number cannot
fail) and the **defensive write/rename/chmod I/O closures** (P-COV-2: transient OS faults that
cannot be triggered deterministically offline — the *reachable* read/parse error IS covered). The
residual misses in `apps/desktop/src-tauri/src/lib.rs` are the tracker command `?`-operator
early-return fragments (the short-circuit `Err` path the happy-path control flow does not take) —
the **same P-COV-1** class already documented for item #3. No NEW pragma id is introduced.

### Item #5 review (Finding 1) — per-user store hardening, 100% of reachable code, no new pragma
The security hardening (per-user app-data default path, `0700`/`0600` Unix modes, non-predictable
same-dir atomic temp) keeps `tracker_store.rs` at the 100%-of-reachable-lines floor with the SAME
P-COV-1/P-COV-2 classes — by construction, NOT by widening the pragma surface:
- the path-resolution branch logic is a **pure** `resolve_default_path(xdg, home, temp, tag)` +
  `user_tag_from(user, username)` (env read only at the thin boundary), so EVERY branch — XDG
  present / empty / absent, HOME present / empty / absent, the per-user temp fallback, and the
  `USER`→`USERNAME`→`"shared"` precedence — is unit-tested **without racy env mutation**
  (`#[cfg(test)]` in `tracker_store.rs`);
- the atomic-write `tmp_path` no-filename / no-parent arms are exercised directly
  (`tmp_path_handles_path_without_a_filename` / `_empty_path_without_a_parent`);
- the Unix `0600`/`0700` SUCCESS path is an L4 coordinate
  (`saved_file_is_owner_only_0600_and_dir_0700`, `#[cfg(unix)]`) and the same-dir temp location is
  pinned by `temp_sibling_lives_in_the_store_dir_not_shared_temp`; the default path is pinned by
  `default_path_is_per_user_app_scoped_not_shared_temp_file`.
The only un-hit fragments remain the **defensive `create_dir_all`/`set_permissions` error
closures** (P-COV-2: an injected dir already exists so the create+chmod success path runs, and the
error arm is a transient-OS-fault closure that cannot be triggered deterministically offline —
the create-dir failure IS observed via the save-to-an-unwritable-path L4 test). No NEW pragma id.

---

# Coverage policy — item 8b (adaptive CLI ingestion + SAMPLE honesty guard)

**Same floor: 100% of reachable code** (`cargo llvm-cov --workspace --all-targets
--ignore-filename-regex 'crates/cli/' --summary-only --fail-under-lines 99`). Workspace line
coverage with item 8b is **99.38%** (above the floor). Item 8b adds the pure decision module
`crates/core/src/samples.rs`, watermark-threading methods in `crates/core/src/render.rs`, the
adaptive ingestion + missing-field flow in `crates/cli/src/main.rs` (P-COV-4 excluded), a
cross-item synonym addition in `crates/cvimport/src/mine_json.rs` (DISCUSS-8b-1), the new render
templates' watermark overlay, and two new test targets + a tracked STORY baseline.

## `crates/core/src/samples.rs` — 100% lines, regions AND functions, NO new pragma
The load-bearing SAMPLE-guard logic is a pure module with no infallible-serialize or defensive-IO
arms, so every branch is reachable from a real input. The L1 in-module tests carry the burden:
`decide()`'s full truth table (no-samples→RenderNormal for BOTH flag values, samples+no-allow→
Blocked, samples+allow→RenderWithWatermark); the `ExportDecision::{renders,is_sample}` predicates;
`cv_filename`/`cover_letter_filename` for both sample-ness; `MissingFields::any`; and
`fill_with_samples` for every IMPORTANT class — name, whole-experience synthesis (real synthetic
ids `imp_exp_0`/`imp_exp_0_b0`), achievement-attach-to-existing, the **no-experience-to-attach-to
fail-CLOSED arm** (returns `used_samples=false`, inserts nothing), skill, the all-classes case, and
the nothing-missing no-op. The watermark sentinel + blocked-message wording are pinned to exact
strings (a safety message must not silently drift). **No `P-COV-8b-N` pragma was required.**

## `crates/core/src/render.rs` — item-8b watermark additions, existing P-COV-1/P-COV-2 classes only
`compile_inner` threads a SECOND typst `--input samples=<bool>` (the normal path passes `false`,
which is byte-identical to the pre-8b render — verified). `render_cv_watermarked` /
`render_cover_letter_watermarked` (the `CliRenderer` overrides) ARE covered by the L-render
integration tests in `crates/core/tests/watermark_render.rs` (sample-present + normal-absent, both
directions, classic + compact + letter, via `pdf-extract`). The **trait-default** watermark methods
(`render.rs:156-175`, the ADDITIVE delegate-and-ignore arms) are pinned by
`trait_default_watermarked_methods_ignore_the_flag_and_delegate`. The only un-hit render.rs
fragments remain the SAME P-COV-1 (serialize) + P-COV-2 (defensive IO) closures already documented
above and the P-COV-3 feature-gated embedded module — item 8b introduces **no new pragma id**.

## `crates/cvimport/src/mine_json.rs` — DISCUSS-8b-1 synonym addition, stays 100%
`extract_person` gains the real-DW_CV contact synonyms `emailAddress`/`mail`/`e-mail` and
`phoneNumber`/`mobile`/`tel`/`telephone` (appended AFTER `email`/`phone`, so no existing fixture
changes which field it picks — key-match is exact case-insensitive equality, not substring). Pinned
by `dwcv_email_address_and_phone_number_synonyms_survive`. `mine_json.rs` remains **100% lines**;
the 8a STORY stays byte-identical. (Rationale: the real DW_CV keys contact as `EmailAddress`/
`PhoneNumber`, distinct WORDS from `email`/`phone`, so case-insensitivity alone dropped them — the
item-8b acceptance requires email/experience intact.)

## What IS covered to 100% (reachable) — item 8b
The whole SAMPLE-guard decision surface and watermark render path. The CLI binary (`main.rs`:
strict-then-mine `ingest`, `resolve_gaps`, the single `decide()` gate, interactive prompting, output
naming) is the **P-COV-4** excluded entrypoint — its end-to-end behaviour is proven by the L5 STORY
`crates/cli/tests/ingestion_story_l5.rs` (mine a synthetic gap-having JSON → `--use-fakes` →
watermarked `cv.SAMPLE.pdf` produced and the watermark text extracted from it; normal export refused
without the flag), perf-delta gated against the NEW TRACKED baseline
`doc/perf/cli-ingestion-story-baseline.txt` (`0.400000`s ≈ 4× the ~0.10s observed steady-state —
its own baseline, not a shared one, so the 3×-delta arm is non-vacuous).
