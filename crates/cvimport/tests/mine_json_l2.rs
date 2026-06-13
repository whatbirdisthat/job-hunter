//! L2 — public surface of the adaptive JSON miner (item 8a). Drives `import_cv_json`
//! / `completeness` / `CompletenessReport` over the synthetic INPUT fixtures under
//! `tests/fixtures/json/`. The motivating regression: the DW_CV-shaped file recovers
//! the contact block AND real proficiencies (proving the miner does NOT route through
//! the text `Segments`/`map` path, which would drop both). R-INGEST-1/2/3/6/7/8/9/14.

mod support;

use aa_cvimport::{completeness, import_cv_json, ImportError};
use support::load_json;

#[test]
fn dwcv_recovers_name_and_contact_block() {
    // R-INGEST-1 / R-INGEST-9 — the PascalCase legacy shape recovers the whole contact
    // block (name/email/linkedin/github/website). This is the regression that motivates
    // the item: routing through Segments/map would have dropped every one of these.
    let cv = import_cv_json(&load_json("dwcv_shaped.json")).unwrap();
    assert_eq!(cv.person.name.as_deref(), Some("Dana Wexford"));
    assert_eq!(cv.person.email.as_deref(), Some("dana.wexford@example.com"));
    assert_eq!(
        cv.person.linkedin.as_deref(),
        Some("https://linkedin.example/in/dana-wexford")
    );
    assert_eq!(
        cv.person.github.as_deref(),
        Some("https://github.example/dwexford")
    );
    assert_eq!(
        cv.person.website.as_deref(),
        Some("https://dana.example.org")
    );
    assert_eq!(
        cv.person.professional_title.as_deref(),
        Some("Principal Platform Engineer")
    );
    // headline mirrors the professional title (as map.rs does)
    assert_eq!(cv.headline.as_deref(), Some("Principal Platform Engineer"));
}

#[test]
fn dwcv_preserves_real_proficiencies() {
    // R-INGEST-7 — the source proficiencies (5/4/3) survive; they are NOT forced to the
    // default 3 (which the text-map path would have done).
    let cv = import_cv_json(&load_json("dwcv_shaped.json")).unwrap();
    let by_name: std::collections::HashMap<&str, u8> = cv
        .programming_languages
        .iter()
        .map(|s| (s.name.as_str(), s.proficiency))
        .collect();
    assert_eq!(by_name.get("Rust"), Some(&5));
    assert_eq!(by_name.get("Go"), Some(&4));
    assert_eq!(by_name.get("Python"), Some(&3));
    // `Tools` bucket → tools_technologies, real level preserved
    assert_eq!(cv.tools_technologies.len(), 1);
    assert_eq!(cv.tools_technologies[0].name, "Kubernetes");
    assert_eq!(cv.tools_technologies[0].proficiency, 4);
}

#[test]
fn dwcv_contact_block_preserved_proves_not_segment_routed() {
    // R-INGEST-9 — a Segments/map route has no slots for email/linkedin/github/website,
    // so their presence proves the direct-build path was taken.
    let cv = import_cv_json(&load_json("dwcv_shaped.json")).unwrap();
    assert!(cv.person.email.is_some());
    assert!(cv.person.linkedin.is_some());
    assert!(cv.person.github.is_some());
    assert!(cv.person.website.is_some());
    // experience + achievements recovered; the \n-joined first block split into 3 bullets
    assert_eq!(cv.experience.len(), 2);
    assert_eq!(cv.experience[0].job_title, "Principal Platform Engineer");
    assert_eq!(cv.experience[0].business_name, "Harbour Compute");
    assert_eq!(cv.experience[0].achievements_tasks.len(), 3);
    assert_eq!(cv.experience[1].achievements_tasks.len(), 1);
    // synthetic ids
    assert_eq!(cv.experience[0].id, "imp_exp_0");
    assert_eq!(cv.experience[0].achievements_tasks[0].id, "imp_exp_0_b0");
    assert_eq!(cv.experience[1].id, "imp_exp_1");
}

#[test]
fn json_resume_recovers_from_basics() {
    // R-INGEST-2 — dedicated `basics` object is the person source.
    let cv = import_cv_json(&load_json("json_resume_shaped.json")).unwrap();
    assert_eq!(cv.person.name.as_deref(), Some("Rowan Adler"));
    assert_eq!(
        cv.person.professional_title.as_deref(),
        Some("Staff Software Engineer")
    );
    assert_eq!(
        cv.person.professional_description.as_deref(),
        Some("Distributed systems and developer tooling.")
    );
    assert_eq!(
        cv.person.email.as_deref(),
        Some("rowan.adler@job-hunter.example")
    );
}

#[test]
fn json_resume_work_position_highlights() {
    // R-INGEST-3 / R-INGEST-5 — work[] via position/name synonyms; highlights → bullets.
    let cv = import_cv_json(&load_json("json_resume_shaped.json")).unwrap();
    assert_eq!(cv.experience.len(), 2);
    assert_eq!(cv.experience[0].job_title, "Staff Software Engineer");
    assert_eq!(cv.experience[0].business_name, "Lindenbaum GmbH");
    assert_eq!(cv.experience[0].start_date, "2020-04");
    assert_eq!(cv.experience[0].end_date.as_deref(), Some("2024-09"));
    assert_eq!(cv.experience[0].achievements_tasks.len(), 2);
    assert_eq!(
        cv.experience[0].achievements_tasks[0].description,
        "Designed the event-sourcing core"
    );
    assert_eq!(cv.experience[1].achievements_tasks.len(), 1);
}

#[test]
fn numeric_dates_fixture_coerces() {
    // R-INGEST-6 — numeric startDate/endDate → integer strings.
    let cv = import_cv_json(&load_json("numeric_dates.json")).unwrap();
    assert_eq!(cv.experience.len(), 1);
    assert_eq!(cv.experience[0].start_date, "2019");
    assert_eq!(cv.experience[0].end_date.as_deref(), Some("2022"));
}

#[test]
fn empty_json_returns_err_empty() {
    // R-INGEST-14 — `{}` carries no recognisable content.
    let err = import_cv_json(&load_json("empty.json")).unwrap_err();
    assert!(matches!(err, ImportError::Empty));
    assert!(err.to_string().contains("no recognisable content"));
}

#[test]
fn import_is_deterministic_byte_identical() {
    // R-INGEST-8 — same value → byte-identical serialisation across runs.
    let v = load_json("dwcv_shaped.json");
    let a = import_cv_json(&v).unwrap().to_json().unwrap();
    let b = import_cv_json(&v).unwrap().to_json().unwrap();
    assert_eq!(a, b);
}

#[test]
fn minimal_is_valid_but_incomplete() {
    // R-INGEST-11 — sparse input still yields a valid (mostly-empty) MasterCv; the
    // completeness report flags the missing classes.
    let cv = import_cv_json(&load_json("minimal.json")).unwrap();
    assert_eq!(cv.person.name.as_deref(), Some("A. Tester"));
    assert_eq!(cv.skills.len(), 1);
    assert_eq!(cv.skills[0].name, "Rust");
    assert_eq!(cv.skills[0].proficiency, 3);
    let report = completeness(&cv, &[]);
    assert!(!report.missing_person_name);
    assert!(report.missing_experience);
    assert!(report.missing_achievement);
    assert!(!report.missing_skill);
    assert!(!report.is_complete());
}
