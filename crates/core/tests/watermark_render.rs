//! Item 8b — NON-VACUOUS watermark render assertions (R-INGEST-CLI-5).
//!
//! The SAMPLE watermark guarantee is only meaningful if the sentinel text actually
//! appears in a sample-rendered PDF — and does NOT appear in a normal one. These tests
//! render through the real `typst` CLI (the same path the CLI uses) and extract the PDF
//! text via `pdf-extract`, asserting both directions. If the watermark silently stopped
//! being drawn, BOTH a present-assertion and an absent-assertion guard it.

use aa_core::render::CliRenderer;
use aa_core::{tailor, CvTemplate, MasterCv, Renderer, SAMPLE_WATERMARK};
use aa_core::{CoverLetter, NormalizedJob};

fn master() -> MasterCv {
    let s = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/personas/persona-001.cv.json"
    ))
    .unwrap();
    MasterCv::from_json(&s).unwrap()
}

fn job() -> NormalizedJob {
    NormalizedJob::from_json(
        r#"{"title":"Senior Backend Engineer","company":"Acme","location":"",
            "responsibilities":[],
            "requirements":{"mustHave":["caching","Python"],"niceToHave":[]},
            "keywords":[]}"#,
    )
    .unwrap()
}

fn pdf_text(bytes: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(bytes).expect("extract PDF text")
}

#[test]
fn sample_cv_pdf_contains_the_watermark_text() {
    let view = tailor(&master(), &job(), 3);
    let pdf = CliRenderer::default()
        .render_cv_watermarked(&view, CvTemplate::Classic, true)
        .expect("watermarked render");
    let text = pdf_text(&pdf);
    assert!(
        text.contains(SAMPLE_WATERMARK),
        "watermarked CV must contain the sentinel `{SAMPLE_WATERMARK}`; got:\n{text}"
    );
}

#[test]
fn normal_cv_pdf_does_not_contain_the_watermark_text() {
    let view = tailor(&master(), &job(), 3);
    let pdf = CliRenderer::default()
        .render_cv_watermarked(&view, CvTemplate::Classic, false)
        .expect("normal render");
    let text = pdf_text(&pdf);
    assert!(
        !text.contains(SAMPLE_WATERMARK),
        "a NORMAL CV must NOT carry the SAMPLE watermark"
    );
}

#[test]
fn sample_compact_cv_pdf_contains_the_watermark_text() {
    // the watermark is wired into the compact template too (R-INGEST-CLI-5).
    let view = tailor(&master(), &job(), 3);
    let pdf = CliRenderer::default()
        .render_cv_watermarked(&view, CvTemplate::Compact, true)
        .expect("watermarked compact render");
    assert!(pdf_text(&pdf).contains(SAMPLE_WATERMARK));
}

#[test]
fn sample_cover_letter_pdf_contains_the_watermark_text() {
    let letter = CoverLetter {
        greeting: "Dear Hiring Team,".into(),
        why_role: "I'm excited about the role at Acme.".into(),
        strengths: vec![],
        closing: "Kind regards, Alex Sample".into(),
        candidate_name: "Alex Sample".into(),
    };
    let pdf = CliRenderer::default()
        .render_cover_letter_watermarked(&letter, true)
        .expect("watermarked letter render");
    assert!(pdf_text(&pdf).contains(SAMPLE_WATERMARK));
}

#[test]
fn normal_cover_letter_pdf_does_not_contain_the_watermark_text() {
    let letter = CoverLetter {
        greeting: "Dear Hiring Team,".into(),
        why_role: "I'm excited about the role at Acme.".into(),
        strengths: vec![],
        closing: "Kind regards, Devin Voss".into(),
        candidate_name: "Devin Voss".into(),
    };
    let pdf = CliRenderer::default()
        .render_cover_letter_watermarked(&letter, false)
        .expect("normal letter render");
    assert!(!pdf_text(&pdf).contains(SAMPLE_WATERMARK));
}
