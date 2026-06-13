//! Item #10 — L5 STORY: the DOCX output journey, end to end, through the REAL
//! `applicant-advocate` binary. The binding acceptance criterion for `--format both`:
//! against a COMPLETE synthetic persona the run writes OPENABLE `cv.docx` +
//! `cover-letter.docx` ALONGSIDE the `cv.pdf` + `cover-letter.pdf`.
//!
//! Journey (a complete, PII-free synthetic persona → NOT a sample → NO watermark →
//! plain `cv.docx`/`cover-letter.docx`, no `.SAMPLE.` infix):
//!   1. `--format both` → ALL FOUR files exist (cv.pdf, cover-letter.pdf, cv.docx,
//!      cover-letter.docx); the PDFs carry the `%PDF-` magic and each docx is OPENABLE
//!      (a valid ZIP containing `word/document.xml`), then
//!   2. `--format docx` (a SECOND out dir) → ONLY the two `.docx` files are written and
//!      NEITHER `.pdf` appears — proving the format selector actually selects.
//!
//! Perf-delta gated via the shared `perf_gate.rs` (reused from the cvimport tests by
//! `#[path]`) against a NEW TRACKED baseline `doc/perf/cli-docx-output-story-baseline.txt`.
//! Synthetic, PII-free fixtures only; deterministic; no network/LLM.

#[path = "../../cvimport/tests/perf_gate.rs"]
mod perf_gate;

use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const BUDGET_SECS: f64 = 60.0;
const DELTA_FACTOR: f64 = 3.0;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_applicant-advocate")
}

/// A COMPLETE synthetic persona (committed under the repo-root `fixtures/`): it parses
/// strictly, so the run does NOT trigger the missing-field flow — the NORMAL, non-sample
/// path that writes plain `cv.docx` / `cover-letter.docx` (no watermark, no `.SAMPLE.`).
fn complete_persona() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/personas/persona-001.cv.json")
}

/// A throwaway JD file under a unique temp dir.
fn write_jd(dir: &std::path::Path) -> PathBuf {
    let p = dir.join("jd.txt");
    std::fs::write(&p, "Backend engineer. Python and caching required.").unwrap();
    p
}

fn unique_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "aa-cli-docx-story-{tag}-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Assert a `.docx` at `path` is OPENABLE: a valid ZIP archive that contains the
/// `word/document.xml` part. This is the binding "openable docx" acceptance criterion.
fn assert_docx_openable(path: &std::path::Path) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap_or_else(|e| {
        panic!(
            "{} must be a valid ZIP (openable docx): {e}",
            path.display()
        )
    });
    assert!(
        zip.by_name("word/document.xml").is_ok(),
        "{} must contain word/document.xml to be an openable Word document",
        path.display()
    );
}

/// Assert the file at `path` is a PDF (carries the `%PDF-` magic prefix).
fn assert_is_pdf(path: &std::path::Path) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        bytes.starts_with(b"%PDF-"),
        "{} must start with the %PDF- magic to be a valid PDF",
        path.display()
    );
}

#[test]
fn story_docx_output_both_and_format_selection_perf_delta_gated() {
    let start = Instant::now();
    let root = repo_root();
    let persona = complete_persona();

    // ── 1. `--format both` → all four files; PDFs valid; both docx OPENABLE ──────────
    let both_dir = unique_dir("both");
    let jd = write_jd(&both_dir);
    let out = Command::new(bin())
        .current_dir(&root) // so the dev-checkout renderer resolves templates/
        .args(["--cv"])
        .arg(&persona)
        .args(["--jd"])
        .arg(&jd)
        .args(["--out"])
        .arg(&both_dir)
        .args(["--format", "both"])
        .output()
        .expect("run applicant-advocate --format both");
    assert!(
        out.status.success(),
        "--format both journey must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // A complete persona is NOT a sample → plain names, no `.SAMPLE.` infix.
    let cv_pdf = both_dir.join("cv.pdf");
    let cover_pdf = both_dir.join("cover-letter.pdf");
    let cv_docx = both_dir.join("cv.docx");
    let cover_docx = both_dir.join("cover-letter.docx");
    for f in [&cv_pdf, &cover_pdf, &cv_docx, &cover_docx] {
        assert!(
            f.exists(),
            "--format both must write {} (complete persona → non-sample path)",
            f.display()
        );
    }
    // A non-sample run never emits the watermarked `.SAMPLE.` variants.
    assert!(!both_dir.join("cv.SAMPLE.docx").exists());
    assert!(!both_dir.join("cover-letter.SAMPLE.docx").exists());

    // the two PDFs are real PDFs …
    assert_is_pdf(&cv_pdf);
    assert_is_pdf(&cover_pdf);
    // … and each docx OPENS as a Word document (valid ZIP + word/document.xml).
    assert_docx_openable(&cv_docx);
    assert_docx_openable(&cover_docx);

    // ── 2. `--format docx` alone → ONLY the two docx; NEITHER pdf is written ─────────
    let docx_dir = unique_dir("docx-only");
    let jd2 = write_jd(&docx_dir);
    let out2 = Command::new(bin())
        .current_dir(&root)
        .args(["--cv"])
        .arg(&persona)
        .args(["--jd"])
        .arg(&jd2)
        .args(["--out"])
        .arg(&docx_dir)
        .args(["--format", "docx"])
        .output()
        .expect("run applicant-advocate --format docx");
    assert!(
        out2.status.success(),
        "--format docx journey must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let cv_docx_only = docx_dir.join("cv.docx");
    let cover_docx_only = docx_dir.join("cover-letter.docx");
    assert_docx_openable(&cv_docx_only);
    assert_docx_openable(&cover_docx_only);
    // format selection actually selects: no PDF leaks into a docx-only run.
    assert!(
        !docx_dir.join("cv.pdf").exists() && !docx_dir.join("cover-letter.pdf").exists(),
        "--format docx must NOT write any .pdf file"
    );

    // cleanup
    let _ = std::fs::remove_dir_all(&both_dir);
    let _ = std::fs::remove_dir_all(&docx_dir);

    // ── Perf-delta gate: time the whole journey against a TRACKED baseline ───────────
    let elapsed = start.elapsed().as_secs_f64();
    let baseline_path = root.join("doc/perf/cli-docx-output-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "CLI docx-output STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] cli docx-output journey: {elapsed:.3}s (budget {BUDGET_SECS}s, \
         baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
