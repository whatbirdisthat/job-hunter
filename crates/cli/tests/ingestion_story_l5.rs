//! Item 8b — L5 STORY: the adaptive CLI ingestion + SAMPLE honesty-guard journey,
//! end to end, through the REAL `applicant-advocate` binary (R-INGEST-CLI-1..5).
//!
//! Journey: a synthetic gap-having CV JSON (strict-parse-rejected) is
//!   1. mined by the adaptive importer, then
//!   2. run with `--use-fakes` → a WATERMARKED `cv.SAMPLE.pdf` + `cover-letter.SAMPLE.pdf`
//!      are produced (and the watermark text is extractable from the CV), AND
//!   3. run WITHOUT `--allow-samples`/`--use-fakes` (non-interactive) → export is REFUSED
//!      and NO files are written.
//!
//! Perf-delta gated via the shared `perf_gate.rs` (reused from the cvimport tests by
//! `#[path]`) against a NEW TRACKED baseline `doc/perf/cli-ingestion-story-baseline.txt`.
//! Synthetic, PII-free fixtures only.

#[path = "../../cvimport/tests/perf_gate.rs"]
mod perf_gate;

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

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gappy-cv.json")
}

/// A throwaway JD file under a unique temp dir.
fn write_jd(dir: &std::path::Path) -> PathBuf {
    let p = dir.join("jd.txt");
    std::fs::write(&p, "Backend engineer. Python and caching required.").unwrap();
    p
}

fn unique_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "aa-cli-story-{tag}-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn story_adaptive_ingestion_sample_guard_journey_perf_delta_gated() {
    let start = Instant::now();
    let root = repo_root();

    // ── 1+2. --use-fakes → watermarked *.SAMPLE.pdf produced (the "see it working" path) ─
    let sample_dir = unique_dir("fakes");
    let jd = write_jd(&sample_dir);
    let out = Command::new(bin())
        .current_dir(&root) // so the dev-checkout renderer resolves templates/
        .args(["--cv"])
        .arg(fixture())
        .args(["--jd"])
        .arg(&jd)
        .args(["--out"])
        .arg(&sample_dir)
        .arg("--use-fakes")
        .output()
        .expect("run applicant-advocate --use-fakes");
    assert!(
        out.status.success(),
        "--use-fakes journey must succeed; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let cv_sample = sample_dir.join("cv.SAMPLE.pdf");
    let cover_sample = sample_dir.join("cover-letter.SAMPLE.pdf");
    assert!(
        cv_sample.exists(),
        "sample CV must be written to cv.SAMPLE.pdf"
    );
    assert!(
        cover_sample.exists(),
        "sample cover letter must be written to cover-letter.SAMPLE.pdf"
    );
    // normal names must NOT exist (a sample never masquerades as a real document)
    assert!(!sample_dir.join("cv.pdf").exists());
    assert!(!sample_dir.join("cover-letter.pdf").exists());

    // the watermark text is actually in the produced sample CV (non-vacuous)
    let bytes = std::fs::read(&cv_sample).unwrap();
    let text = pdf_extract::extract_text_from_mem(&bytes).expect("extract sample CV text");
    assert!(
        text.contains(aa_core::SAMPLE_WATERMARK),
        "the produced cv.SAMPLE.pdf must carry the SAMPLE watermark"
    );

    // ── 3. WITHOUT the flag (non-interactive) → export REFUSED, no files written ─────
    let blocked_dir = unique_dir("blocked");
    let jd2 = write_jd(&blocked_dir);
    let out2 = Command::new(bin())
        .current_dir(&root)
        .args(["--cv"])
        .arg(fixture())
        .args(["--jd"])
        .arg(&jd2)
        .args(["--out"])
        .arg(&blocked_dir)
        .arg("--non-interactive")
        .output()
        .expect("run applicant-advocate --non-interactive");
    assert!(
        !out2.status.success(),
        "a sample-requiring CV must NOT export without an explicit opt-in"
    );
    assert!(
        !blocked_dir.join("cv.SAMPLE.pdf").exists() && !blocked_dir.join("cv.pdf").exists(),
        "refused export must write NO CV file"
    );

    // cleanup
    let _ = std::fs::remove_dir_all(&sample_dir);
    let _ = std::fs::remove_dir_all(&blocked_dir);

    let elapsed = start.elapsed().as_secs_f64();
    let baseline_path = root.join("doc/perf/cli-ingestion-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "CLI ingestion STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] cli adaptive-ingestion sample-guard journey: {elapsed:.3}s (budget \
         {BUDGET_SECS}s, baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
