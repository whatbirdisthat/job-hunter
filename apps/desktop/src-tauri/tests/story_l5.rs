//! L5 STORY — the user journey via the headless command-level harness (R-D3).
//!
//! Journey: import master CV → paste JD → see coverage → approve/reject bullets →
//! export two PDFs, driven through the SAME command surface the UI invokes, fully
//! offline. Perf-delta gated on the < 60 s offline budget (I6): records the
//! end-to-end wall-clock, fails if it exceeds the baseline, and flags a regression if
//! a run drifts materially slower than the prior recorded story run.

// Shared perf-gate logic (Finding 3) — single source, reused across both L5 stories.
#[path = "../../../../crates/cvimport/tests/perf_gate.rs"]
mod perf_gate;

use aa_advocate::StubProvider;
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
/// Perf-delta tolerance vs the TRACKED story baseline (`doc/perf/...`). Independent of the
/// absolute budget (Finding 3): a >3× regression fails the delta arm even though it is far
/// under the 60 s budget.
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

    // ── perf-delta gate (I6, Finding 3) ─────────────────────────────────────────
    // TWO independent obligations, read from a TRACKED baseline (never self-overwritten):
    //   (a) absolute I6 budget   — `elapsed < BUDGET_SECS`
    //   (b) regression delta     — `elapsed <= baseline * DELTA_FACTOR` (can actually fire)
    // Shares the `perf_gate` helper with the cvimport L5; non-vacuity proven in perf_gate_l1.
    let baseline_path = root().join("doc/perf/desktop-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "import-to-export STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    // SAMPLE emission (kept) — observability, NOT a self-overwriting baseline write.
    eprintln!(
        "[L5 STORY perf] end-to-end: {elapsed:.3}s (budget {BUDGET_SECS}s, baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}

/// Item #3 L5 STORY — the SAME journey with the Applicant Advocate flag ON and the
/// deterministic stub provider (NO live model). Proves the advocate rewrite path is
/// inside the user journey and is perf-delta gated against its OWN tracked baseline.
#[test]
fn story_advocate_rewrite_perf_delta_gated() {
    let cv_json =
        std::fs::read_to_string(root().join("fixtures/personas/persona-001.cv.json")).unwrap();
    let raw_jd = "We are hiring a Senior Backend Engineer at Acme Group. You will own delivery end to end. \
                  Required: Strong TypeScript or Python; Stakeholder management; AWS or GCP experience. \
                  Nice to have: GraphQL; Fintech domain knowledge.";

    let start = Instant::now();

    // 1. import master CV (honest deterministic advocate provider injected)
    let mut s = Session::with_provider(Box::new(StubProvider::new()));
    s.import_master_cv(&cv_json).expect("import");

    // 2. paste JD → parse
    s.parse_job(raw_jd).expect("parse");

    // 3. user opts INTO the Applicant Advocate (the new review-step toggle)
    s.set_advocate_enabled(true);

    // 4. see coverage + review bullets
    let coverage = s.compute_coverage().expect("coverage");
    assert!(!coverage.must_have.is_empty());
    let view = s.tailored_view().expect("view");
    let top_id = view.cv.experience[0].achievements_tasks[0].id.clone();
    s.set_decision(&top_id, true);

    // 5. export two PDFs (advocate rewrite runs, then the ledger guard, then render)
    let (cv_pdf, letter_pdf, result) = s.export_application().expect("export");
    assert!(is_valid_pdf(&cv_pdf), "advocate cv.pdf is a valid PDF");
    assert!(
        is_valid_pdf(&letter_pdf),
        "advocate cover-letter.pdf is a valid PDF"
    );
    assert!(result.ai_used, "the journey ran with AI → ai_used true");
    assert_eq!(result.provider.as_deref(), Some("stub"));

    let elapsed = start.elapsed().as_secs_f64();

    // ── perf-delta gate (I6, Finding 3) — its OWN tracked baseline ───────────────
    let baseline_path = root().join("doc/perf/desktop-advocate-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "advocate-rewrite STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] advocate end-to-end: {elapsed:.3}s (budget {BUDGET_SECS}s, baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
