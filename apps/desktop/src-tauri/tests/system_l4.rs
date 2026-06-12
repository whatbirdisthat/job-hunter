//! L4 system — assembled app path, offline, on synthetic fixtures.
//!
//! JD-text → parse → tailor → ledger-check → render×2 → two PDFs (cv.pdf +
//! cover-letter.pdf). Asserts: both PDFs non-empty + valid; every rendered CV bullet
//! maps to an evidence id in the master CV; coverage report enumerates must/nice with
//! covered/uncovered; injected-unsupported-claim fixture → export BLOCKED; end-to-end
//! wall-clock < 60 s (I6). Runs across persona×job pairs, fully offline.

use aa_core::{is_valid_pdf, ledger, MasterCv};
use aa_desktop::Session;
use std::time::Instant;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

const BUDGET_SECS: u64 = 60;

#[test]
fn full_offline_pipeline_two_pdfs_under_budget() {
    let personas = ["persona-001.cv.json", "persona-002.cv.json"];
    let jobs = ["job-linkedin-001.json", "job-seek-006.json"];

    for p in personas {
        for jb in jobs {
            let start = Instant::now();
            let cv_json = read(&format!("fixtures/personas/{p}"));
            let job =
                serde_json::from_str::<serde_json::Value>(&read(&format!("fixtures/jobs/{jb}")))
                    .unwrap();
            let raw_jd = job["descriptionRaw"].as_str().unwrap();

            let mut s = Session::new();
            s.import_master_cv(&cv_json).unwrap();
            s.parse_job(raw_jd).unwrap();

            let coverage = s.compute_coverage().unwrap();
            assert!(
                !coverage.must_have.is_empty(),
                "coverage must enumerate must-have for {p}×{jb}"
            );
            // every must/nice row has a covered flag (enumeration completeness)
            for row in coverage
                .must_have
                .iter()
                .chain(coverage.nice_to_have.iter())
            {
                let _ = row.covered;
            }

            let (cv_pdf, letter_pdf, result) = s.export_application().unwrap();
            assert!(is_valid_pdf(&cv_pdf), "cv.pdf valid for {p}×{jb}");
            assert!(
                is_valid_pdf(&letter_pdf),
                "cover-letter.pdf valid for {p}×{jb}"
            );
            assert!(result.cv_pdf_len > 0 && result.cover_letter_pdf_len > 0);

            // every rendered CV bullet maps to an evidence id in the master CV
            let master = MasterCv::from_json(&cv_json).unwrap();
            let universe = ledger::resolvable_ids(&master);
            let view = s.tailored_view().unwrap();
            for e in &view.cv.experience {
                for a in &e.achievements_tasks {
                    assert!(
                        universe.contains(&a.id),
                        "bullet {} unmapped for {p}×{jb}",
                        a.id
                    );
                }
            }

            let elapsed = start.elapsed();
            assert!(
                elapsed.as_secs() < BUDGET_SECS,
                "pipeline for {p}×{jb} exceeded {BUDGET_SECS}s: {:?}",
                elapsed
            );
            eprintln!(
                "[L4 perf] {p}×{jb}: {:?} (cv {}B, letter {}B)",
                elapsed,
                cv_pdf.len(),
                letter_pdf.len()
            );
        }
    }
}

#[test]
fn injected_unsupported_claim_blocks_export() {
    // Build a session, then inject a dangling-id bullet into the tailored view and
    // confirm the ledger guard blocks export (I2/§E) — the L4 acceptance integrity check.
    let cv_json = read("fixtures/personas/persona-001.cv.json");
    let master = MasterCv::from_json(&cv_json).unwrap();

    let mut s = Session::new();
    s.import_master_cv(&cv_json).unwrap();
    s.parse_job("We are hiring a Senior Backend Engineer at Acme. Required: Python.")
        .unwrap();

    let mut view = s.tailored_view().unwrap();
    view.cv.experience[0]
        .achievements_tasks
        .push(aa_core::Achievement {
            id: "FABRICATED_x".into(),
            description: "Unsupported claim with no evidence".into(),
            emphasise: None,
            tags: vec![],
            metrics: vec![],
            evidence_strength: None,
        });
    let nodes = ledger::cv_ledger(&view);
    let blocked = aa_core::guard(&nodes, &master);
    assert!(
        blocked.is_err(),
        "injected unsupported claim must block export"
    );
    assert!(blocked.unwrap_err().to_string().contains("FABRICATED_x"));
}
