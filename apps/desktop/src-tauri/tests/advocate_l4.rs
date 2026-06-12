//! L4 system — the Applicant Advocate export path (R-ADV-7..11). Drives the ACTUAL
//! command layer (`Session::export_application`) with the advocate flag ON, using the
//! deterministic stub (NO live model). Proves:
//!   * flag ON + honest stub → two valid PDFs, `ai_used == true`;
//!   * THE MANDATORY adversarial test: a fabricating provider's dangling cited id is
//!     NAMED and BLOCKS the export — with a NON-VACUOUS twin (the same journey with an
//!     honest stub PASSES, so the block is proven real, not vacuous);
//!   * flag OFF → export byte-identical to the deterministic path, `ai_used == false`;
//!   * flag ON + unreachable provider → explicit error (NO silent fallback).

use aa_advocate::{
    AdvocateError, AdvocateProvider, RewriteKind, RewriteRequest, RewriteResponse, StubProvider,
};
use aa_desktop::{CommandError, Session};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn persona() -> String {
    std::fs::read_to_string(root().join("fixtures/personas/persona-001.cv.json")).unwrap()
}

const JD: &str = "We are hiring a Senior Backend Engineer at Acme Group. You will own delivery \
    end to end. Required: Strong TypeScript or Python; Stakeholder management; AWS or GCP \
    experience. Nice to have: GraphQL; Fintech domain knowledge.";

/// A provider that is always unreachable (R-ADV-9 fixture). No network.
struct UnreachableProvider;
impl AdvocateProvider for UnreachableProvider {
    fn rewrite(&self, _req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError> {
        Err(AdvocateError::Unreachable(
            "connection refused (test)".into(),
        ))
    }
    fn name(&self) -> &'static str {
        "unreachable-test"
    }
}

/// Honest for CV bullets, fabricating ONLY for cover-letter strength paragraphs. This
/// lets the CV ledger guard PASS (so execution reaches the letter loop) and proves the
/// SAME re-guard re-entry blocks a fabricated cover-letter strength id (R-ADV-8, the
/// "same re-guard re-entry to cover-letter strength paragraphs" clause).
struct LetterFabricator;
impl AdvocateProvider for LetterFabricator {
    fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError> {
        let cited = match req.kind {
            RewriteKind::CvBullet => req.evidence_id.clone(),
            RewriteKind::CoverLetterStrength => "FABRICATED_letter_strength_id".to_string(),
        };
        Ok(RewriteResponse {
            rewritten_text: format!("Refined: {}", req.evidence_text),
            cited_evidence_id: cited,
        })
    }
    fn name(&self) -> &'static str {
        "letter-fabricator"
    }
}

fn seeded(provider: Box<dyn AdvocateProvider + Send + Sync>, enabled: bool) -> Session {
    let mut s = Session::with_provider(provider);
    s.import_master_cv(&persona()).unwrap();
    s.parse_job(JD).unwrap();
    s.set_advocate_enabled(enabled);
    assert_eq!(s.advocate_enabled(), enabled, "flag reflects the toggle");
    s
}

#[test]
fn rewrite_enabled_clean_stub_exports_two_pdfs() {
    // R-ADV-7, R-ADV-10: flag ON + honest stub → two valid PDFs, ai_used true.
    let s = seeded(Box::new(StubProvider::new()), true);
    let (cv_pdf, letter_pdf, result) = s.export_application().expect("clean rewrite exports");
    assert!(aa_core::is_valid_pdf(&cv_pdf), "cv.pdf valid");
    assert!(aa_core::is_valid_pdf(&letter_pdf), "cover-letter.pdf valid");
    assert!(result.ai_used, "ai_used must be true when the flag is on");
    assert_eq!(result.provider.as_deref(), Some("stub"));
    assert!(
        !result.rewritten_ids.is_empty(),
        "honest stub cites ids back → bullets are marked rewritten"
    );
}

// ── THE MANDATORY ADVERSARIAL NON-VACUOUS TEST (R-ADV-8) ────────────────────────────
#[test]
fn adversarial_stub_fabricates_dangling_id_blocks_export() {
    // Flag ON + a provider that cites an id absent from the master CV → the bullet
    // ADOPTS the fabricated id → the EXISTING ledger guard (against the IMMUTABLE master)
    // NAMES it and BLOCKS the export.
    let s = seeded(Box::new(StubProvider::fabricating()), true);
    let err = s
        .export_application()
        .expect_err("a fabricated cited id MUST block the export");
    match err {
        CommandError::ExportBlocked(msg) => {
            assert!(
                msg.contains("FABRICATED_evidence_id"),
                "the block message must NAME the fabricated id: {msg}"
            );
        }
        other => panic!("expected ExportBlocked, got {other:?}"),
    }
}

#[test]
fn fabricated_cover_letter_strength_id_blocks_export() {
    // R-ADV-8 (cover-letter clause): a provider that is honest for CV bullets but
    // fabricates the cited id for cover-letter strength paragraphs. The CV guard PASSES
    // (honest bullets) so execution reaches the letter loop, then the SAME re-guard
    // re-entry NAMES and BLOCKS the fabricated strength id.
    let s = seeded(Box::new(LetterFabricator), true);
    let err = s
        .export_application()
        .expect_err("a fabricated cover-letter strength id MUST block the export");
    match err {
        CommandError::ExportBlocked(msg) => assert!(
            msg.contains("FABRICATED_letter_strength_id"),
            "the block must NAME the fabricated strength id: {msg}"
        ),
        other => panic!("expected ExportBlocked, got {other:?}"),
    }
}

#[test]
fn non_vacuous_twin_honest_stub_same_journey_passes() {
    // NON-VACUITY: the EXACT same journey with an HONEST stub PASSES. This proves the
    // adversarial block above is caused by the fabricated id, not by the advocate path
    // itself — the guard distinguishes honest from fabricated.
    let s = seeded(Box::new(StubProvider::new()), true);
    let (cv_pdf, letter_pdf, result) = s
        .export_application()
        .expect("the honest-stub twin journey must PASS");
    assert!(aa_core::is_valid_pdf(&cv_pdf));
    assert!(aa_core::is_valid_pdf(&letter_pdf));
    assert!(result.ai_used);
}

#[test]
fn flag_off_is_byte_identical_to_deterministic() {
    // R-ADV-11: with the flag OFF the advocate path is skipped entirely, so the render
    // INPUTS are byte-identical to a session with no advocate involvement. We compare the
    // deterministic pre-render artefact (the guarded view-CV JSON + cover-letter JSON),
    // NOT the output PDFs — typst does not emit byte-stable PDFs across invocations
    // (R-D2: render assertions are valid-PDF + ledger invariant, never raw-byte equality),
    // so comparing PDF bytes would be a flaky, wrong anchor. The render inputs ARE the
    // deterministic boundary, and equality there proves the flag-off path is the
    // deterministic path verbatim.
    let off = seeded(Box::new(StubProvider::new()), false);
    let (cv_off, letter_off, ai_off) = off.render_inputs().expect("flag-off render inputs");

    // a plain default session (no advocate ever touched) — the deterministic baseline
    let mut plain = Session::new();
    plain.import_master_cv(&persona()).unwrap();
    plain.parse_job(JD).unwrap();
    let (cv_plain, letter_plain, ai_plain) =
        plain.render_inputs().expect("deterministic render inputs");

    assert_eq!(
        cv_off, cv_plain,
        "the CV render input must be byte-identical with the flag off"
    );
    assert_eq!(
        letter_off, letter_plain,
        "the cover-letter render input must be byte-identical with the flag off"
    );
    assert!(!ai_off, "flag off → ai_used false");
    assert!(!ai_plain);

    // and the export surface confirms ai_used == false + no rewritten ids with the flag off
    let off2 = seeded(Box::new(StubProvider::new()), false);
    let (cv_pdf, letter_pdf, res_off) = off2.export_application().expect("flag-off export");
    assert!(aa_core::is_valid_pdf(&cv_pdf));
    assert!(aa_core::is_valid_pdf(&letter_pdf));
    assert!(!res_off.ai_used, "flag off → ai_used false");
    assert!(
        res_off.provider.is_none(),
        "no provider name when ai not used"
    );
    assert!(res_off.rewritten_ids.is_empty());
}

#[test]
fn cover_letter_strength_is_rewritten_exactly_once() {
    // FIX 5 (single-rewrite): the cover-letter strength paragraphs share their source text
    // with the CV bullets. The honest stub prefixes "Demonstrated impact: " on every rewrite.
    // Before the fix the strength text was rewritten TWICE (the bullet was rewritten in place,
    // then build_cover_letter read the already-rewritten text and rewrote it again →
    // "Demonstrated impact: Demonstrated impact: ..."). With the letter built from the ORIGINAL
    // view text, the prefix must appear EXACTLY ONCE in each strength paragraph.
    let s = seeded(Box::new(StubProvider::new()), true);
    let (_cv_json, letter_json, ai_used) = s.render_inputs().expect("flag-on render inputs");
    assert!(ai_used, "flag on → ai_used true");

    let letter: serde_json::Value = serde_json::from_str(&letter_json).unwrap();
    let strengths = letter["strengths"].as_array().expect("strengths array");
    assert!(
        !strengths.is_empty(),
        "there must be strength paragraphs to check"
    );
    for sp in strengths {
        let text = sp["text"].as_str().expect("strength text");
        let occurrences = text.matches("Demonstrated impact: ").count();
        assert_eq!(
            occurrences, 1,
            "strength text must be rewritten EXACTLY once (no double-prefix): {text:?}"
        );
    }
}

#[test]
fn unreachable_provider_surfaces_error() {
    // R-ADV-9: flag ON + unreachable provider → explicit CommandError, NO silent
    // fallback to the deterministic text.
    let s = seeded(Box::new(UnreachableProvider), true);
    let err = s
        .export_application()
        .expect_err("an unreachable provider must surface an error, not fall back silently");
    match err {
        CommandError::Advocate(msg) => assert!(msg.contains("unreachable"), "msg: {msg}"),
        other => panic!("expected CommandError::Advocate, got {other:?}"),
    }
}
