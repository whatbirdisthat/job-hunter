//! L4 — system/integration of the two public miner fns TOGETHER (item 8a): the exact
//! `import_cv_json` → `completeness` pair that item 8b's CLI will drive. R-INGEST-4/11/12.

mod support;

use aa_cvimport::{completeness, import_cv_json};
use support::load_json;

#[test]
fn multi_role_arrays_reports_ignored() {
    // R-INGEST-4 / R-INGEST-12 — `experience` wins; `work` and `positions` are NAMED in
    // ignored_role_arrays (no silent merge). The miner returns the ignored names; the
    // caller threads them into `completeness`.
    let v = load_json("multi_role_arrays.json");
    let cv = import_cv_json(&v).unwrap();
    // experiences come only from experience[] (one element)
    assert_eq!(cv.experience.len(), 1);
    assert_eq!(cv.experience[0].job_title, "Lead Engineer");
    assert_eq!(cv.experience[0].business_name, "Northwind Labs");

    // The miner exposes ignored role-array names via the completeness path. Item 8b's
    // CLI mines, then asks the miner for the ignored names, then builds the report.
    let ignored = aa_cvimport::ignored_role_arrays(&v);
    assert!(ignored.contains(&"work".to_string()));
    assert!(ignored.contains(&"positions".to_string()));

    let report = completeness(&cv, &ignored);
    assert!(report.ignored_role_arrays.contains(&"work".to_string()));
    assert!(report
        .ignored_role_arrays
        .contains(&"positions".to_string()));
    // experience present (jobTitle+businessName), skills present, name present, but no
    // achievements on the winning element's sibling → check the flags honestly:
    assert!(!report.missing_experience);
    assert!(!report.missing_person_name);
    assert!(!report.missing_skill);
}

#[test]
fn minimal_flags_missing_experience_and_achievement() {
    // R-INGEST-11 — the sparse value: name + skill present; experience + achievement missing.
    let v = load_json("minimal.json");
    let cv = import_cv_json(&v).unwrap();
    let report = completeness(&cv, &[]);
    assert!(!report.missing_person_name);
    assert!(!report.missing_skill);
    assert!(report.missing_experience);
    assert!(report.missing_achievement);
    assert!(report.ignored_role_arrays.is_empty());
    assert!(!report.is_complete());
}

#[test]
fn complete_fixture_is_complete() {
    // Non-vacuous twin: a fully-populated value → is_complete() == true (all four
    // missing_* false). Proves the gate can report success, not only failure.
    let v = load_json("dwcv_shaped.json");
    let cv = import_cv_json(&v).unwrap();
    let report = completeness(&cv, &aa_cvimport::ignored_role_arrays(&v));
    assert!(!report.missing_person_name);
    assert!(!report.missing_experience);
    assert!(!report.missing_achievement);
    assert!(!report.missing_skill);
    assert!(report.is_complete());
}
