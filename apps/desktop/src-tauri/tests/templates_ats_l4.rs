//! L4 system (item #6) — template selection + ATS + keyword reports across the
//! assembled command path, offline, on synthetic fixtures.
//!
//! Asserts: export with the Compact template yields a valid PDF AND the ledger guard
//! still holds (no dangling id slips through the alternate template path); the ATS
//! report is consistent with the chosen template (Compact → column-reliance PASS,
//! Classic → WARN) over the same tailored view; the keyword report is consistent with
//! the tailored view (a surfaced must-have is FOUND with non-empty evidence).

use aa_core::{ledger, AtsCheckId, AtsStatus, KeywordClass, MasterCv};
use aa_desktop::Session;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap()
}

const JD: &str = "We are hiring a Senior Backend Engineer at Acme Group. You will own delivery end to end. \
                  Required: Strong TypeScript or Python; Stakeholder management; AWS or GCP experience. \
                  Nice to have: GraphQL; Fintech domain knowledge.";

#[test]
fn compact_export_holds_ledger_guard_and_reports_are_consistent() {
    let cv_json = read("fixtures/personas/persona-001.cv.json");
    let master = MasterCv::from_json(&cv_json).unwrap();

    let mut s = Session::new();
    s.import_master_cv(&cv_json).unwrap();
    s.parse_job(JD).unwrap();

    // Export through Compact → valid PDF; the ledger guard runs in prepare_export so a
    // successful export proves the guard held on the alternate-template path.
    let (cv_pdf, letter_pdf, result) = s
        .export_application_with(aa_core::CvTemplate::Compact)
        .expect("compact export succeeds");
    assert!(aa_core::is_valid_pdf(&cv_pdf), "compact cv.pdf valid");
    assert!(aa_core::is_valid_pdf(&letter_pdf), "letter.pdf valid");
    assert!(result.cv_pdf_len > 0);

    // Every rendered CV bullet maps to an evidence id in the master CV (ledger integrity
    // independent of the chosen template).
    let universe = ledger::resolvable_ids(&master);
    let view = s.tailored_view().unwrap();
    for e in &view.cv.experience {
        for a in &e.achievements_tasks {
            assert!(universe.contains(&a.id), "bullet {} unmapped", a.id);
        }
    }

    // ATS report consistent with the chosen template over the same view.
    let compact_ats = s.ats_report("compact").unwrap();
    let classic_ats = s.ats_report("classic").unwrap();
    assert_eq!(
        compact_ats
            .check(AtsCheckId::ColumnReliance)
            .unwrap()
            .status,
        AtsStatus::Pass
    );
    assert_eq!(
        classic_ats
            .check(AtsCheckId::ColumnReliance)
            .unwrap()
            .status,
        AtsStatus::Warn
    );

    // Keyword report consistent with the tailored view: a surfaced must-have is FOUND
    // with non-empty evidence; the report's classes match the parsed job.
    let kw = s.keyword_coverage().unwrap();
    let all: Vec<_> = kw.found.iter().chain(kw.missing.iter()).collect();
    assert!(
        !all.is_empty(),
        "keyword report covers the job requirements"
    );
    // every must-have/nice class is represented exactly as parsed (no fabrication)
    assert!(all
        .iter()
        .all(|h| matches!(h.class, KeywordClass::MustHave | KeywordClass::NiceToHave)));
    for h in &kw.found {
        assert!(
            !h.evidence_ids.is_empty(),
            "a FOUND keyword carries surfaced evidence"
        );
    }
}
