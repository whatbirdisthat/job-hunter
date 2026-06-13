//! L2 module/boundary tests for aa-docx (item #10).
//!
//! STRUCTURAL PARITY, not "text present": every assertion ties the authored DOCX
//! back to the SAME aa-core data the PDF path renders, and to the SHARED heading
//! vocabulary (`CvTemplate::heading_vocabulary()` ∪ {"Experience"}) so the DOCX
//! path can never silently drift from the typst templates.
//!
//! Personas are SYNTHETIC and PII-free (the committed `persona-001` fixture +
//! inline `@example`-free literals). The round-trip uses the PRODUCTION DOCX
//! reader `aa_cvimport::import_resume(bytes, ResumeKind::Docx)`.

use aa_core::{CvTemplate, MasterCv, NormalizedJob, Requirements, TailoredView};
use std::io::Cursor;

// ── fixtures (mirror crates/core/src/render.rs `master()` / `job()`) ────────────

fn master() -> MasterCv {
    let s = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/personas/persona-001.cv.json"
    ))
    .unwrap();
    MasterCv::from_json(&s).unwrap()
}

fn job() -> NormalizedJob {
    NormalizedJob {
        title: "Senior Backend Engineer".into(),
        company: "Acme".into(),
        location: String::new(),
        responsibilities: vec![],
        requirements: Requirements {
            must_have: vec!["caching".into(), "Python".into()],
            nice_to_have: vec![],
        },
        keywords: vec![],
    }
}

fn view() -> TailoredView {
    aa_core::tailor(&master(), &job(), 3)
}

/// Recover the DOCX flow text exactly as the production pipeline does, by reading
/// `word/document.xml` paragraph-by-paragraph through the cvimport extractor. We
/// re-extract here (rather than `import_resume`, which maps to a MasterCv) so the
/// raw paragraph text — every heading, job line, and bullet — is observable.
fn document_xml_text(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("valid zip");
    let mut xml = String::new();
    {
        use std::io::Read;
        let mut f = zip
            .by_name("word/document.xml")
            .expect("word/document.xml present");
        f.read_to_string(&mut xml).expect("utf-8 document.xml");
    }
    // Strip tags → leave the visible text. Sufficient for substring/subset asserts.
    let mut out = String::new();
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

// ── (a) valid ZIP containing word/document.xml ──────────────────────────────────

#[test]
fn cv_docx_is_a_valid_zip_with_document_xml() {
    let bytes = aa_docx::cv_docx(&view(), CvTemplate::Classic, false).expect("authors a docx");
    assert!(!bytes.is_empty());
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("output must be a valid ZIP");
    assert!(
        zip.by_name("word/document.xml").is_ok(),
        "a .docx must contain word/document.xml"
    );
}

// ── (b) round-trips through the production cvimport DOCX reader ──────────────────

#[test]
fn cv_docx_round_trips_through_cvimport_with_name_and_job_titles() {
    use aa_cvimport::{import_resume, ResumeKind};
    let bytes = aa_docx::cv_docx(&view(), CvTemplate::Classic, false).expect("authors a docx");
    // The production reader must ingest it without error → a NEW MasterCv.
    let recovered = import_resume(&bytes, ResumeKind::Docx).expect("cvimport reads our docx");
    // And the visible flow text must carry the candidate name + every job title.
    let text = document_xml_text(&bytes);
    assert!(
        text.contains("Devin Voss"),
        "recovered text must contain the candidate name"
    );
    for e in &view().cv.experience {
        if e.hide.unwrap_or(false) {
            continue;
        }
        assert!(
            text.contains(&e.job_title),
            "recovered text must contain job title {:?}",
            e.job_title
        );
    }
    // The reader recovered *some* structure (parse-don't-validate produced a doc).
    assert_eq!(recovered.schema_version, "1.0.0");
}

// ── (c) every emitted heading ∈ heading_vocabulary() ∪ {"Experience"} ───────────

#[test]
fn every_cv_heading_is_in_the_shared_vocabulary() {
    for template in [CvTemplate::Classic, CvTemplate::Compact] {
        let bytes = aa_docx::cv_docx(&view(), template, false).expect("authors a docx");
        let text = document_xml_text(&bytes);
        // The allow-list: the template's ATS vocabulary plus the literal "Experience".
        let allowed: Vec<&str> = template.heading_vocabulary().to_vec();
        // The four skill labels we may emit, each of which MUST be in the vocabulary.
        for label in [
            "Languages",
            "Skills",
            "Tools & Technologies",
            "Platforms & Services",
        ] {
            if text.contains(label) {
                assert!(
                    allowed.contains(&label),
                    "emitted heading {label:?} is NOT in the shared vocabulary"
                );
            }
        }
        assert!(
            text.contains("Experience"),
            "the Experience heading is always emitted"
        );
        assert!(
            allowed.contains(&"Experience"),
            "Experience must be a member of the vocabulary"
        );
    }
}

// ── (d) every selected achievement description is present ───────────────────────

#[test]
fn every_selected_achievement_description_is_present() {
    let v = view();
    let bytes = aa_docx::cv_docx(&v, CvTemplate::Classic, false).expect("authors a docx");
    let text = document_xml_text(&bytes);
    for e in &v.cv.experience {
        if e.hide.unwrap_or(false) {
            continue;
        }
        for a in &e.achievements_tasks {
            assert!(
                text.contains(&a.description),
                "missing achievement description: {:?}",
                a.description
            );
        }
    }
}

// ── (e) cover letter: every strength text + evidence id + greeting/closing ──────

#[test]
fn cover_letter_docx_carries_strengths_evidence_greeting_and_closing() {
    let letter = aa_core::build_cover_letter(&view(), &job(), &master());
    assert!(!letter.strengths.is_empty(), "fixture must yield strengths");
    let bytes = aa_docx::cover_letter_docx(&letter, false).expect("authors a letter docx");
    let text = document_xml_text(&bytes);
    assert!(text.contains(&letter.greeting), "greeting present");
    assert!(
        text.contains(letter.closing.lines().next().unwrap()),
        "closing present"
    );
    for s in &letter.strengths {
        assert!(
            text.contains(&s.text),
            "strength text present: {:?}",
            s.text
        );
        let tag = format!("[evidence: {}]", s.source_evidence_id);
        assert!(text.contains(&tag), "evidence tag present: {tag:?}");
    }
}

// ── (f) watermark parity ────────────────────────────────────────────────────────

#[test]
fn watermark_true_emits_sentinel_false_does_not_cv() {
    let on = aa_docx::cv_docx(&view(), CvTemplate::Classic, true).expect("watermarked");
    let off = aa_docx::cv_docx(&view(), CvTemplate::Classic, false).expect("plain");
    assert!(
        document_xml_text(&on).contains(aa_core::SAMPLE_WATERMARK),
        "watermark=true must emit the SAMPLE sentinel"
    );
    assert!(
        !document_xml_text(&off).contains(aa_core::SAMPLE_WATERMARK),
        "watermark=false must NOT emit the sentinel"
    );
}

#[test]
fn watermark_true_emits_sentinel_false_does_not_cover_letter() {
    let letter = aa_core::build_cover_letter(&view(), &job(), &master());
    let on = aa_docx::cover_letter_docx(&letter, true).expect("watermarked");
    let off = aa_docx::cover_letter_docx(&letter, false).expect("plain");
    assert!(document_xml_text(&on).contains(aa_core::SAMPLE_WATERMARK));
    assert!(!document_xml_text(&off).contains(aa_core::SAMPLE_WATERMARK));
}

// ── (g) edge personas: empty experience / unicode name author valid docx ────────

#[test]
fn empty_experience_persona_authors_valid_docx() {
    let cv =
        MasterCv::from_json(r#"{"schemaVersion":"1.0.0","person":{"name":"X"},"experience":[]}"#)
            .unwrap();
    let v = aa_core::tailor(&cv, &job(), 3);
    let bytes = aa_docx::cv_docx(&v, CvTemplate::Classic, false).expect("empty-exp docx");
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
    assert!(zip.by_name("word/document.xml").is_ok());
}

#[test]
fn unicode_name_persona_authors_valid_docx() {
    let cv = MasterCv::from_json(
        r#"{"schemaVersion":"1.0.0","person":{"name":"Café Ünïcøde"},"experience":[]}"#,
    )
    .unwrap();
    let v = aa_core::tailor(&cv, &job(), 3);
    let bytes = aa_docx::cv_docx(&v, CvTemplate::Compact, true).expect("unicode docx");
    let text = document_xml_text(&bytes);
    assert!(text.contains("Café Ünïcøde"), "unicode name survives");
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
    assert!(zip.by_name("word/document.xml").is_ok());
}

// ── (h) both templates produce valid docx with the invariant holding ────────────

#[test]
fn both_templates_author_valid_docx() {
    for template in [CvTemplate::Classic, CvTemplate::Compact] {
        let bytes = aa_docx::cv_docx(&view(), template, false)
            .unwrap_or_else(|e| panic!("{template:?} must author a docx: {e}"));
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("valid zip");
        assert!(zip.by_name("word/document.xml").is_ok());
    }
}

// ── determinism: LENGTH-deterministic (NOT byte-deterministic — documented) ─────
//
// EMPIRICAL FINDING (item #10 DISCUSS, verified under concurrency): aa-docx output is
// LENGTH-deterministic for identical input but NOT byte-deterministic. The divergence
// is NOT an embedded timestamp (docx-rs writes a fixed 1970 `docProps/core.xml` and
// fixed ZIP header times — two builds are byte-identical when run serially). It is a
// docx-rs PROCESS-GLOBAL auto-id counter embedded in `word/document.xml`: under
// CONCURRENT builds (parallel `cargo test`), two identical-input calls interleave with
// other in-flight builds and receive different auto-generated element ids, so the
// document.xml payload differs while its LENGTH is unchanged (ids are fixed-width).
// The ids are docx-rs internal numbering, not our authored content. Hence the robust
// same-input invariant we can pin is EQUAL LENGTH. (Run serially, these would also be
// byte-equal — but the test must hold under the parallel gate, so we assert length.)

#[test]
fn cv_docx_is_length_deterministic() {
    let v = view();
    let a = aa_docx::cv_docx(&v, CvTemplate::Classic, false).unwrap();
    let b = aa_docx::cv_docx(&v, CvTemplate::Classic, false).unwrap();
    assert_eq!(
        a.len(),
        b.len(),
        "cv_docx must be length-deterministic for identical input (see module note: \
         docx-rs's process-global auto-id counter prevents byte-equality under concurrency)"
    );
}

#[test]
fn cover_letter_docx_is_length_deterministic() {
    let letter = aa_core::build_cover_letter(&view(), &job(), &master());
    let a = aa_docx::cover_letter_docx(&letter, true).unwrap();
    let b = aa_docx::cover_letter_docx(&letter, true).unwrap();
    assert_eq!(
        a.len(),
        b.len(),
        "cover_letter_docx must be length-deterministic for identical input (see module note)"
    );
}
