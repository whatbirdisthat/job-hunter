//! Item #9 — the cover letter must render to EXACTLY ONE A4 page, deterministically.
//!
//! Page count is read from the PDF page tree via `lopdf` (NOT `pdfinfo`/`pdf-extract`):
//! `Document::load_mem(bytes).get_pages().len()`. This is a structural count of the
//! `/Page` objects in the document, immune to text-layout heuristics.
//!
//! Three coordinates pin the guarantee:
//!   (a) a NORMAL persona + job → 1 page (the easy case);
//!   (b) a HOSTILE long-content fixture (100+ char name, 500+ char descriptions) →
//!       still 1 page, AND the strengths are demonstrably truncated to budget — this
//!       proves the content budget + tightened template hold one page on bad data;
//!   (c) the SAMPLE-watermarked normal letter is still 1 page (watermark must not
//!       push to a second page). The watermark TEXT presence/absence is already pinned
//!       in `watermark_render.rs`; here we only guard the page count under watermark.

use aa_core::render::CliRenderer;
use aa_core::{
    build_cover_letter, render_cover_letter, tailor, CoverLetter, MasterCv, NormalizedJob,
    Renderer, StrengthParagraph, DEFAULT_TOP_N,
};

/// Count PDF pages structurally via the page tree. A valid one-page render returns 1.
fn page_count(pdf: &[u8]) -> usize {
    let doc = lopdf::Document::load_mem(pdf).expect("render must be a loadable PDF");
    doc.get_pages().len()
}

fn persona_001() -> MasterCv {
    let s = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/personas/persona-001.cv.json"
    ))
    .unwrap();
    MasterCv::from_json(&s).unwrap()
}

fn synthetic_job() -> NormalizedJob {
    NormalizedJob::from_json(
        r#"{"title":"Senior Backend Engineer","company":"Acme","location":"",
            "responsibilities":[],
            "requirements":{"mustHave":["caching","Python"],"niceToHave":["Mentored"]},
            "keywords":[]}"#,
    )
    .unwrap()
}

#[test]
fn normal_cover_letter_is_one_page() {
    // (a) easy case — a real persona + synthetic job renders to one A4 page.
    let m = persona_001();
    let j = synthetic_job();
    let view = tailor(&m, &j, DEFAULT_TOP_N);
    let letter = build_cover_letter(&view, &j, &m);
    let pdf = render_cover_letter(&letter).expect("normal letter renders");
    assert_eq!(
        page_count(&pdf),
        1,
        "a normal cover letter must be exactly one page"
    );
}

/// A synthetic, PII-free MasterCv with a VERY long name and several VERY long
/// achievement descriptions, all obviously-fake (example.com / lorem-style). The job
/// selects the achievements via a shared `caching` requirement/tag.
fn hostile_long_content() -> (MasterCv, NormalizedJob) {
    // 100+ char name — synthetic, obviously not a real person.
    let long_name = format!("Test Persona {}", "Longname ".repeat(12)); // > 100 chars
                                                                        // ~520 chars of deterministic lorem-style synthetic text per achievement.
    let lorem = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium";
    assert!(
        lorem.len() > 500,
        "fixture description must exceed 500 chars"
    );
    assert!(
        long_name.chars().count() > 100,
        "fixture name must exceed 100 chars"
    );

    let cv_json = format!(
        r#"{{
            "schemaVersion":"1.0.0",
            "person":{{"name":"{long_name}","email":"test@example.com"}},
            "experience":[
              {{"id":"exp_h_0","jobTitle":"Engineer","businessName":"Example Corp",
                "startDate":"Jan 2020","endDate":"Present","tags":["caching"],
                "achievementsTasks":[
                  {{"id":"exp_h_0_b0","description":"{lorem}","tags":["caching"]}},
                  {{"id":"exp_h_0_b1","description":"{lorem}","tags":["caching"]}},
                  {{"id":"exp_h_0_b2","description":"{lorem}","tags":["caching"]}},
                  {{"id":"exp_h_0_b3","description":"{lorem}","tags":["caching"]}}
                ]}}
            ]
        }}"#
    );
    let m = MasterCv::from_json(&cv_json).expect("synthetic hostile CV parses");
    let j = NormalizedJob::from_json(
        r#"{"title":"Staff Engineer Position With A Very Long Title Indeed",
            "company":"Example Corporation International Holdings Limited",
            "location":"",
            "responsibilities":[],
            "requirements":{"mustHave":["caching"],"niceToHave":[]},
            "keywords":[]}"#,
    )
    .expect("synthetic hostile job parses");
    (m, j)
}

#[test]
fn hostile_long_content_cover_letter_is_still_one_page() {
    // (b) the budget guarantee on HOSTILE data: a 100+ char name and 500+ char
    // descriptions still render to a single page, AND truncation demonstrably fired.
    let (m, j) = hostile_long_content();
    let view = tailor(&m, &j, DEFAULT_TOP_N);
    let letter = build_cover_letter(&view, &j, &m);

    // truncation actually fired: <= 200 chars and ends with the ellipsis, ids survived.
    assert!(
        !letter.strengths.is_empty(),
        "hostile job must select strengths"
    );
    assert!(letter.strengths.len() <= 3, "strength cap holds");
    for s in &letter.strengths {
        assert!(
            s.text.chars().count() <= 200,
            "strength over 200 chars: {}",
            s.text.chars().count()
        );
        assert!(
            s.text.ends_with('…'),
            "long descriptions must be truncated with an ellipsis"
        );
        assert!(
            !s.source_evidence_id.is_empty(),
            "evidence id must survive truncation"
        );
    }

    let pdf = render_cover_letter(&letter).expect("hostile letter renders");
    assert_eq!(
        page_count(&pdf),
        1,
        "even hostile long content must fit one page"
    );
}

/// ~2500 chars of deterministic, PII-free lorem text per call (extends the lorem
/// vocabulary already used by `hostile_long_content`). Used to build content that is
/// genuinely multi-page when rendered UNbudgeted.
fn lorem_blob() -> String {
    // ~520-char base sentence (same vocabulary as the hostile fixture), repeated to
    // safely exceed the ~2000-char single-strength overflow point at the 3-strength cap.
    let base = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium ";
    let blob = base.repeat(5); // ~2600 chars
    assert!(
        blob.chars().count() >= 2500,
        "lorem blob must be at least 2500 chars, got {}",
        blob.chars().count()
    );
    blob
}

#[test]
fn budget_is_load_bearing_raw_is_multipage_budgeted_is_one_page() {
    // Non-vacuity coordinate (item #9): proves the per-strength content budget — not
    // the template + 3-strength cap alone — is what holds the cover letter to one page.
    //
    // Step 1: the SAME content, UNtruncated, is genuinely multi-page. We build a
    // CoverLetter directly via the public API with 3 strengths of ~2500 chars each and
    // assert the raw render is >= 2 pages. This is the control: without the budget the
    // letter overflows.
    //
    // Step 2: feed that exact content through the real path (a MasterCv whose
    // achievement descriptions ARE those blobs + a job that selects them via the shared
    // `caching` tag), call build_cover_letter (which applies the budget), and assert the
    // render is exactly 1 page.
    //
    // If `truncate_ellipsis` were removed from build_cover_letter, the budgeted render
    // would carry the full ~2500-char blobs and flip to >= 2 pages, FAILING step 2 —
    // that is the dependency this coordinate pins.
    let blob = lorem_blob();

    // Step 1 — RAW: untruncated content, built directly, must overflow to >= 2 pages.
    let raw_letter = CoverLetter {
        greeting: "Dear Hiring Team,".to_string(),
        why_role: "I'm writing to apply for the Engineer position at Example Corp.".to_string(),
        strengths: vec![
            StrengthParagraph {
                text: blob.clone(),
                source_evidence_id: "exp_b_0".to_string(),
            },
            StrengthParagraph {
                text: blob.clone(),
                source_evidence_id: "exp_b_1".to_string(),
            },
            StrengthParagraph {
                text: blob.clone(),
                source_evidence_id: "exp_b_2".to_string(),
            },
        ],
        closing: "I would welcome the chance to discuss further.\n\nKind regards,\nTest Persona"
            .to_string(),
        candidate_name: "Test Persona".to_string(),
    };
    let raw_pdf = render_cover_letter(&raw_letter).expect("raw letter renders");
    let raw_pages = page_count(&raw_pdf);
    assert!(
        raw_pages >= 2,
        "raw (unbudgeted) 3x~2500-char content must overflow to >= 2 pages, got {raw_pages}"
    );

    // Step 2 — BUDGETED: the SAME blobs as achievement descriptions, selected by the
    // job, pushed through build_cover_letter (which truncates to the 200-char budget),
    // must collapse to exactly 1 page.
    let cv_json = format!(
        r#"{{
            "schemaVersion":"1.0.0",
            "person":{{"name":"Test Persona","email":"test@example.com"}},
            "experience":[
              {{"id":"exp_b_0","jobTitle":"Engineer","businessName":"Example Corp",
                "startDate":"Jan 2020","endDate":"Present","tags":["caching"],
                "achievementsTasks":[
                  {{"id":"exp_b_0_b0","description":"{blob}","tags":["caching"]}},
                  {{"id":"exp_b_0_b1","description":"{blob}","tags":["caching"]}},
                  {{"id":"exp_b_0_b2","description":"{blob}","tags":["caching"]}}
                ]}}
            ]
        }}"#
    );
    let m = MasterCv::from_json(&cv_json).expect("synthetic budget CV parses");
    let j = NormalizedJob::from_json(
        r#"{"title":"Senior Backend Engineer","company":"Example Corp","location":"",
            "responsibilities":[],
            "requirements":{"mustHave":["caching"],"niceToHave":[]},
            "keywords":[]}"#,
    )
    .expect("synthetic budget job parses");
    let view = tailor(&m, &j, DEFAULT_TOP_N);
    let letter = build_cover_letter(&view, &j, &m);

    // sanity: the budget genuinely fired on this content (truncated, ellipsis-tagged).
    assert!(!letter.strengths.is_empty(), "job must select strengths");
    for s in &letter.strengths {
        assert!(
            s.text.chars().count() <= 200 && s.text.ends_with('…'),
            "the budget must truncate this content for the test to be meaningful"
        );
    }

    let budgeted_pdf = render_cover_letter(&letter).expect("budgeted letter renders");
    let budgeted_pages = page_count(&budgeted_pdf);
    assert_eq!(
        budgeted_pages, 1,
        "the SAME content, after the budget fires, must collapse to exactly 1 page"
    );
}

#[test]
fn watermarked_cover_letter_is_still_one_page() {
    // (c) regression — the SAMPLE watermark overlay must not push the letter to a
    // second page. (Watermark TEXT presence is pinned in watermark_render.rs.)
    let m = persona_001();
    let j = synthetic_job();
    let view = tailor(&m, &j, DEFAULT_TOP_N);
    let letter = build_cover_letter(&view, &j, &m);
    let pdf = CliRenderer::default()
        .render_cover_letter_watermarked(&letter, true)
        .expect("watermarked letter renders");
    assert_eq!(
        page_count(&pdf),
        1,
        "the watermarked letter must remain one page"
    );
}
