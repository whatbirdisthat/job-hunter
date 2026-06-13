//! L3 — boundary (output ↔ schema). The imported MasterCv's serialized JSON is run
//! through the existing `tools/fake-data/validate.js` and MUST validate against
//! `doc/schemas/master-cv.schema.json` (R-CVI-7). One source of truth for "valid
//! master CV", reused from slice 1 (R-D1/R-D2 spirit: reuse, don't re-author).

mod support;

use aa_cvimport::{import_cv_json, import_resume, ResumeKind};
use std::io::Write;
use std::time::Instant;

use std::sync::atomic::{AtomicU64, Ordering};
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Validate a master-CV-shaped JSON string with the existing Node validator. Each
/// call writes a UNIQUE temp file (pid + atomic seq) so concurrent test threads
/// under `cargo test`/`llvm-cov` never race on the same path.
fn validate_with_node(json: &str) -> (bool, String) {
    let root = support::repo_root();
    let tmp = root.join(format!(
        "cvimport-boundary-{}-{}.cv.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    {
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }
    let out = std::process::Command::new("node")
        .arg(root.join("tools/fake-data/validate.js"))
        .arg(&tmp)
        .output()
        .expect("node validator runs");
    let _ = std::fs::remove_file(&tmp);
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn imported_docx_output_validates_against_master_cv_schema() {
    // R-CVI-7 — the DOCX import path produces schema-valid output.
    let bytes = support::synth_persona_docx("persona-001.cv.json");
    let cv = import_resume(&bytes, ResumeKind::Docx).unwrap();
    let json = cv.to_json().unwrap();
    let t0 = Instant::now();
    let (ok, msg) = validate_with_node(&json);
    eprintln!("[L3 perf] validate.js round-trip: {:?}", t0.elapsed());
    assert!(ok, "imported DOCX master CV must validate: {msg}");
}

#[test]
fn imported_pdf_output_validates_against_master_cv_schema() {
    // R-CVI-7 — the PDF import path also produces schema-valid output (even with R3b).
    let bytes = support::render_persona_pdf("persona-001.cv.json");
    let cv = import_resume(&bytes, ResumeKind::Pdf).unwrap();
    let (ok, msg) = validate_with_node(&cv.to_json().unwrap());
    assert!(ok, "imported PDF master CV must validate: {msg}");
}

#[test]
fn mined_dwcv_validates() {
    // R-INGEST-10 — the adaptive JSON miner's output for a DW_CV-shaped value (item 8a)
    // validates against the master-cv schema. This fixture carries full experiences
    // (jobTitle+businessName+startDate), so it satisfies validate.js's non-empty rule
    // (DISCUSS-8a-2 / R-INGEST-13). Reuses the SAME `validate_with_node` harness.
    let v = support::load_json("dwcv_shaped.json");
    let cv = import_cv_json(&v).unwrap();
    let (ok, msg) = validate_with_node(&cv.to_json().unwrap());
    assert!(ok, "mined DW_CV-shaped master CV must validate: {msg}");
}

#[test]
fn mined_json_resume_validates() {
    // R-INGEST-10 — the JSON-Resume-shaped value also mines to schema-valid output.
    let v = support::load_json("json_resume_shaped.json");
    let cv = import_cv_json(&v).unwrap();
    let (ok, msg) = validate_with_node(&cv.to_json().unwrap());
    assert!(ok, "mined JSON-Resume master CV must validate: {msg}");
}

#[test]
fn all_personas_import_to_schema_valid_docx() {
    // R-CVI-7 — robustness across all four personas (the test oracle).
    for name in [
        "persona-001.cv.json",
        "persona-002.cv.json",
        "persona-003.cv.json",
        "persona-004.cv.json",
    ] {
        let bytes = support::synth_persona_docx(name);
        let cv = import_resume(&bytes, ResumeKind::Docx).unwrap();
        let (ok, msg) = validate_with_node(&cv.to_json().unwrap());
        assert!(ok, "{name} imported DOCX must validate: {msg}");
    }
}
