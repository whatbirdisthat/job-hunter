#!/usr/bin/env bash
# Build the portable Applicant Advocate CLI release bundle and a .tar.gz + .sha256.
#
# Produces a SELF-CONTAINED bundle that runs on any Linux x86_64 with no Rust,
# Node.js, or typst installed: a statically-linked (musl) binary + the bundled
# `typst` renderer + templates + Liberation fonts + a synthetic sample.
#
# Build-host requirements: cargo, rustup, `typst` on PATH, and the musl C toolchain
# (`musl-tools` → x86_64-linux-musl-gcc) which the `zip`/`docx-rs` deps need to
# cross-compile the static musl target. On Debian/Ubuntu: `sudo apt-get install -y musl-tools`.
# Usage:  scripts/package-cli.sh [version]   (run from the repo root)
set -euo pipefail

command -v x86_64-linux-musl-gcc >/dev/null 2>&1 || {
  echo "error: x86_64-linux-musl-gcc not found — install the musl toolchain (e.g. 'sudo apt-get install -y musl-tools')"; exit 1; }

VER="${1:-0.1.0}"
TARGET="x86_64-unknown-linux-musl"
NAME="applicant-advocate-${VER}-linux-x86_64"
DIST="dist"
B="${DIST}/${NAME}"

command -v cargo >/dev/null || { echo "error: need cargo"; exit 1; }
command -v typst >/dev/null || { echo "error: need 'typst' on PATH to bundle the renderer"; exit 1; }
rustup target add "${TARGET}" >/dev/null 2>&1 || true

echo "==> building static binary (${TARGET})"
cargo build --release -p aa-cli --target "${TARGET}"

echo "==> assembling ${B}"
rm -rf "${B}"
mkdir -p "${B}/templates" "${B}/fonts" "${B}/samples"
cp "target/${TARGET}/release/applicant-advocate" "${B}/"
cp "$(command -v typst)" "${B}/typst"
cp -r templates/cv templates/letter "${B}/templates/"
cp crates/core/fonts/*.ttf "${B}/fonts/"
cp fixtures/personas/persona-001.cv.json "${B}/samples/sample-cv.json"
cp scripts/release/sample-job.txt "${B}/samples/sample-job.txt"
cp scripts/release/BUNDLE_README.txt "${B}/README.txt"
chmod +x "${B}/applicant-advocate" "${B}/typst"

echo "==> tarball + checksum"
tar -C "${DIST}" -czf "${DIST}/${NAME}.tar.gz" "${NAME}"
( cd "${DIST}" && sha256sum "${NAME}.tar.gz" > "${NAME}.tar.gz.sha256" )

echo "==> done"
ls -lh "${DIST}/${NAME}.tar.gz"
cat "${DIST}/${NAME}.tar.gz.sha256"
