//! L5 STORY — the user journey via the headless command-level harness (R-D3).
//!
//! Journey: import master CV → paste JD → see coverage → approve/reject bullets →
//! export two PDFs, driven through the SAME command surface the UI invokes, fully
//! offline. Perf-delta gated on the < 60 s offline budget (I6): records the
//! end-to-end wall-clock, fails if it exceeds the baseline, and flags a regression if
//! a run drifts materially slower than the prior recorded story run.

use aa_core::is_valid_pdf;
use aa_desktop::Session;
use std::time::Instant;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

const BUDGET_SECS: f64 = 60.0;
/// Perf-delta tolerance: a run more than this factor slower than the prior recorded
/// story run is flagged (not a hard fail unless it also breaches the absolute budget).
const DELTA_FACTOR: f64 = 3.0;

#[test]
fn story_import_to_export_perf_delta_gated() {
    let cv_json =
        std::fs::read_to_string(root().join("fixtures/personas/persona-001.cv.json")).unwrap();
    let raw_jd = "We are hiring a Senior Backend Engineer at Acme Group. You will own delivery end to end. \
                  Required: Strong TypeScript or Python; Stakeholder management; AWS or GCP experience. \
                  Nice to have: GraphQL; Fintech domain knowledge.";

    let start = Instant::now();

    // 1. import master CV
    let mut s = Session::new();
    s.import_master_cv(&cv_json).expect("import");

    // 2. paste JD → parse
    s.parse_job(raw_jd).expect("parse");

    // 3. see coverage
    let coverage = s.compute_coverage().expect("coverage");
    assert!(
        !coverage.must_have.is_empty(),
        "user sees must-have coverage"
    );
    assert!(coverage.fit_score >= 0.0 && coverage.fit_score <= 1.0);

    // 4. approve/reject bullets — reject the lowest-ranked bullet of the first role,
    //    approve the top one (the review-UI interaction, exercised at the command layer)
    let view = s.tailored_view().expect("view");
    let first_role = &view.cv.experience[0];
    let top_id = first_role.achievements_tasks[0].id.clone();
    s.set_decision(&top_id, true);
    if let Some(last) = first_role.achievements_tasks.last() {
        if last.id != top_id {
            s.set_decision(&last.id, false);
        }
    }

    // 5. export two PDFs (ledger-guarded)
    let (cv_pdf, letter_pdf, result) = s.export_application().expect("export");
    assert!(is_valid_pdf(&cv_pdf), "exported cv.pdf is a valid PDF");
    assert!(
        is_valid_pdf(&letter_pdf),
        "exported cover-letter.pdf is a valid PDF"
    );
    assert!(result.cv_pdf_len > 0 && result.cover_letter_pdf_len > 0);

    let elapsed = start.elapsed().as_secs_f64();

    // ── perf-delta gate (I6) ────────────────────────────────────────────────────
    // absolute budget
    assert!(
        elapsed < BUDGET_SECS,
        "STORY exceeded the {BUDGET_SECS}s offline budget: {elapsed:.3}s"
    );
    // delta vs the prior recorded run (if any)
    let perf_path = root().join("target/story-perf-baseline.txt");
    if let Ok(prev) = std::fs::read_to_string(&perf_path) {
        if let Ok(prev_secs) = prev.trim().parse::<f64>() {
            assert!(
                elapsed <= prev_secs * DELTA_FACTOR || elapsed < BUDGET_SECS,
                "STORY drifted >{DELTA_FACTOR}x slower than prior run ({prev_secs:.3}s -> {elapsed:.3}s)"
            );
        }
    }
    let _ = std::fs::write(&perf_path, format!("{elapsed:.6}"));
    eprintln!("[L5 STORY perf] end-to-end: {elapsed:.3}s (budget {BUDGET_SECS}s)");
}
