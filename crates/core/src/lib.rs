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
pub use tailor::{pick_summary, requirement_for, tailor, TailoredView, DEFAULT_TOP_N};
pub use types::{Achievement, CoreError, Experience, MasterCv, Person, Skill};

/// The two exported PDFs of an application (§G: cv.pdf + cover-letter.pdf).
pub struct ApplicationExport {
    pub cv_pdf: Vec<u8>,
    pub cover_letter_pdf: Vec<u8>,
    pub coverage: CoverageReport,
    pub view: TailoredView,
}

/// Build a cover letter (§G) from a tailored view + job. Greeting/why-role are
/// scaffold (templated from job fields); 2–3 strength paragraphs each wrap one of
/// the top selected achievements, carrying its evidence id.
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

    // top strengths: first up-to-3 selected achievements, resolved from the view
    let mut strengths = Vec::new();
    'outer: for e in &view.cv.experience {
        for a in &e.achievements_tasks {
            strengths.push(StrengthParagraph {
                text: a.description.clone(),
                source_evidence_id: a.id.clone(),
            });
            if strengths.len() >= 3 {
                break 'outer;
            }
        }
    }

    CoverLetter {
        greeting: "Dear Hiring Team,".to_string(),
        why_role: format!(
            "I'm writing to apply for the {role} position at {company}. My background maps directly to what you're looking for, and the highlights below are drawn verbatim from my track record."
        ),
        strengths,
        closing: format!("I would welcome the chance to discuss further.\n\nKind regards,\n{candidate}"),
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
}
