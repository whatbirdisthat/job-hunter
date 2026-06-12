//! aa-advocate — the Applicant Advocate LLM layer (item #3).
//!
//! Optional, feature-flagged, evidence-bounded rewrite/draft. The design wedge is
//! **structural redaction**: the ONLY value that crosses the process boundary to any
//! model is a [`RewriteRequest`], whose type has NO `Person` field — so a name, email,
//! phone, LinkedIn or GitHub handle *cannot* be carried out even by mistake. The model
//! must cite the evidence id it was given; the EXISTING `aa_core` ledger `guard` (run
//! against the IMMUTABLE master CV, not the tailored view) is the backstop that NAMES
//! and BLOCKS any fabricated/swapped id.
//!
//! Default cargo features compile the trait + a deterministic [`StubProvider`] ONLY,
//! with **no network dependency**. The live adapters ([`OllamaProvider`],
//! [`HttpKeyProvider`]) sit behind `--features live-http` and are never compiled by the
//! `--workspace` CI gate (`aa-desktop` depends on this crate with DEFAULT features), so
//! CI carries no network code and runs no live model.
//!
//! Crate graph: depends on `aa-core` ONLY (one-way, mirroring `cvimport`).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What kind of text the advocate is rewriting — a CV bullet or a cover-letter
/// strength paragraph. Threaded into the prompt so the template can shape tone, but it
/// carries no PII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RewriteKind {
    CvBullet,
    CoverLetterStrength,
}

/// The OUTBOUND request — the only bytes that leave the process to a model.
///
/// PII firewall (I-PII, R-ADV-3/R-ADV-6): this type has NO `aa_core::Person` field and
/// NO master-CV field. Its serialized JSON keys are EXACTLY
/// `{evidence_id, evidence_text, requirement, kind}`. Redaction is therefore a property
/// of the *type*, not of a fallible scrubbing pass — the boundary cannot be widened
/// without changing this struct (and the L3 boundary test that pins its keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewriteRequest {
    /// The evidence id of the achievement this rewrite is bound to. The model is asked
    /// to cite it back; a different id is treated as fabrication downstream.
    pub evidence_id: String,
    /// The achievement description text — the ONLY CV content that crosses the boundary.
    pub evidence_text: String,
    /// The job requirement to tailor toward (a must-have string, or the joined list).
    pub requirement: String,
    /// Bullet vs cover-letter strength paragraph.
    pub kind: RewriteKind,
}

/// The INBOUND response from a provider. `cited_evidence_id` is what the model claims
/// it based the rewrite on; the ledger guard checks it against the immutable master CV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RewriteResponse {
    pub rewritten_text: String,
    pub cited_evidence_id: String,
}

/// Typed advocate errors. Surfaced across the command boundary without panicking.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdvocateError {
    /// The flag was ON but the provider could not be reached (R-ADV-9). NOT a silent
    /// fallback — the user must know AI did not run.
    #[error("advocate provider unreachable: {0}")]
    Unreachable(String),
    /// The provider returned a malformed / unparseable response.
    #[error("advocate provider returned a malformed response: {0}")]
    Malformed(String),
    /// A BYO-key live endpoint was configured with a non-`https://` scheme (live-http
    /// adapters only). Rejected by construction so the bearer key + evidence can never
    /// be transmitted in cleartext (parse-don't-validate).
    #[error("advocate endpoint must use https (got: {0})")]
    InsecureEndpoint(String),
}

/// A provider turns a redacted [`RewriteRequest`] into a [`RewriteResponse`]. The stub
/// is always compiled; the live adapters are `live-http`-gated.
pub trait AdvocateProvider {
    fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError>;
    fn name(&self) -> &'static str;
}

/// Feature flag + provider selection. `Default::default().enabled == false` (I-FLAG,
/// R-ADV-11): the advocate is OFF until the user explicitly opts in. The derived
/// `Default` yields `enabled: false` (the `bool` default) — this is the disabled-by-
/// default guarantee, pinned by `config_disabled_is_default`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvocateConfig {
    pub enabled: bool,
}

/// Assemble the outbound prompt (R-ADV-4). The ONLY function that builds outbound bytes
/// from a request. It is a fixed template + the request's `evidence_text` + its
/// `requirement` — NOTHING else. No `Person` field is in scope here (the request type
/// has none), so this cannot leak PII.
pub fn build_prompt(req: &RewriteRequest) -> String {
    let kind = match req.kind {
        RewriteKind::CvBullet => "CV bullet point",
        RewriteKind::CoverLetterStrength => "cover-letter strength paragraph",
    };
    format!(
        "Rewrite the following {kind} to better address the requirement, using ONLY the \
         facts present in the evidence. Do not invent achievements, metrics, employers, or \
         dates. Keep it truthful and concise.\n\
         Requirement: {requirement}\n\
         Evidence: {evidence}\n",
        kind = kind,
        requirement = req.requirement,
        evidence = req.evidence_text,
    )
}

/// Build a [`RewriteRequest`] from an achievement + a requirement string (R-ADV-3).
///
/// This is the redaction step: it copies ONLY the achievement's `id` and `description`
/// (plus the caller-supplied requirement + kind). It takes an `&aa_core::Achievement`,
/// which carries no `Person` data; the resulting request type physically cannot hold
/// PII. Defaults to `CvBullet`; the cover-letter caller passes `CoverLetterStrength`.
///
/// **Accepted residual (R-ADV-RES-3) — the firewall is STRUCTURAL, not content-scrubbing.**
/// The `Person` block (name/contact) is blocked *by construction*: [`RewriteRequest`] has
/// no `Person` field, so it cannot cross the boundary even by mistake. But free-text PII a
/// user *pastes into* a bullet `description` (the `evidence_text`) is the user's own content
/// and is NOT scrubbed in this slice — it is carried verbatim into the prompt. A future slice
/// may add a free-text PII scrub/warn pass over `evidence_text`; until then this is a
/// documented, accepted risk.
pub fn redact(achievement: &aa_core::Achievement, requirement: &str) -> RewriteRequest {
    redact_kind(achievement, requirement, RewriteKind::CvBullet)
}

/// Redaction with an explicit kind (used for cover-letter strength paragraphs).
pub fn redact_kind(
    achievement: &aa_core::Achievement,
    requirement: &str,
    kind: RewriteKind,
) -> RewriteRequest {
    RewriteRequest {
        evidence_id: achievement.id.clone(),
        evidence_text: achievement.description.clone(),
        requirement: requirement.to_string(),
        kind,
    }
}

/// The deterministic, network-free [`AdvocateProvider`] used by every CI test (no live
/// model). It echoes a lightly-templated rewrite and — in the honest mode — cites back
/// the evidence id it was given. `fabricating()` is the adversarial fixture: it cites
/// an id that resolves NOWHERE in any master CV, so the ledger guard MUST block it.
#[derive(Debug, Clone, Default)]
pub struct StubProvider {
    /// When set, the stub cites this id instead of the request's evidence id (the
    /// adversarial / fabrication mode).
    fabricated_id: Option<String>,
}

impl StubProvider {
    /// An honest stub: cites back the request's own evidence id.
    pub fn new() -> Self {
        StubProvider {
            fabricated_id: None,
        }
    }

    /// The adversarial stub: cites a fixed id that exists in NO master CV
    /// (`FABRICATED_evidence_id`), so the export ledger guard NAMES and BLOCKS it.
    pub fn fabricating() -> Self {
        StubProvider {
            fabricated_id: Some("FABRICATED_evidence_id".to_string()),
        }
    }
}

impl AdvocateProvider for StubProvider {
    fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError> {
        let cited = self
            .fabricated_id
            .clone()
            .unwrap_or_else(|| req.evidence_id.clone());
        // deterministic, evidence-bounded rewrite: prefixes a tailoring lead-in, keeps
        // the evidence text verbatim (never invents). Same input → byte-identical output.
        let rewritten_text = format!("Demonstrated impact: {}", req.evidence_text);
        Ok(RewriteResponse {
            rewritten_text,
            cited_evidence_id: cited,
        })
    }

    fn name(&self) -> &'static str {
        "stub"
    }
}

#[cfg(feature = "live-http")]
mod live;
#[cfg(feature = "live-http")]
pub use live::{HttpKeyProvider, OllamaProvider};

#[cfg(test)]
mod tests {
    //! L1 — advocate-internal unit tests. NO network (stub only).
    use super::*;
    use aa_core::{MasterCv, Person};

    fn achievement() -> aa_core::Achievement {
        aa_core::Achievement {
            id: "exp_1_0_b0".into(),
            description: "Cut p99 API latency by 38% via a read-through cache".into(),
            emphasise: None,
            tags: vec![],
            metrics: vec!["38%".into()],
            evidence_strength: None,
        }
    }

    /// R-ADV-3 — the serialized request carries NONE of the Person PII substrings.
    #[test]
    fn redact_strips_all_person_pii() {
        // a Person FULL of PII; redact() takes an Achievement, so none of this can
        // structurally enter the request — we prove it by asserting absence in the JSON.
        let person = Person {
            name: Some("Devin Voss".into()),
            professional_title: Some("Staff Engineer".into()),
            professional_description: None,
            location: Some("Melbourne".into()),
            email: Some("devin@example.com".into()),
            phone: Some("+61 400 000 000".into()),
            linkedin: Some("linkedin.example/in/devinvoss".into()),
            github: Some("github.example/devinvoss".into()),
            website: None,
            image: None,
        };
        let req = redact(&achievement(), "caching");
        let json = serde_json::to_string(&req).unwrap();
        for pii in [
            person.name.as_deref().unwrap(),
            person.email.as_deref().unwrap(),
            person.phone.as_deref().unwrap(),
            person.linkedin.as_deref().unwrap(),
            person.github.as_deref().unwrap(),
        ] {
            assert!(
                !json.contains(pii),
                "outbound request leaked PII substring: {pii} in {json}"
            );
        }
        // and the evidence text + requirement ARE present (the legitimate payload)
        assert!(json.contains("p99 API latency"));
        assert!(json.contains("caching"));
    }

    /// R-ADV-4 — the prompt contains ONLY the evidence text + requirement (no PII).
    #[test]
    fn build_prompt_contains_only_evidence_and_requirement() {
        let req = redact(&achievement(), "distributed caching");
        let prompt = build_prompt(&req);
        assert!(prompt.contains("Cut p99 API latency by 38%"));
        assert!(prompt.contains("distributed caching"));
        // no PII could be present — the request has no Person field — but assert the
        // shape: the prompt is template + evidence + requirement, nothing resembling a CV id leak
        assert!(prompt.contains("Do not invent"));
        assert!(!prompt.contains("Devin Voss"));
        // the bullet kind is named in the template
        assert!(prompt.contains("CV bullet point"));
    }

    /// R-ADV-4 — the cover-letter-strength kind shapes the prompt's lead-in (covers the
    /// CoverLetterStrength arm of build_prompt).
    #[test]
    fn build_prompt_for_cover_letter_strength_kind() {
        let req = redact_kind(
            &achievement(),
            "leadership",
            RewriteKind::CoverLetterStrength,
        );
        let prompt = build_prompt(&req);
        assert!(prompt.contains("cover-letter strength paragraph"));
        assert!(prompt.contains("leadership"));
        assert!(prompt.contains("Cut p99 API latency by 38%"));
    }

    /// R-ADV-5 — the honest stub is deterministic and cites the request's evidence id.
    #[test]
    fn stub_returns_deterministic_response() {
        let req = redact(&achievement(), "caching");
        let a = StubProvider::new().rewrite(&req).unwrap();
        let b = StubProvider::new().rewrite(&req).unwrap();
        assert_eq!(a, b, "same input → byte-identical response");
        assert_eq!(a.cited_evidence_id, "exp_1_0_b0");
        assert!(a.rewritten_text.contains("p99 API latency"));
    }

    /// R-ADV-5 — the fabricating stub cites an id absent from any master CV.
    #[test]
    fn stub_fabricating_cites_absent_id() {
        let req = redact(&achievement(), "caching");
        let resp = StubProvider::fabricating().rewrite(&req).unwrap();
        assert_ne!(resp.cited_evidence_id, req.evidence_id);
        // prove absence against a real master CV's resolvable universe
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{},"experience":[
                {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020",
                 "achievementsTasks":[{"id":"exp_1_0_b0","description":"x"}]}]}"#,
        )
        .unwrap();
        let universe = aa_core::ledger::resolvable_ids(&cv);
        assert!(
            !universe.contains(&resp.cited_evidence_id),
            "fabricated id must resolve nowhere"
        );
    }

    /// R-ADV-5 — the response type carries the cited evidence id.
    #[test]
    fn rewrite_response_carries_cited_evidence_id() {
        let req = redact_kind(&achievement(), "caching", RewriteKind::CoverLetterStrength);
        let resp = StubProvider::new().rewrite(&req).unwrap();
        assert_eq!(resp.cited_evidence_id, req.evidence_id);
    }

    /// R-ADV-11 — disabled is the default.
    #[test]
    fn config_disabled_is_default() {
        assert!(!AdvocateConfig::default().enabled);
    }

    /// provider name is surfaced for the provenance badge (R-ADV-10).
    #[test]
    fn provider_names_itself() {
        assert_eq!(StubProvider::new().name(), "stub");
    }

    /// AdvocateError Display variants (coverage of the observable error surface).
    #[test]
    fn error_display_variants() {
        assert!(AdvocateError::Unreachable("connection refused".into())
            .to_string()
            .contains("unreachable"));
        assert!(AdvocateError::Malformed("no json".into())
            .to_string()
            .contains("malformed"));
        assert!(AdvocateError::InsecureEndpoint("http://evil".into())
            .to_string()
            .contains("https"));
    }

    /// RewriteKind round-trips through serde (camelCase) — covers both variants.
    #[test]
    fn rewrite_kind_serde_roundtrips() {
        for k in [RewriteKind::CvBullet, RewriteKind::CoverLetterStrength] {
            let s = serde_json::to_string(&k).unwrap();
            let back: RewriteKind = serde_json::from_str(&s).unwrap();
            assert_eq!(k, back);
        }
        assert_eq!(
            serde_json::to_string(&RewriteKind::CvBullet).unwrap(),
            "\"cvBullet\""
        );
    }
}
