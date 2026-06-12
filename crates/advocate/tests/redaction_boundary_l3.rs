//! L3 — the redaction BOUNDARY (R-ADV-6, the PII firewall). Serialize a RewriteRequest
//! and assert its JSON keys are EXACTLY {evidence_id, evidence_text, requirement, kind} —
//! i.e. structurally NO master-CV / Person field can cross the boundary to a model. This
//! is the proof that redaction is a property of the *type*, not a fallible scrub.

use aa_advocate::redact;
use std::collections::BTreeSet;

fn achievement() -> aa_core::Achievement {
    aa_core::Achievement {
        id: "exp_2_1_b3".into(),
        description: "Reduced cloud spend 22% by right-sizing autoscaling groups".into(),
        emphasise: Some(true),
        tags: vec!["cost".into(), "cloud".into()],
        metrics: vec!["22%".into()],
        evidence_strength: Some(0.9),
    }
}

#[test]
fn outbound_payload_has_no_master_cv_fields() {
    let req = redact(&achievement(), "cloud cost optimisation");
    let value: serde_json::Value = serde_json::to_value(&req).unwrap();
    let obj = value
        .as_object()
        .expect("request serializes to a JSON object");

    let keys: BTreeSet<&str> = obj.keys().map(|s| s.as_str()).collect();
    let expected: BTreeSet<&str> = ["evidence_id", "evidence_text", "requirement", "kind"]
        .into_iter()
        .collect();

    assert_eq!(
        keys, expected,
        "outbound RewriteRequest must carry EXACTLY {{evidence_id, evidence_text, requirement, kind}} \
         — any extra key is a PII-firewall breach; got {keys:?}"
    );

    // Spot-check that NO master-CV/Person key name leaked in (belt-and-braces against a
    // future field being added without updating the boundary test).
    for forbidden in [
        "name",
        "email",
        "phone",
        "linkedin",
        "github",
        "person",
        "experience",
        "schemaVersion",
        "metrics",
        "tags",
        "emphasise",
        "evidenceStrength",
    ] {
        assert!(
            !obj.contains_key(forbidden),
            "forbidden key `{forbidden}` crossed the redaction boundary"
        );
    }
}
