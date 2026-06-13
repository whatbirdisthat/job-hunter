//! aa-core — the Applicant Advocate deterministic tailoring engine.
//!
//! Honours the pinned algorithms §A–H (see SUBJECT_MATTER_UNDERSTANDING.md §7) and
//! the invariants I1–I6. No LLM, no network, deterministic. The tailored view is a
//! schema-conformant `MasterCv` (§H) so `templates/cv/classic.typ` renders it
//! unchanged.
//!
//! Public surface (L2 module contract):
//!   - [`MasterCv::from_json`] / [`NormalizedJob::from_json`] — parse-don't-validate
//!   - [`coverage::coverage_report`] — §B coverage + §C fit
//!   - [`tailor::tailor`] — §D ranking/summary + §H view assembly
//!   - [`ledger::cv_ledger`] / [`ledger::guard`] — §E evidence-ledger guard
//!   - [`render::render_cv`] / [`render::render_cover_letter`] — §H/§G embedded render
//!   - [`assemble_application`] — the L4 system path: job + cv → two PDFs (export-guarded)

pub mod ats;
pub mod coverage;
pub mod job;
pub mod keyword_coverage;
pub mod ledger;
pub mod matching;
pub mod normalize;
pub mod render;
pub mod samples;
pub mod tailor;
pub mod types;

pub use ats::{ats_report, AtsCheck, AtsCheckId, AtsReport, AtsStatus};
pub use coverage::{coverage_report, CoverageReport, RequirementCoverage};
pub use job::{NormalizedJob, Requirements};
pub use keyword_coverage::{keyword_coverage, KeywordClass, KeywordCoverage, KeywordHit};
pub use ledger::{cv_ledger, guard, LedgerNode};
pub use render::{
    is_valid_pdf, render_cover_letter, render_cv, render_cv_with_template, CoverLetter, CvTemplate,
    Renderer, StrengthParagraph,
};
pub use samples::{
    cover_letter_filename, cv_filename, decide, empty_person, fill_with_samples, ExportDecision,
    MissingFields, BLOCKED_MESSAGE, SAMPLE_WATERMARK,
};
pub use tailor::{pick_summary, requirement_for, tailor, TailoredView, DEFAULT_TOP_N};
pub use types::{Achievement, CoreError, Experience, MasterCv, Person, Skill};

/// The two exported PDFs of an application (§G: cv.pdf + cover-letter.pdf).
pub struct ApplicationExport {
    pub cv_pdf: Vec<u8>,
    pub cover_letter_pdf: Vec<u8>,
    pub coverage: CoverageReport,
    pub view: TailoredView,
}

/// Content budget that keeps the cover letter to a single A4 page (item #9): the
/// maximum number of strength paragraphs, and the per-field char caps. Tuned with
/// the tightened `classic-letter.typ` so even hostile (very long) input fits one page.
const MAX_STRENGTHS: usize = 3;
const STRENGTH_MAX_CHARS: usize = 200;
const WHY_ROLE_MAX_CHARS: usize = 300;

/// Truncate `s` to at most `max_chars` CHARACTERS (not bytes — unicode-safe), word-aware.
///
/// - If `s` already fits (`chars().count() <= max_chars`), it is returned unchanged
///   with NO ellipsis appended.
/// - Otherwise the prefix is cut so the result INCLUDING the trailing `…` (U+2026,
///   one char) is within `max_chars`. The cut is pulled back to the last whitespace
///   boundary inside the window so a word is never split; if the window has no
///   whitespace it is a hard cut. Trailing whitespace before the ellipsis is trimmed.
///
/// Deterministic and panic-free for any input (no indexing, no unwrap).
fn truncate_ellipsis(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    // Reserve one char for the ellipsis. If the budget is a single char, the result
    // is just the ellipsis (no room for any body).
    let body_budget = max_chars.saturating_sub(1);
    let window: String = s.chars().take(body_budget).collect();
    // Pull back to the last whitespace boundary so we don't split a word; if there
    // is no whitespace in the window, hard-cut the whole window.
    let cut = match window.rfind(char::is_whitespace) {
        Some(idx) => &window[..idx],
        None => &window,
    };
    let mut out = cut.trim_end().to_string();
    out.push('…');
    out
}

/// Build a cover letter (§G) from a tailored view + job. Greeting/why-role are
/// scaffold (templated from job fields); up to 3 strength paragraphs each wrap one
/// of the top selected achievements, carrying its evidence id. Each strength text and
/// the why-role are bounded (item #9 content budget) so the letter is one A4 page.
pub fn build_cover_letter(
    view: &TailoredView,
    job: &NormalizedJob,
    master: &MasterCv,
) -> CoverLetter {
    let candidate = master.person.name.clone().unwrap_or_default();
    let company = if job.company.is_empty() {
        "your team".to_string()
    } else {
        job.company.clone()
    };
    let role = if job.title.is_empty() {
        "this role".to_string()
    } else {
        job.title.clone()
    };

    // top strengths: first up-to-MAX_STRENGTHS selected achievements, resolved from
    // the view. Each description is truncated to the content budget (word-boundary
    // ellipsis) so the rendered bullets fit one page; the evidence id is untouched.
    let mut strengths = Vec::new();
    'outer: for e in &view.cv.experience {
        for a in &e.achievements_tasks {
            strengths.push(StrengthParagraph {
                text: truncate_ellipsis(&a.description, STRENGTH_MAX_CHARS),
                source_evidence_id: a.id.clone(),
            });
            if strengths.len() >= MAX_STRENGTHS {
                break 'outer;
            }
        }
    }

    // why-role: one concise sentence naming the role + company. Defensively bounded
    // so a pathological role/company string can't push the letter to a second page.
    let why_role = truncate_ellipsis(
        &format!("I'm writing to apply for the {role} position at {company}, where my track record maps directly to what you're looking for."),
        WHY_ROLE_MAX_CHARS,
    );

    CoverLetter {
        greeting: "Dear Hiring Team,".to_string(),
        why_role,
        strengths,
        closing: format!(
            "I would welcome the chance to discuss further.\n\nKind regards,\n{candidate}"
        ),
        candidate_name: candidate,
    }
}

/// The L4 system path (offline, deterministic): master CV + normalized job →
/// coverage + tailored view + ledger-guarded two PDFs. Export is BLOCKED (Err) if
/// any claim-bearing node has a dangling evidence id (§E / I2).
pub fn assemble_application(
    master: &MasterCv,
    job: &NormalizedJob,
) -> Result<ApplicationExport, CoreError> {
    let coverage = coverage_report(master, job);
    let view = tailor(master, job, DEFAULT_TOP_N);

    // §E guard on the CV ledger BEFORE rendering/exporting.
    let cv_nodes = cv_ledger(&view);
    guard(&cv_nodes, master)?;

    let letter = build_cover_letter(&view, job, master);
    // guard the letter's strength paragraphs too (scaffold greeting/why-role exempt)
    let mut letter_nodes: Vec<LedgerNode> = vec![
        LedgerNode::scaffold("letter.greeting"),
        LedgerNode::scaffold("letter.whyRole"),
    ];
    for (i, s) in letter.strengths.iter().enumerate() {
        letter_nodes.push(LedgerNode::claim(
            format!("letter.strength[{i}]"),
            s.source_evidence_id.clone(),
        ));
    }
    guard(&letter_nodes, master)?;

    let cv_pdf = render_cv(&view)?;
    let cover_letter_pdf = render_cover_letter(&letter)?;

    Ok(ApplicationExport {
        cv_pdf,
        cover_letter_pdf,
        coverage,
        view,
    })
}

#[cfg(test)]
mod module_tests {
    //! L2 — public-surface (module) tests + L4 system-path tests.
    use super::*;

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
                nice_to_have: vec!["Mentored".into()],
            },
            keywords: vec![],
        }
    }

    #[test]
    fn public_api_tailor_coverage_render_ledger() {
        let m = master();
        let j = job();
        let cov = coverage_report(&m, &j);
        assert!(cov.fit_score >= 0.0 && cov.fit_score <= 1.0);
        let view = tailor(&m, &j, DEFAULT_TOP_N);
        guard(&cv_ledger(&view), &m).unwrap();
        assert!(render::is_valid_pdf(&render_cv(&view).unwrap()));
    }

    #[test]
    fn assemble_application_produces_two_valid_pdfs() {
        let export = assemble_application(&master(), &job()).expect("clean app assembles");
        assert!(render::is_valid_pdf(&export.cv_pdf));
        assert!(render::is_valid_pdf(&export.cover_letter_pdf));
        assert!(!export.coverage.must_have.is_empty());
    }

    #[test]
    fn assemble_blocks_on_injected_unsupported_claim() {
        // L4 acceptance: injected dangling-id node → export BLOCKED.
        let m = master();
        let j = job();
        let mut view = tailor(&m, &j, DEFAULT_TOP_N);
        view.cv.experience[0].achievements_tasks.push(Achievement {
            id: "GHOST_b1".into(),
            description: "Unsupported fabricated claim".into(),
            emphasise: None,
            tags: vec![],
            metrics: vec![],
            evidence_strength: None,
        });
        let nodes = cv_ledger(&view);
        let err = guard(&nodes, &m).unwrap_err();
        assert!(err.to_string().contains("GHOST_b1"));
    }

    #[test]
    fn every_rendered_bullet_maps_to_evidence_id() {
        // L4 integrity: every bullet in the view resolves in the master CV.
        let m = master();
        let view = tailor(&m, &job(), DEFAULT_TOP_N);
        let universe = ledger::resolvable_ids(&m);
        for e in &view.cv.experience {
            for a in &e.achievements_tasks {
                assert!(
                    universe.contains(&a.id),
                    "bullet {} must map to evidence",
                    a.id
                );
            }
        }
    }

    #[test]
    fn cover_letter_scaffold_defaults_for_empty_job_fields() {
        // covers the empty company/title scaffold branches in build_cover_letter
        let m = master();
        let empty_job = NormalizedJob {
            title: String::new(),
            company: String::new(),
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let view = tailor(&m, &empty_job, DEFAULT_TOP_N);
        let letter = build_cover_letter(&view, &empty_job, &m);
        assert!(letter.why_role.contains("this role"));
        assert!(letter.why_role.contains("your team"));
    }

    #[test]
    fn cover_letter_strengths_carry_evidence() {
        let m = master();
        let view = tailor(&m, &job(), DEFAULT_TOP_N);
        let letter = build_cover_letter(&view, &job(), &m);
        assert!(!letter.strengths.is_empty() && letter.strengths.len() <= 3);
        for s in &letter.strengths {
            assert!(!s.source_evidence_id.is_empty());
        }
    }

    #[test]
    fn cover_letter_strengths_are_truncated_to_budget() {
        // item #9: every strength text is kept within the 200-char budget so the
        // letter stays one page. A view whose achievement descriptions exceed the
        // budget must come back truncated with an ellipsis, evidence id intact.
        let long = "word ".repeat(80); // 400 chars, well over 200
        let doc = format!(
            r#"{{"schemaVersion":"1.0.0","person":{{"name":"X"}},"experience":[
                {{"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020",
                 "tags":["caching"],
                 "achievementsTasks":[{{"id":"e0_b0","description":"{long}"}}]}}]}}"#
        );
        let m = MasterCv::from_json(&doc).unwrap();
        let j = NormalizedJob {
            title: "T".into(),
            company: "C".into(),
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: vec!["caching".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let view = tailor(&m, &j, DEFAULT_TOP_N);
        let letter = build_cover_letter(&view, &j, &m);
        assert!(!letter.strengths.is_empty());
        for s in &letter.strengths {
            assert!(
                s.text.chars().count() <= 200,
                "strength over budget: {} chars",
                s.text.chars().count()
            );
            assert!(
                s.text.ends_with('…'),
                "truncated text must end with ellipsis"
            );
            assert!(!s.source_evidence_id.is_empty());
        }
    }

    #[test]
    fn cover_letter_why_role_is_bounded() {
        // A pathological role/company string cannot blow the why_role budget.
        let m = master();
        let huge = "Z".repeat(5000);
        let j = NormalizedJob {
            title: huge.clone(),
            company: huge,
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let view = tailor(&m, &j, DEFAULT_TOP_N);
        let letter = build_cover_letter(&view, &j, &m);
        assert!(
            letter.why_role.chars().count() <= 300,
            "why_role over budget: {} chars",
            letter.why_role.chars().count()
        );
    }
}

#[cfg(test)]
mod truncate_tests {
    //! L1 — the `truncate_ellipsis` helper (item #9 content budget). Char-based,
    //! word-boundary, unicode-safe, never panics, result always within the budget.
    use super::truncate_ellipsis;

    #[test]
    fn empty_is_unchanged() {
        assert_eq!(truncate_ellipsis("", 200), "");
    }

    #[test]
    fn short_is_unchanged_without_ellipsis() {
        let s = "a short strength sentence";
        assert_eq!(truncate_ellipsis(s, 200), s);
        assert!(!truncate_ellipsis(s, 200).ends_with('…'));
    }

    #[test]
    fn exactly_at_max_is_unchanged() {
        let s = "a".repeat(200);
        assert_eq!(truncate_ellipsis(&s, 200), s);
        assert!(!truncate_ellipsis(&s, 200).ends_with('…'));
    }

    #[test]
    fn over_max_truncates_with_ellipsis_on_word_boundary() {
        // 201 chars made of words → cut to a whitespace boundary, trailing space
        // trimmed, single ellipsis appended, result within budget.
        let s = "word ".repeat(60); // 300 chars
        let out = truncate_ellipsis(&s, 200);
        assert!(out.chars().count() <= 200);
        assert!(out.ends_with('…'));
        // must not split a word: the char before the ellipsis is a word char (we
        // trimmed trailing whitespace), and there is no broken partial token.
        let body = out.trim_end_matches('…');
        assert!(
            body.ends_with("word"),
            "cut on a word boundary, got: {body:?}"
        );
    }

    #[test]
    fn unicode_counts_chars_not_bytes_and_never_breaks_a_char() {
        // multi-byte chars (é is 2 bytes, 😀 is 4) — budget is in CHARS.
        let s = "é".repeat(500); // 500 chars, 1000 bytes
        let out = truncate_ellipsis(&s, 200);
        assert!(out.chars().count() <= 200);
        assert!(out.ends_with('…'));
        // round-trips as valid UTF-8 (no broken char) — String guarantees it, but
        // assert the body is all 'é' (no mojibake / partial code unit).
        let body = out.trim_end_matches('…');
        assert!(body.chars().all(|c| c == 'é'));
    }

    #[test]
    fn no_whitespace_long_string_is_hard_cut() {
        // no whitespace in the window → hard-cut at the budget, ellipsis appended.
        let s = "a".repeat(500);
        let out = truncate_ellipsis(&s, 200);
        assert_eq!(out.chars().count(), 200);
        assert!(out.ends_with('…'));
        assert!(out.trim_end_matches('…').chars().all(|c| c == 'a'));
    }

    #[test]
    fn result_char_count_never_exceeds_max() {
        for max in [1usize, 2, 5, 10, 200, 300] {
            for s in [
                "",
                "x",
                "hello world",
                &"y".repeat(1000),
                &"z z ".repeat(400),
            ] {
                assert!(
                    truncate_ellipsis(s, max).chars().count() <= max,
                    "max={max} s.len={}",
                    s.len()
                );
            }
        }
    }
}
