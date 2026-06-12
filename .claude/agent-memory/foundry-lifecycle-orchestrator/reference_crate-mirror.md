---
name: crate-mirror
description: cargo in this env uses a private registry mirror "fleetmirror", not crates.io; only a subset of crate versions are vendored.
metadata:
  type: reference
---

`cargo` here resolves against a replacement registry named **`fleetmirror`** (config replaces `crates-io`). `cargo search`/`cargo update --precise` to a non-vendored version fails with "location searched: fleetmirror index ... perhaps a crate was updated and forgotten to be re-vendored". Network fetch of vendored versions works (serde, typst stack all download).

**How to apply:** before pinning a crate version, assume only one/a few patch versions are vendored. Check `find ~/.cargo/registry/src/mirror.local-*/ -maxdepth 1 -name '<crate>-*'` and `~/.cargo/registry/cache/`. Do not burn retries on `--precise` pins to versions that may not exist in the mirror.
