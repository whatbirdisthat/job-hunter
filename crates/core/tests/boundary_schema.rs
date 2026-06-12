//! L3 boundary — the serialized tailored-view JSON conforms to master-cv.schema.json.
//!
//! §H: the tailored view IS a filtered/reordered master-CV document, so it must pass
//! the SAME schema validator the source fixtures pass (tools/fake-data/validate.js).
//! This is the seam that lets `classic.typ` render the view unchanged. We reuse the
//! existing Node validator (R-D1/R-D2 spirit: reuse, don't re-author).

use aa_core::{tailor, MasterCv, NormalizedJob, Requirements, DEFAULT_TOP_N};
use std::io::Write;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn master(name: &str) -> MasterCv {
    let p = repo_root().join("fixtures/personas").join(name);
    MasterCv::from_json(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn job() -> NormalizedJob {
    NormalizedJob {
        title: "Senior Backend Engineer".into(),
        company: "Acme".into(),
        location: String::new(),
        responsibilities: vec![],
        requirements: Requirements {
            must_have: vec!["caching".into(), "Python".into()],
            nice_to_have: vec!["Mentored".into()],
        },
        keywords: vec![],
    }
}

use std::sync::atomic::{AtomicU64, Ordering};
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Validate a master-CV-shaped JSON string with the existing Node validator. Each
/// call uses a UNIQUE temp file (pid + atomic seq) so concurrent test threads (esp.
/// under `cargo llvm-cov`, which changes timing) never race on the same path.
fn validate_with_node(json: &str) -> (bool, String) {
    let root = repo_root();
    let tmp = root.join(format!(
        "aa-view-test-{}-{}.cv.json",
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
fn tailored_view_conforms_to_master_cv_schema_all_personas() {
    for name in [
        "persona-001.cv.json",
        "persona-002.cv.json",
        "persona-003.cv.json",
        "persona-004.cv.json",
    ] {
        let m = master(name);
        let view = tailor(&m, &job(), DEFAULT_TOP_N);
        let json = view.cv.to_json().unwrap();
        let (ok, msg) = validate_with_node(&json);
        assert!(ok, "tailored view for {name} must validate: {msg}");
    }
}

#[test]
fn source_fixture_validates_as_control() {
    // control: the untouched source persona validates (proves the validator is wired)
    let m = master("persona-001.cv.json");
    let (ok, msg) = validate_with_node(&m.to_json().unwrap());
    assert!(ok, "source fixture must validate (control): {msg}");
}
