# PR_REVIEW.md — item #5 (Application tracker / CRM)

**Range:** `main..item-5-tracker-crm` · **Date:** 2026-06-13 · **Mode:** adversarial fan-out (refute-the-change) · **Governance:** pr-approval

## VERDICT: NEEDS_REVISION → (revised; see foot)

Max-severity rule across 6 reviewer roles + SENTINEL security-gate. One HIGH (security) + converging MEDIUMs (doc-vs-code path divergence found by three lenses; perf baseline too loose; stray doc artifact) gated the change on first pass. No CRITICAL — the security defect was latent (the Tauri host that would reach the insecure default is not wired in this slice). The deterministic logic, architecture, and regression surface were clean on first pass.

## Findings (first pass)

| # | Severity | Lens(es) | Locus | Finding |
|---|----------|----------|-------|---------|
| 1 | HIGH | SECURITY | `lib.rs` default + `tracker_store.rs` | `Session::default()` persisted plaintext PII to a fixed shared-temp path `std::env::temp_dir().join("aa-tracker.json")` (world-readable umask); predictable temp sibling written without O_EXCL/O_NOFOLLOW (symlink-follow / clobber). Strictly worse at-rest posture than slices 1-4 (in-memory). DISCUSS-STORAGE deferred *encryption*, NOT file *location*. |
| 2 | MEDIUM | SECURITY, DOCUMENT | `doc/design/item-5-storage-decision.md` | Decision record stated the store path is "under the OS app-data dir; tests inject a temp dir" — but the shipped default WAS the temp dir (no prod `with_tracker_store` wiring; only tests called it). Doc misdescribed where the at-rest file lives. |
| 3 | MEDIUM | PERFORMANCE | `doc/perf/desktop-tracker-story-baseline.txt` | Baseline `0.500000` vs ~0.001s measured steady-state ⇒ delta arm fired only above 1.5s (~1500× tolerance). Mechanism non-vacuous (proven by `perf_gate_l1.rs`), but THIS baseline too loose to catch a real 100-1000× regression of this story. |
| 4 | MEDIUM | DOCUMENT | `doc/design/item-5-storage-decision.md:82-83` | Stray authoring-tool artifact lines `</content>` / `</invoke>` committed at the foot of the doc. |
| L | LOW | CORRECTNESS, ARCHITECTURE | `lib.rs` default | (non-gating) default store eagerly built at the shared temp path; relates to #1/#2. |
| S | SUGGESTION | CORRECTNESS | `tracker_store.rs` save | (non-gating) no fsync before rename — process-crash safe (claim satisfied) but not power-loss durable. |

## Per-role verdicts (first pass)
- CORRECTNESS — **PASS** (every fence-post / matrix / date-anchor probe refuted; logic correct)
- REGRESSION — **PASS** (purely additive to `lib.rs`; existing perf baselines byte-identical; full workspace + UI suites green)
- ARCHITECTURE — **PASS** (aa-tracker → aa-core only; pure cores, no clock/IO; real `TrackerStore` seam; honest deferral grounds)
- SECURITY — **NEEDS_REVISION** (#1 HIGH, #2 MEDIUM)
- PERFORMANCE — **NEEDS_REVISION** (#3 MEDIUM)
- DOCUMENT — **NEEDS_REVISION** (#2, #4 MEDIUM)
- SENTINEL security-gate — **PASS** (pii-audit clean — Contact carries no email/phone field; secret-scan clean; dependency-audit dependency-neutral, one local path crate, no external/transitive additions)

## Adversarial verification (HIGH/MEDIUM refutation pass)
- #1 escalation to CRITICAL **refuted**: no in-tree Tauri `generate_handler`/`manage()` reaches `Session::default()` for tracker persistence in this slice → held at HIGH (latent).
- #3 OR-vacuity (the item-2 class) **refuted**: the gate has no `|| budget` disjunct; the issue is solely the loose baseline value → MEDIUM (gate-quality, not catastrophe).

## What was NOT reviewed
- No running Tauri app / rendered UI crawl (UI proven by local RTL only, per the npm-registry CI constraint, issue #2).
- Power-loss (vs process-crash) durability of the atomic write — noted as SUGGESTION.
- Cross-platform OS app-data-dir behaviour (no host wiring yet to exercise).

## Resolution (revision 1) — VERDICT: PASS

All gating findings fixed and re-verified by the originating reviewers on the revised diff:

| # | First-pass | Fix | Re-review |
|---|-----------|-----|-----------|
| 1 (HIGH, security) | shared-temp plaintext PII + symlink-followable temp | `Session::default()` → per-user app-data path (`$XDG_DATA_HOME`/`$HOME/.local/share` under `job-hunter/`); file 0600 + dir 0700 on Unix; non-predictable same-dir temp (`pid+nanos`); std-only, no new dep, no `unsafe`; perm test added | **SECURITY: PASS** |
| 2 (MEDIUM, doc) | doc claimed app-data dir the code didn't take | doc rewritten to the true per-user-private 0600 posture | **SECURITY/DOCUMENT: PASS** |
| 3 (MEDIUM, perf) | baseline 0.5 vs ~0.001s (delta arm dead) | baseline → `0.030000` (~30× steady-state; delta arm trips at 0.090s); README headroom rule added | **PERFORMANCE: PASS** |
| 4 (MEDIUM, doc) | stray `</content>`/`</invoke>` artifact | removed | **DOCUMENT/PERFORMANCE: PASS** |

### Residual (non-gating, accepted-with-rationale) — recorded as follow-up
- **MEDIUM (accepted)** `tracker_store.rs` no-HOME fallback: on the `$TMPDIR/aa-tracker-<user>/job-hunter/` fallback path, only the leaf dir is chmodded 0700; the intermediate `aa-tracker-<user>` (predictable name in a world-writable tmp) is not, leaving a symlink-pre-plant redirect vector. **Does not gate:** unreachable whenever `HOME` is set (every real desktop / Tauri session — the production path); the 0600 file mode preserves PII *content* confidentiality even through a redirected dir (residual is integrity/DoS, not disclosure); the plaintext-but-private posture is documented. **Follow-up:** chmod each created ancestor (or `mkdir`-per-component with `O_NOFOLLOW`) on the temp fallback — carried alongside DISCUSS-STORAGE for the dedicated storage slice.

**Final synthesized verdict: PASS.** Gates green locally: fmt ✓, clippy -D warnings ✓, `cargo test --workspace` ✓ (L1-L5), `llvm-cov --fail-under-lines 99` ✓ (99.12%), SENTINEL security-gate ✓ (PII/secrets/deps). Per `pr-approval` governance, FOUNDRY opens a PR; the human merges.
