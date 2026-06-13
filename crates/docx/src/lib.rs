//! aa-docx — item #10: pure DOCX authoring for the Applicant Advocate.
//!
//! Two pure functions author Word `.docx` bytes from the SAME `aa-core` structs the
//! PDF path renders ([`cv_docx`] from a [`TailoredView`], [`cover_letter_docx`] from a
//! [`CoverLetter`]). No I/O, no network, no panics on any input: every fallible step
//! maps to [`CoreError::Render`], the existing render error arm.
//!
//! ## Anti-drift heading contract (load-bearing)
//!
//! The CV section headings are NOT reinvented strings — each is drawn from
//! [`CvTemplate::heading_vocabulary`] (the ATS allow-list in `aa-core`'s `render.rs`)
//! plus the literal `"Experience"`, mirroring `templates/cv/classic.typ`. The skill
//! labels live in one explicit table [`skill_sections`] so a reviewer can see they are
//! not free-floating literals, and [`debug_assert`] / a unit test pin that every label
//! is a member of the vocabulary. This guarantees the DOCX and typst paths cannot
//! silently diverge on section structure.
//!
//! DOCX is inherently a single linear flow, so Classic and Compact (which differ only
//! in column layout in the PDF) emit the SAME heading set here; the template parameter
//! is honoured for API parity and future shape divergence.
//!
//! Crate graph (one-way): depends on `aa-core` ONLY. `docx-rs` (MIT) is the runtime
//! packing dependency.

use aa_core::{CoreError, CoverLetter, CvTemplate, Skill, TailoredView};
use docx_rs::{Docx, Paragraph, Run};
use std::io::Cursor;

/// The four CV skill sections, each label paired with the `MasterCv` field it renders.
/// Defined as an explicit table (not inline literals) so a reviewer can confirm every
/// label is an ATS-vocabulary heading, not a reinvented string. Mirrors `classic.typ`'s
/// `skillBlock(...)` calls and `crates/cvimport/tests/support/mod.rs`.
///
/// Each `&'static str` here MUST satisfy `template.heading_vocabulary().contains(label)`
/// — asserted at construction ([`debug_assert`] in [`cv_docx`]) and pinned by a unit test.
fn skill_sections(view: &TailoredView) -> [(&'static str, &[Skill]); 4] {
    [
        ("Languages", &view.cv.programming_languages),
        ("Skills", &view.cv.skills),
        ("Tools & Technologies", &view.cv.tools_technologies),
        ("Platforms & Services", &view.cv.as_a_services),
    ]
}

/// Build a single-run paragraph carrying `text`.
fn para(text: impl Into<String>) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text.into()))
}

/// Pack a built `Docx` into `.docx` bytes, mapping any pack/IO error to
/// [`CoreError::Render`] (the existing render error arm — see `render.rs`). The pattern
/// is the one proven in-repo (`crates/cvimport/tests/support/mod.rs`).
fn pack(docx: Docx) -> Result<Vec<u8>, CoreError> {
    let mut buf = Cursor::new(Vec::new());
    docx.build()
        // P-COV-2 (docx): packing a successfully-built Docx into an in-memory
        // Cursor<Vec<u8>> cannot return an I/O error, so this map_err closure is dead by
        // construction (defensive-IO class — see doc/COVERAGE.md). Kept for a total Result.
        .pack(&mut buf)
        .map_err(|e| CoreError::Render(e.to_string()))?;
    Ok(buf.into_inner())
}

/// Author the tailored CV as `.docx` bytes (item #10).
///
/// Mirrors `templates/cv/classic.typ`: a header (name, professional title, contact
/// line), then the non-empty skill sections (labels drawn from the shared ATS heading
/// vocabulary), then the `"Experience"` heading followed by each non-hidden experience.
///
/// When `watermark` is true the document's FIRST paragraph is the exact
/// [`aa_core::SAMPLE_WATERMARK`] sentinel (item 8b parity), mirroring how the typst
/// templates emit the sentinel inline at the top of the flow for robust extraction.
///
/// `template` is honoured for parity; DOCX is linear so Classic and Compact emit the
/// same heading set (both draw from the same `heading_vocabulary()`).
pub fn cv_docx(
    view: &TailoredView,
    template: CvTemplate,
    watermark: bool,
) -> Result<Vec<u8>, CoreError> {
    let cv = &view.cv;
    let mut docx = Docx::new();

    // Item 8b: the SAMPLE sentinel is the FIRST paragraph when watermarking, so a sample
    // document is self-identifying and the sentinel is trivially extractable.
    if watermark {
        docx = docx.add_paragraph(para(aa_core::SAMPLE_WATERMARK));
    }

    // ── header: name / professional title / contact line ─────────────────────────
    if let Some(name) = &cv.person.name {
        docx = docx.add_paragraph(para(name.clone()));
    }
    if let Some(title) = &cv.person.professional_title {
        if !title.is_empty() {
            docx = docx.add_paragraph(para(title.clone()));
        }
    }
    if let Some(contact) = contact_line(view) {
        docx = docx.add_paragraph(para(contact));
    }

    // ── skill sections (labels from the shared ATS vocabulary) ───────────────────
    let vocab = template.heading_vocabulary();
    for (label, items) in skill_sections(view) {
        // The anti-drift contract: every emitted skill heading is a vocabulary member.
        debug_assert!(
            vocab.contains(&label),
            "skill heading {label:?} is not in the ATS heading vocabulary"
        );
        if items.is_empty() {
            continue; // skip an empty section, exactly like skillBlock(...)
        }
        docx = docx.add_paragraph(para(label));
        let names: Vec<&str> = items.iter().map(|s| s.name.as_str()).collect();
        docx = docx.add_paragraph(para(names.join(", ")));
    }

    // ── experience ───────────────────────────────────────────────────────────────
    // "Experience" is the one heading that is NOT a skill label; it is still a
    // vocabulary member (asserted), keeping the whole emitted set inside the allow-list.
    debug_assert!(vocab.contains(&"Experience"));
    docx = docx.add_paragraph(para("Experience"));
    for e in &cv.experience {
        if e.hide.unwrap_or(false) {
            continue; // honour the hide flag, like jobEntry(...)
        }
        // job line: "<jobTitle>  <startDate> – <endDate?>"
        let mut job_line = e.job_title.clone();
        job_line.push_str("  ");
        job_line.push_str(&e.start_date);
        if let Some(end) = e.end_date.as_deref() {
            if !end.is_empty() {
                job_line.push_str(" – ");
                job_line.push_str(end);
            }
        }
        docx = docx.add_paragraph(para(job_line));

        // "<businessName> · <location?>"
        let mut biz_line = e.business_name.clone();
        if let Some(loc) = e.location.as_deref() {
            if !loc.is_empty() {
                biz_line.push_str(" · ");
                biz_line.push_str(loc);
            }
        }
        docx = docx.add_paragraph(para(biz_line));

        for a in &e.achievements_tasks {
            docx = docx.add_paragraph(para(a.description.clone()));
        }
    }

    pack(docx)
}

/// Build the contact line, mirroring `classic.typ`'s `contactLine(person)`: the present,
/// non-empty fields in order (location, email, phone, linkedin, github, website) joined
/// with `"  ·  "`. `None` when no field is present (no empty paragraph emitted).
fn contact_line(view: &TailoredView) -> Option<String> {
    let p = &view.cv.person;
    let parts: Vec<&str> = [
        p.location.as_deref(),
        p.email.as_deref(),
        p.phone.as_deref(),
        p.linkedin.as_deref(),
        p.github.as_deref(),
        p.website.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("  ·  "))
    }
}

/// Author the cover letter as `.docx` bytes (item #10), mirroring the item-9 one-page
/// structure of `templates/letter/classic-letter.typ`:
///   candidate name (header) → greeting → why-role → each strength as a BULLETED
///   paragraph ending in `[evidence: <sourceEvidenceId>]` → closing.
///
/// When `watermark` is true the FIRST paragraph is the [`aa_core::SAMPLE_WATERMARK`]
/// sentinel (item 8b parity).
pub fn cover_letter_docx(letter: &CoverLetter, watermark: bool) -> Result<Vec<u8>, CoreError> {
    let mut docx = Docx::new();

    if watermark {
        docx = docx.add_paragraph(para(aa_core::SAMPLE_WATERMARK));
    }

    if !letter.candidate_name.is_empty() {
        docx = docx.add_paragraph(para(letter.candidate_name.clone()));
    }
    docx = docx.add_paragraph(para(letter.greeting.clone()));
    docx = docx.add_paragraph(para(letter.why_role.clone()));

    for s in &letter.strengths {
        // Bulleted paragraph whose text ENDS with the evidence id rendered as
        // `[evidence: <id>]`, so the round-trip + parity tests find every id AND text.
        let line = format!("• {}  [evidence: {}]", s.text, s.source_evidence_id);
        docx = docx.add_paragraph(para(line));
    }

    docx = docx.add_paragraph(para(letter.closing.clone()));

    pack(docx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_core::{MasterCv, NormalizedJob, Requirements};

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
                must_have: vec!["caching".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        }
    }

    fn view() -> TailoredView {
        aa_core::tailor(&master(), &job(), 3)
    }

    /// The load-bearing anti-drift unit pin: EVERY skill label this crate can emit is a
    /// member of `heading_vocabulary()`, for every template. If a future edit reinvents a
    /// label, this fails before any DOCX is authored.
    #[test]
    fn every_skill_label_is_in_the_heading_vocabulary() {
        let v = view();
        for template in [CvTemplate::Classic, CvTemplate::Compact] {
            let vocab = template.heading_vocabulary();
            for (label, _) in skill_sections(&v) {
                assert!(
                    vocab.contains(&label),
                    "{label:?} not in {template:?} vocabulary"
                );
            }
            // "Experience" — the only non-skill heading — is also a vocabulary member.
            assert!(vocab.contains(&"Experience"));
        }
    }

    #[test]
    fn cv_docx_authors_non_empty_bytes() {
        let bytes = cv_docx(&view(), CvTemplate::Classic, false).unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.starts_with(b"PK"), "docx is a zip → PK signature");
    }

    /// A person with NO contact fields and NO professional title exercises the
    /// `contact_line == None` arm and the title-skip arm (no empty paragraphs emitted).
    #[test]
    fn cv_docx_handles_person_without_contact_or_title() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"Solo"},"experience":[]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        assert!(contact_line(&v).is_none());
        let bytes = cv_docx(&v, CvTemplate::Compact, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }

    /// A person whose `professionalTitle` is present BUT EMPTY (`Some("")`) exercises the
    /// FALSE arm of the `if !title.is_empty()` guard (the `Some(title)` branch matches but
    /// the empty title is not emitted), which the `None`-title test above cannot reach.
    #[test]
    fn cv_docx_skips_an_empty_professional_title() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0",
                "person":{"name":"Solo","professionalTitle":""},"experience":[]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        assert_eq!(
            v.cv.person.professional_title.as_deref(),
            Some(""),
            "title present but empty"
        );
        let bytes = cv_docx(&v, CvTemplate::Classic, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }

    /// A person WITH contact fields exercises the `Some(..)` join arm and the
    /// empty-string filter (an empty title/contact field is skipped, not emitted).
    #[test]
    fn contact_line_joins_present_nonempty_fields() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0",
                "person":{"name":"Has Contact","professionalTitle":"Engineer",
                          "location":"Berlin","email":"","github":"gh/x"},
                "experience":[]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        let line = contact_line(&v).expect("has contact fields");
        assert!(line.contains("Berlin"));
        assert!(line.contains("gh/x"));
        assert!(
            !line.contains("  ·    ·  "),
            "empty email must be filtered out"
        );
    }

    /// A person with NO name exercises the name-skip arm; an experience with an
    /// `endDate` and a `location` exercises both optional-suffix arms of the job/biz
    /// lines; a hidden experience exercises the `hide` skip.
    #[test]
    fn cv_docx_handles_no_name_end_date_location_and_hidden() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{},"experience":[
                {"id":"e0","jobTitle":"Engineer","businessName":"Acme",
                 "location":"Remote","startDate":"Jan 2020","endDate":"Dec 2022",
                 "achievementsTasks":[{"id":"e0_b0","description":"Did a thing"}]},
                {"id":"e1","jobTitle":"Hidden","businessName":"Ghost",
                 "startDate":"2019","hide":true,
                 "achievementsTasks":[{"id":"e1_b0","description":"unseen"}]}]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        assert!(v.cv.person.name.is_none());
        let bytes = cv_docx(&v, CvTemplate::Classic, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }

    /// Recover the visible DOCX flow text from packed bytes by stripping the XML tags
    /// of `word/document.xml`. Mirrors the boundary extractor in `tests/module_l2.rs`;
    /// kept local so these private-behaviour reachability tests stay inline.
    fn document_text(bytes: &[u8]) -> String {
        use std::io::Read;
        let mut zip =
            zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("authored bytes are a zip");
        let mut xml = String::new();
        zip.by_name("word/document.xml")
            .expect("word/document.xml present")
            .read_to_string(&mut xml)
            .expect("utf-8 document.xml");
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

    /// `tailor` strips hidden experiences (`tailor.rs` filters `!hide`), so a hidden
    /// entry only reaches `cv_docx` when a caller hand-builds the (all-public) view and
    /// injects one — a reachable path. This pins the `continue` at the hide check: the
    /// hidden jobTitle MUST NOT appear in the authored document text.
    #[test]
    fn cv_docx_skips_an_injected_hidden_experience() {
        use aa_core::{Achievement, Experience};
        let mut v = view();
        v.cv.experience.push(Experience {
            id: "hidden0".into(),
            job_title: "ZZZHiddenTitle".into(),
            business_name: "Ghost".into(),
            consultancy: None,
            location: Some("Nowhere".into()),
            employment_type: None,
            start_date: "2018".into(),
            end_date: Some("2019".into()),
            domain: None,
            hide: Some(true),
            contact: None,
            tags: vec![],
            achievements_tasks: vec![Achievement {
                id: "hidden0_b0".into(),
                description: "unseen work".into(),
                emphasise: None,
                tags: vec![],
                metrics: vec![],
                evidence_strength: None,
            }],
        });
        let bytes = cv_docx(&v, CvTemplate::Classic, false).unwrap();
        assert!(bytes.starts_with(b"PK"), "docx is a zip → PK signature");
        let text = document_text(&bytes);
        assert!(
            !text.contains("ZZZHiddenTitle"),
            "the hidden experience's jobTitle must be skipped, got: {text}"
        );
    }

    /// An experience with NO `endDate` and NO `location` exercises the FALSE arm of the
    /// `!end.is_empty()` / `!loc.is_empty()` guards (the `None` path): the optional
    /// suffixes are absent. Existing tests only feed non-empty endDate+location.
    #[test]
    fn cv_docx_handles_missing_end_date_and_location() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"Cur"},"experience":[
                {"id":"e0","jobTitle":"Cur","businessName":"Now","startDate":"2021",
                 "achievementsTasks":[{"id":"e0_b0","description":"ongoing"}]}]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        let e = &v.cv.experience[0];
        assert!(e.end_date.is_none(), "no endDate → None arm");
        assert!(e.location.is_none(), "no location → None arm");
        let bytes = cv_docx(&v, CvTemplate::Classic, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
        let text = document_text(&bytes);
        assert!(text.contains("Cur"), "job line still authored");
        assert!(
            !text.contains(" – "),
            "no endDate → no date-range separator emitted"
        );
        assert!(
            !text.contains(" · "),
            "no location → no business/location separator emitted"
        );
    }

    /// An experience with `endDate: ""` and `location: ""` (present-but-empty) exercises
    /// the FALSE arm via the `is_empty()` guard rather than the `None` arm, pinning the
    /// empty-string path of the same two `if` blocks.
    #[test]
    fn cv_docx_handles_empty_end_date_and_location() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"Cur"},"experience":[
                {"id":"e0","jobTitle":"Cur","businessName":"Now","startDate":"2021",
                 "endDate":"","location":"",
                 "achievementsTasks":[{"id":"e0_b0","description":"ongoing"}]}]}"#,
        )
        .unwrap();
        let v = aa_core::tailor(&cv, &job(), 3);
        let e = &v.cv.experience[0];
        assert_eq!(e.end_date.as_deref(), Some(""), "endDate present but empty");
        assert_eq!(
            e.location.as_deref(),
            Some(""),
            "location present but empty"
        );
        let bytes = cv_docx(&v, CvTemplate::Classic, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
        let text = document_text(&bytes);
        assert!(
            !text.contains(" – "),
            "empty endDate → no date-range suffix"
        );
        assert!(!text.contains(" · "), "empty location → no location suffix");
    }

    #[test]
    fn cover_letter_docx_authors_non_empty_bytes_and_handles_empty_name() {
        let letter = aa_core::build_cover_letter(&view(), &job(), &master());
        let bytes = cover_letter_docx(&letter, false).unwrap();
        assert!(bytes.starts_with(b"PK"));
        // empty candidate name → the name-skip arm
        let mut bare = letter.clone();
        bare.candidate_name = String::new();
        bare.strengths.clear();
        let bytes = cover_letter_docx(&bare, true).unwrap();
        assert!(bytes.starts_with(b"PK"));
    }
}
