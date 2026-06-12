//! L2 — public-surface (module) tests for the advocate crate. NO network (stub only).
//! Exercises the provider trait via `StubProvider` for both rewrite kinds and the
//! default-disabled config (R-ADV-5, R-ADV-11).

use aa_advocate::{
    redact, redact_kind, AdvocateConfig, AdvocateProvider, RewriteKind, StubProvider,
};

fn achievement(id: &str, desc: &str) -> aa_core::Achievement {
    aa_core::Achievement {
        id: id.into(),
        description: desc.into(),
        emphasise: None,
        tags: vec![],
        metrics: vec![],
        evidence_strength: None,
    }
}

#[test]
fn stub_rewrites_a_cv_bullet() {
    let req = redact(
        &achievement("e0_b0", "Led a migration to event sourcing"),
        "events",
    );
    let resp = StubProvider::new().rewrite(&req).unwrap();
    assert_eq!(resp.cited_evidence_id, "e0_b0");
    assert!(resp.rewritten_text.contains("event sourcing"));
}

#[test]
fn stub_rewrites_a_cover_letter_strength() {
    let req = redact_kind(
        &achievement("e1_b2", "Mentored five engineers to senior"),
        "mentoring",
        RewriteKind::CoverLetterStrength,
    );
    let resp = StubProvider::new().rewrite(&req).unwrap();
    assert_eq!(resp.cited_evidence_id, "e1_b2");
    assert!(resp.rewritten_text.contains("Mentored five engineers"));
    assert_eq!(req.kind, RewriteKind::CoverLetterStrength);
}

#[test]
fn config_disabled_is_default() {
    // R-ADV-11: the public default is OFF.
    assert!(!AdvocateConfig::default().enabled);
    // and it round-trips as camelCase JSON for the command boundary
    let json = serde_json::to_string(&AdvocateConfig::default()).unwrap();
    assert_eq!(json, r#"{"enabled":false}"#);
}

#[test]
fn provider_name_is_surfaced() {
    assert_eq!(StubProvider::new().name(), "stub");
    assert_eq!(StubProvider::fabricating().name(), "stub");
}
