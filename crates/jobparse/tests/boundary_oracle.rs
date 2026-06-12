//! L3 boundary — jobparse output equals the fixture §F oracle across all 6 jobs.
//!
//! Each job fixture carries `descriptionRaw` (parser input) AND structured
//! `requirements.mustHave[]` / `niceToHave[]` (the §F expected-output oracle). The
//! parser's classification MUST reproduce the oracle exactly on the synthetic
//! fixtures (where cues are clean — §F: 100% classification). Also asserts the
//! Normalized Job validates against doc/schemas/normalized-job.schema.json (R-D1).

use aa_jobparse::{parse, to_json};
use serde_json::Value;

const FIXTURES: &[&str] = &[
    "job-linkedin-001",
    "job-linkedin-003",
    "job-linkedin-005",
    "job-seek-002",
    "job-seek-004",
    "job-seek-006",
];

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/../../fixtures/jobs/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn as_str_vec(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn parser_reproduces_must_nice_oracle_for_all_six_jobs() {
    for name in FIXTURES {
        let fx = fixture(name);
        let raw = fx["descriptionRaw"].as_str().unwrap();
        let oracle_must = as_str_vec(&fx["requirements"]["mustHave"]);
        let oracle_nice = as_str_vec(&fx["requirements"]["niceToHave"]);

        let parsed = parse(raw);
        assert_eq!(
            parsed.requirements.must_have, oracle_must,
            "must-have mismatch for {name}: got {:?}",
            parsed.requirements.must_have
        );
        assert_eq!(
            parsed.requirements.nice_to_have, oracle_nice,
            "nice-to-have mismatch for {name}: got {:?}",
            parsed.requirements.nice_to_have
        );
    }
}

#[test]
fn parser_extracts_title_for_all_six_jobs() {
    for name in FIXTURES {
        let fx = fixture(name);
        let raw = fx["descriptionRaw"].as_str().unwrap();
        let parsed = parse(raw);
        assert_eq!(
            parsed.title,
            fx["title"].as_str().unwrap(),
            "title for {name}"
        );
    }
}

#[test]
fn normalized_job_round_trips_losslessly() {
    for name in FIXTURES {
        let fx = fixture(name);
        let raw = fx["descriptionRaw"].as_str().unwrap();
        let parsed = parse(raw);
        let json = to_json(&parsed).unwrap();
        let reparsed: aa_jobparse::NormalizedJob = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, reparsed, "round trip for {name}");
    }
}

// ── R-D1: validate the Normalized Job against normalized-job.schema.json ─────────
// Zero-dependency structural validation of the required shape (mirrors the spirit of
// tools/fake-data/validate.js: assert required keys + types without a schema engine).
#[test]
fn normalized_job_conforms_to_schema_shape() {
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/../../doc/schemas/normalized-job.schema.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    // required top-level keys per the schema
    let required: Vec<String> = as_str_vec(&schema["required"]);
    assert!(required.contains(&"requirements".to_string()));

    for name in FIXTURES {
        let fx = fixture(name);
        let parsed = parse(fx["descriptionRaw"].as_str().unwrap());
        let v: Value = serde_json::from_str(&to_json(&parsed).unwrap()).unwrap();
        // every required key present
        for k in &required {
            assert!(v.get(k).is_some(), "{name}: missing required key {k}");
        }
        // requirements has mustHave + niceToHave arrays of strings
        assert!(v["requirements"]["mustHave"].is_array(), "{name}");
        assert!(v["requirements"]["niceToHave"].is_array(), "{name}");
        // no additionalProperties beyond the schema's declared set
        let allowed: std::collections::HashSet<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for k in v.as_object().unwrap().keys() {
            assert!(
                allowed.contains(k),
                "{name}: unexpected key {k} not in schema"
            );
        }
    }
}
