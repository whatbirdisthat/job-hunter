//! L5 STORY — the adaptive JSON-miner user journey, end to end, on the DW_CV-shaped
//! fixture (item 8a): mine → completeness → schema-validate, asserting the contact
//! block AND real proficiencies survived AND the report is complete. This is the
//! motivating regression as a full journey.
//!
//! Perf-delta gated via the shared `perf_gate.rs` (`#[path]`-include, same as
//! `story_l5.rs`) against a NEW TRACKED baseline `doc/perf/cvimport-jsonmine-story-baseline.txt`
//! (NOT the import-story baseline — a pure in-memory JSON walk is orders of magnitude
//! faster than the PDF/DOCX render+extract journey, so sharing would make the delta vacuous).

mod support;

#[path = "perf_gate.rs"]
mod perf_gate;

use aa_cvimport::{completeness, ignored_role_arrays, import_cv_json};
use std::io::Write;
use std::time::Instant;

/// Absolute budget: the pure JSON walk is trivially under the < 60 s journey budget (I6).
const BUDGET_SECS: f64 = 60.0;
/// Perf-delta tolerance vs the TRACKED json-mine-story baseline. A >3× regression fails
/// the delta arm even though it is far under the 60 s budget.
const DELTA_FACTOR: f64 = 3.0;

use std::sync::atomic::{AtomicU64, Ordering};
static SEQ: AtomicU64 = AtomicU64::new(0);

fn validates(json: &str) -> bool {
    let root = support::repo_root();
    let tmp = root.join(format!(
        "cvimport-jsonmine-story-{}-{}.cv.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(json.as_bytes())
        .unwrap();
    let ok = std::process::Command::new("node")
        .arg(root.join("tools/fake-data/validate.js"))
        .arg(&tmp)
        .status()
        .expect("node runs")
        .success();
    let _ = std::fs::remove_file(&tmp);
    ok
}

#[test]
fn story_json_mine_round_trip_perf_delta_gated() {
    let start = Instant::now();

    let v = support::load_json("dwcv_shaped.json");

    // ── mine ───────────────────────────────────────────────────────────────────
    let cv = import_cv_json(&v).expect("mine DW_CV-shaped value");

    // contact block survived (the regression)
    assert_eq!(cv.person.name.as_deref(), Some("Dana Wexford"));
    assert_eq!(cv.person.email.as_deref(), Some("dana.wexford@example.com"));
    assert!(cv.person.linkedin.is_some());
    assert!(cv.person.github.is_some());
    assert!(cv.person.website.is_some());

    // real proficiencies survived (not forced to default 3)
    let rust = cv
        .programming_languages
        .iter()
        .find(|s| s.name == "Rust")
        .expect("Rust skill present");
    assert_eq!(rust.proficiency, 5, "real proficiency preserved");

    // ── completeness ─────────────────────────────────────────────────────────────
    let report = completeness(&cv, &ignored_role_arrays(&v));
    assert!(
        report.is_complete(),
        "the DW_CV journey yields a complete report"
    );

    // ── schema-validate ───────────────────────────────────────────────────────────
    assert!(
        validates(&cv.to_json().unwrap()),
        "mined master CV is schema-valid"
    );

    let elapsed = start.elapsed().as_secs_f64();

    // ── perf-delta gate (I6) — TWO independent obligations, TRACKED baseline ───────
    let baseline_path = support::repo_root().join("doc/perf/cvimport-jsonmine-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "JSON-mine STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] json-mine round-trip: {elapsed:.3}s (budget {BUDGET_SECS}s, \
         baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
