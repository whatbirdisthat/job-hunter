# Storage decision — item #5 (Application tracker / CRM)

> Decision record for the persistence mechanism of the phase-2 workflow slice. The
> FOUNDER has already chosen the bottom line; this record encodes the rationale, the seam,
> and the deferred work. **This is not a relitigation** — it documents WHY the smallest sound
> option is correct for THIS slice and how the heavier option stays a localized later change.

---

## Context

ARCHITECTURE.md names **SQLite/SQLCipher** as the v1 persistence target. It was **never wired**:

- slice-1's "SQLCipher store" is an *aspirational comment* in `apps/desktop/src-tauri/src/lib.rs`
  (the `Session` doc-comment says "the SQLCipher-backed store persists the imported master CV" —
  there is no such store);
- the `Session` is **in-memory** (`master: Option<MasterCv>`, `job: Option<CoreJob>`,
  `decisions: BTreeMap<…>`) — nothing is persisted across process restarts today;
- there are **zero** `sqlite`/`rusqlite`/`sqlcipher` dependencies anywhere in the workspace.

Item #5 is the FIRST slice that genuinely needs durable on-device state: applications, their
lifecycle, contacts, and notes must survive a restart to be a tracker at all.

## Decision

**This slice ships a documented local-file JSON store behind a `TrackerStore` trait/seam.**

- (a) **Seam:** a `TrackerStore` trait defines the persistence port (load/save the tracker
  document). The pure cores (lifecycle SM, scheduler, call-sheet builder) take NO store — they
  are clock-injected pure functions over values. Only the thin command layer touches the store.
- (b) **Concrete impl this slice:** `JsonFileStore` — a single on-device JSON document written
  **atomically** (write to a NON-PREDICTABLE temp sibling IN THE SAME private dir, then `rename`
  over the target, so a crash mid-write never corrupts the live file and the rename is
  same-filesystem). The **default** path is a **per-user, app-scoped data dir** — `$XDG_DATA_HOME`
  else `$HOME/.local/share` (else the platform equivalent), under a **`job-hunter/`** subdir, i.e.
  `<data_dir>/job-hunter/tracker.json` — NOT the shared world-readable temp dir (Finding 1). On
  Unix the dir we create is `0700` and the file `0600` (`std::os::unix::fs::PermissionsExt`,
  std-only — **no new dependency**). Only when no home is resolvable does it fall back to a
  **per-user-isolated** subdir under the temp dir (`aa-tracker-<user>/`), never a single shared
  file. The temp sibling's name carries the pid + a monotonic nanos counter so it cannot be
  pre-created/symlinked. Tests inject a temp dir via `with_tracker_store` (an injected/pre-existing
  dir is the host's to own — the store does not re-chmod it).
- (c) **DISCUSS-STORAGE (deferred):** full **encryption-at-rest via SQLCipher** is deferred to a
  dedicated **storage slice**, with the recommendation to adopt SQLCipher **behind the same
  `TrackerStore` seam** — swapping the impl, not rewriting the layer.
- (d) **Security gate:** IF encrypted storage IS later introduced and SENTINEL is available, run
  `/security-gate` on that slice (key management, at-rest threat model, key-derivation UX).

## Rationale

**Why not SQLCipher now.** SQLCipher pulls a **C / OpenSSL build toolchain** (or a vendored
`libsqlite3-sys` + sqlcipher amalgamation) plus a **key-management UX** (where the at-rest key
lives, how it is derived from a user secret, how it is rotated) onto:

- **registry-constrained CI** — the runners cannot reach package registries (issue #2); a C build
  dependency with system-library expectations is exactly the heavy, flaky surface the project has
  fought to keep off the blocking path (`rust-workspace` builds pure-Rust crates only);
- **deterministic CI** — encryption introduces nondeterministic ciphertext + key state that the
  L1–L5 coordinate discipline does not need in order to test the *workflow logic*.

That cost is **disproportionate** for a workflow slice whose value is the lifecycle/scheduler/
call-sheet logic, not the bytes-on-disk format.

**Why the seam makes deferral safe.** The pure cores have **no storage dependency at all** — they
are clock-injected pure functions over plain values, so the persistence choice **never touches the
tested coordinates**. Swapping `JsonFileStore` for a `SqlCipherStore` later is a **localized change
behind `TrackerStore`**, not a rewrite: the command layer, the cores, the EARS coordinates, and the
STORY journey are all unchanged. The seam is the whole point — it converts "we picked the wrong DB"
from a multiplicative downstream cost into a single-file swap.

**Why a JSON file is sound here.** The tracker document is small (tens to low-hundreds of
applications + contacts for a single on-device user), single-writer (one desktop process), and
already has a serde representation for free (the cores' types are `Serialize`/`Deserialize`).
Atomic rename gives crash-safety without a transaction engine. This is the smallest mechanism that
is actually durable.

## Threat-model note (recorded, deferred)

The JSON file is **plaintext at rest** (encryption is still deferred — DISCUSS-STORAGE), but it is
now **per-user-private (`0600`), NOT world-readable**: the default store lives in a per-user
app-data dir (`0700` on Unix), not the shared temp dir, so another local user cannot read the
plaintext PII. For a single-user, on-device app this matches the slice-1/2 posture (the imported
master CV and parsed jobs were never persisted encrypted either — they were in-memory). The privacy
model in the brief (no PII leaves the device; nothing sent to a model without redaction) is
**unaffected** — this is a *local at-rest* question, not a *data-egress* one. **Encryption** at rest
(confidentiality against an attacker with raw disk access) remains the DISCUSS-STORAGE follow-up; it
is a real hardening item, not a gap this slice silently ignores. The per-user `0600`/`0700` hardening
(Finding 1) closes the world-readable-shared-temp vector in the interim.

**Residual (accepted, non-gating — follow-up).** On the *no-`HOME`* fallback path
(`$TMPDIR/aa-tracker-<user>/job-hunter/`), only the leaf dir is chmodded `0700`; the intermediate
`aa-tracker-<user>` (a predictable name in a world-writable tmp) is not, leaving a symlink-pre-plant
redirect vector. This is **unreachable whenever `HOME` is set** — i.e. every real desktop / Tauri
session, the production path — and the `0600` file mode preserves PII *content* confidentiality even
through a redirected dir (the residual is integrity/DoS, not disclosure). **Follow-up** (carried with
DISCUSS-STORAGE): chmod each created ancestor, or `mkdir`-per-component with `O_NOFOLLOW`, on the
temp fallback.

## What this record binds

- The spec (`doc/spec/item-5-tracker-crm.md`) defines `TrackerStore` + `JsonFileStore` per (a)/(b).
- No `sqlite`/`rusqlite`/`sqlcipher` dependency enters the workspace in item #5.
- The DISCUSS-STORAGE item is carried forward to the FOUNDER for a dedicated storage slice.
