//! L5 STORY (item #6) — the template→ATS→keyword journey via the headless command
//! surface (R-D3), fully offline.
//!
//! Journey: import persona-001 → parse a synthetic JD → pick the Compact template →
//! export (a valid PDF) → compute `ats_report(Compact, view)` and assert the
//! column-reliance check PASSES → compute `keyword_coverage(view, job)` and assert a
//! known must-have keyword is FOUND with non-empty evidence AND a known-absent keyword
//! is MISSING. Perf-delta gated on the < 60 s offline budget (I6) against its OWN
//! tracked baseline.

// Shared perf-gate logic — single source, reused across all L5 stories.
#[path = "../../../../crates/cvimport/tests/perf_gate.rs"]
mod perf_gate;

use aa_core::{AtsCheckId, AtsStatus, CvTemplate};
use aa_desktop::Session;
use std::time::Instant;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

const BUDGET_SECS: f64 = 60.0;
const DELTA_FACTOR: f64 = 3.0;

#[test]
fn story_template_ats_keyword_perf_delta_gated() {
    let cv_json =
        std::fs::read_to_string(root().join("fixtures/personas/persona-001.cv.json")).unwrap();
    // A synthetic JD whose requirements we can assert against persona-001: "caching"
    // (a must-have that surfaces via exp_1_0_b0) is FOUND; "Cobol" (absent) is MISSING.
    let raw_jd = "We are hiring a Senior Backend Engineer at Acme Group. \
                  Required: caching; Cobol. Nice to have: Mentored.";

    let start = Instant::now();

    // 1. import master CV
    let mut s = Session::new();
    s.import_master_cv(&cv_json).expect("import");

    // 2. paste JD → parse
    s.parse_job(raw_jd).expect("parse");

    // 3. user picks the Compact template + exports (valid PDF)
    let view = s.tailored_view().expect("view");
    let top_id = view.cv.experience[0].achievements_tasks[0].id.clone();
    s.set_decision(&top_id, true);
    let (cv_pdf, letter_pdf, result) = s
        .export_application_with(CvTemplate::Compact)
        .expect("compact export");
    assert!(aa_core::is_valid_pdf(&cv_pdf), "compact cv.pdf is valid");
    assert!(aa_core::is_valid_pdf(&letter_pdf), "letter.pdf is valid");
    assert!(result.cv_pdf_len > 0);

    // 4. ATS report for Compact → column-reliance PASSES (single-column, R-ATS-3)
    let ats = s.ats_report("compact").expect("ats");
    assert_eq!(
        ats.check(AtsCheckId::ColumnReliance).unwrap().status,
        AtsStatus::Pass,
        "Compact is single-column → column-reliance PASS"
    );

    // 5. keyword coverage → a known must-have FOUND with non-empty evidence + a
    //    known-absent must-have MISSING.
    let kw = s.keyword_coverage().expect("keyword");
    let caching = kw
        .found
        .iter()
        .find(|h| h.keyword == "caching")
        .expect("caching is a FOUND must-have");
    assert!(
        !caching.evidence_ids.is_empty(),
        "caching surfaces with evidence"
    );
    assert!(
        kw.missing.iter().any(|h| h.keyword == "Cobol"),
        "Cobol is a MISSING must-have"
    );

    let elapsed = start.elapsed().as_secs_f64();

    // ── perf-delta gate (I6) — its OWN tracked baseline ──────────────────────────
    let baseline_path = root().join("doc/perf/desktop-templates-ats-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "templates-ats-keyword STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] templates-ats end-to-end: {elapsed:.3}s (budget {BUDGET_SECS}s, baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
