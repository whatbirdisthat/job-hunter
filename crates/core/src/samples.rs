//! Item 8b — the SAMPLE honesty guard (the load-bearing safety logic).
//!
//! When the adaptive CLI ingestion flow (see `crates/cli/src/main.rs`) cannot recover
//! an IMPORTANT field from the user's CV and the user opts to fill the gap with a
//! SAMPLE value, a sample CV **must not be able to reach an employer unedited**. That
//! safety property is enforced by THREE independent, opt-in barriers:
//!
//!   1. **Block** — normal export is refused unless `--allow-samples` is passed.
//!   2. **Filename** — sample output is written to `cv.SAMPLE.pdf` / `cover-letter.SAMPLE.pdf`,
//!      never the normal names, so a sample artifact is self-identifying on disk.
//!   3. **Watermark** — the rendered document carries a visible
//!      `[SAMPLE — REPLACE BEFORE SENDING]` overlay (threaded into the render as a
//!      typst `--input samples=true`, see [`crate::render`]).
//!
//! The prompting / stdin / TTY plumbing lives in the CLI binary (excluded from the
//! coverage floor, P-COV-4). The DECISION LOGIC and the SAMPLE-FILL logic live HERE,
//! as pure functions, so they are unit-pinned to 100%-of-reachable: a future change
//! that weakens the guard fails a test, not silently a user.
//!
//! Crate-graph note: this module takes the four `missing_*` booleans directly (NOT
//! `aa_cvimport::CompletenessReport`). `aa-core` must not depend on `aa-cvimport`
//! (the crate graph is one-way `aa-cvimport → aa-core`); the CLI reads the report's
//! fields and passes the booleans in.

use crate::types::{Achievement, Experience, MasterCv, Person, Skill};

/// The exact sentinel text rendered as the watermark overlay on every SAMPLE document.
/// Tests assert this string is PRESENT in a sample-rendered PDF and ABSENT otherwise,
/// so the watermark guarantee is non-vacuous.
pub const SAMPLE_WATERMARK: &str = "[SAMPLE — REPLACE BEFORE SENDING]";

/// The four IMPORTANT field classes the completeness check tracks, mirrored from
/// `aa_cvimport::CompletenessReport` WITHOUT importing it (one-way crate graph). Each
/// boolean is `true` when that class is empty in the produced master CV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingFields {
    pub person_name: bool,
    pub experience: bool,
    pub achievement: bool,
    pub skill: bool,
}

impl MissingFields {
    /// Construct from the four flags (as the CLI reads them off a `CompletenessReport`).
    pub fn new(person_name: bool, experience: bool, achievement: bool, skill: bool) -> Self {
        MissingFields {
            person_name,
            experience,
            achievement,
            skill,
        }
    }

    /// True when at least one IMPORTANT class is empty — i.e. the missing-field flow
    /// must run. (The inverse of `CompletenessReport::is_complete`.)
    pub fn any(self) -> bool {
        self.person_name || self.experience || self.achievement || self.skill
    }
}

/// The export decision: what to do once we know whether any SAMPLE value was inserted
/// and whether the user explicitly accepted sample output. This is the single gate
/// every export passes through; the CLI calls [`decide`] exactly once and the PDF
/// writes are unreachable except through its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportDecision {
    /// No SAMPLE values were used — export normally, normal filenames, no watermark.
    RenderNormal,
    /// SAMPLE values were used AND the user opted in (`--allow-samples`/`--use-fakes`)
    /// — export with the `.SAMPLE.` filenames and the watermark overlay.
    RenderWithWatermark,
    /// SAMPLE values were used and the user did NOT opt in — export is BLOCKED.
    Blocked,
}

impl ExportDecision {
    /// Whether this decision renders at all (the two render arms) vs blocks.
    pub fn renders(self) -> bool {
        !matches!(self, ExportDecision::Blocked)
    }

    /// Whether the rendered output must carry the SAMPLE watermark + `.SAMPLE.` names.
    pub fn is_sample(self) -> bool {
        matches!(self, ExportDecision::RenderWithWatermark)
    }
}

/// THE GUARD. Pure total function over the two facts that decide an export:
///
/// | `used_samples` | `allow_samples` | decision               |
/// |----------------|-----------------|------------------------|
/// | false          | (either)        | `RenderNormal`         |
/// | true           | true            | `RenderWithWatermark`  |
/// | true           | false           | `Blocked`              |
///
/// Note `allow_samples` is IRRELEVANT when no samples were used — a clean CV always
/// renders normally regardless of the flag, so `--allow-samples` can never downgrade
/// a real document to a sample one.
pub fn decide(used_samples: bool, allow_samples: bool) -> ExportDecision {
    if !used_samples {
        ExportDecision::RenderNormal
    } else if allow_samples {
        ExportDecision::RenderWithWatermark
    } else {
        ExportDecision::Blocked
    }
}

/// The human-facing message the CLI prints when export is [`ExportDecision::Blocked`].
/// Lives here so the wording (a safety message) is pinned by a test, not free-floating
/// in the coverage-excluded binary.
pub const BLOCKED_MESSAGE: &str = "output contains SAMPLE data; fix it or pass --allow-samples";

/// CV output filename for a given sample-ness and extension (item #10). Sample output
/// gets a `.SAMPLE.` infix, e.g. `cv.SAMPLE.docx`; a clean CV is `cv.<ext>`.
pub fn cv_filename_ext(contains_samples: bool, ext: &str) -> String {
    if contains_samples {
        format!("cv.SAMPLE.{ext}")
    } else {
        format!("cv.{ext}")
    }
}

/// Cover-letter output filename for a given sample-ness and extension (item #10).
pub fn cover_letter_filename_ext(contains_samples: bool, ext: &str) -> String {
    if contains_samples {
        format!("cover-letter.SAMPLE.{ext}")
    } else {
        format!("cover-letter.{ext}")
    }
}

/// CV output filename for a given sample-ness. Sample output is `cv.SAMPLE.pdf`.
/// Returns `&'static str` (the original item-8b contract); backed by the format-aware
/// [`cv_filename_ext`] so the two can never diverge on the `.pdf` value.
pub fn cv_filename(contains_samples: bool) -> &'static str {
    if contains_samples {
        debug_assert_eq!(cv_filename_ext(true, "pdf"), "cv.SAMPLE.pdf");
        "cv.SAMPLE.pdf"
    } else {
        debug_assert_eq!(cv_filename_ext(false, "pdf"), "cv.pdf");
        "cv.pdf"
    }
}

/// Cover-letter output filename for a given sample-ness.
pub fn cover_letter_filename(contains_samples: bool) -> &'static str {
    if contains_samples {
        debug_assert_eq!(
            cover_letter_filename_ext(true, "pdf"),
            "cover-letter.SAMPLE.pdf"
        );
        "cover-letter.SAMPLE.pdf"
    } else {
        debug_assert_eq!(cover_letter_filename_ext(false, "pdf"), "cover-letter.pdf");
        "cover-letter.pdf"
    }
}

// ── Synthetic SAMPLE values ──────────────────────────────────────────────────────
//
// Obviously-fake, never real-looking. The name literally contains "Sample"; the email
// uses a reserved example domain (pii-guard-safe); the experience and achievement get
// REAL synthetic ids (`imp_exp_0` / `imp_exp_0_b0`, the 8a convention) so that the
// evidence-ledger guard still passes on a sample document — which is EXACTLY why the
// three SAMPLE barriers above are required: the ledger alone cannot tell a synthesised
// claim from a real one.

const SAMPLE_NAME: &str = "Alex Sample";
const SAMPLE_JOB_TITLE: &str = "Sample Role";
const SAMPLE_BUSINESS: &str = "Sample Company";
const SAMPLE_START_DATE: &str = "2020";
const SAMPLE_ACHIEVEMENT: &str = "Sample achievement — replace with a real accomplishment.";
const SAMPLE_SKILL: &str = "Sample Skill";

/// Fill the empty IMPORTANT classes of `cv` with obviously-synthetic SAMPLE values.
///
/// Returns `true` iff at least one sample value was actually inserted (`used_samples`).
/// It is a no-op returning `false` when nothing was missing, so calling it
/// unconditionally is safe and `used_samples` is honest.
///
/// The filled experience/achievement carry REAL synthetic ids so a subsequent
/// evidence-ledger guard passes — the SAMPLE guard (`decide` + filename + watermark),
/// not the ledger, is what stops a sample reaching an employer.
pub fn fill_with_samples(cv: &mut MasterCv, missing: MissingFields) -> bool {
    let mut used = false;

    if missing.person_name {
        cv.person.name = Some(SAMPLE_NAME.to_string());
        used = true;
    }

    if missing.experience {
        // No usable experience at all — synthesise one whole sample role (which also
        // satisfies the achievement class). Real synthetic ids, 8a convention.
        cv.experience.push(sample_experience());
        used = true;
    } else if missing.achievement {
        // Experience exists but no achievement text — attach a sample achievement to
        // the first experience, keyed off that experience's id so the ledger resolves.
        if let Some(first) = cv.experience.first_mut() {
            let aid = format!("{}_b{}", first.id, first.achievements_tasks.len());
            first.achievements_tasks.push(sample_achievement(aid));
            used = true;
        }
    }

    if missing.skill {
        cv.skills.push(Skill {
            name: SAMPLE_SKILL.to_string(),
            proficiency: 3,
            aliases: Vec::new(),
            evidence_ids: Vec::new(),
        });
        used = true;
    }

    used
}

fn sample_experience() -> Experience {
    let id = "imp_exp_0".to_string();
    let aid = format!("{id}_b0");
    Experience {
        id: id.clone(),
        job_title: SAMPLE_JOB_TITLE.to_string(),
        business_name: SAMPLE_BUSINESS.to_string(),
        consultancy: None,
        location: None,
        employment_type: None,
        start_date: SAMPLE_START_DATE.to_string(),
        end_date: None,
        domain: None,
        hide: None,
        contact: None,
        tags: Vec::new(),
        achievements_tasks: vec![sample_achievement(aid)],
    }
}

fn sample_achievement(id: String) -> Achievement {
    Achievement {
        id,
        description: SAMPLE_ACHIEVEMENT.to_string(),
        emphasise: None,
        tags: Vec::new(),
        metrics: Vec::new(),
        evidence_strength: None,
    }
}

/// Construct a deliberately-empty `Person` (used by the CLI when mining yields no person
/// object at all, before sample-fill decides whether to populate the name).
pub fn empty_person() -> Person {
    Person {
        name: None,
        professional_title: None,
        professional_description: None,
        location: None,
        email: None,
        phone: None,
        linkedin: None,
        github: None,
        website: None,
        image: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── decide() truth table (R-INGEST-CLI-3) ─────────────────────────────────────
    #[test]
    fn decide_no_samples_renders_normal_regardless_of_flag() {
        assert_eq!(decide(false, false), ExportDecision::RenderNormal);
        // --allow-samples must NEVER downgrade a clean CV to a sample one.
        assert_eq!(decide(false, true), ExportDecision::RenderNormal);
    }

    #[test]
    fn decide_samples_without_allow_is_blocked() {
        assert_eq!(decide(true, false), ExportDecision::Blocked);
    }

    #[test]
    fn decide_samples_with_allow_renders_watermarked() {
        assert_eq!(decide(true, true), ExportDecision::RenderWithWatermark);
    }

    #[test]
    fn export_decision_predicates() {
        assert!(ExportDecision::RenderNormal.renders());
        assert!(!ExportDecision::RenderNormal.is_sample());
        assert!(ExportDecision::RenderWithWatermark.renders());
        assert!(ExportDecision::RenderWithWatermark.is_sample());
        assert!(!ExportDecision::Blocked.renders());
        assert!(!ExportDecision::Blocked.is_sample());
    }

    // ── filenames (R-INGEST-CLI-4) ────────────────────────────────────────────────
    #[test]
    fn filenames_switch_on_sampleness() {
        assert_eq!(cv_filename(false), "cv.pdf");
        assert_eq!(cv_filename(true), "cv.SAMPLE.pdf");
        assert_eq!(cover_letter_filename(false), "cover-letter.pdf");
        assert_eq!(cover_letter_filename(true), "cover-letter.SAMPLE.pdf");
    }

    // ── item #10: format-aware filename helpers (pdf/docx × normal/sample) ─────────
    #[test]
    fn cv_filename_ext_switches_on_sampleness_and_extension() {
        assert_eq!(cv_filename_ext(false, "pdf"), "cv.pdf");
        assert_eq!(cv_filename_ext(true, "pdf"), "cv.SAMPLE.pdf");
        assert_eq!(cv_filename_ext(false, "docx"), "cv.docx");
        assert_eq!(cv_filename_ext(true, "docx"), "cv.SAMPLE.docx");
    }

    #[test]
    fn cover_letter_filename_ext_switches_on_sampleness_and_extension() {
        assert_eq!(cover_letter_filename_ext(false, "pdf"), "cover-letter.pdf");
        assert_eq!(
            cover_letter_filename_ext(true, "pdf"),
            "cover-letter.SAMPLE.pdf"
        );
        assert_eq!(
            cover_letter_filename_ext(false, "docx"),
            "cover-letter.docx"
        );
        assert_eq!(
            cover_letter_filename_ext(true, "docx"),
            "cover-letter.SAMPLE.docx"
        );
    }

    #[test]
    fn static_pdf_helpers_agree_with_ext_helpers() {
        // The &'static str helpers must equal the format-aware ones for "pdf".
        assert_eq!(cv_filename(false), cv_filename_ext(false, "pdf"));
        assert_eq!(cv_filename(true), cv_filename_ext(true, "pdf"));
        assert_eq!(
            cover_letter_filename(false),
            cover_letter_filename_ext(false, "pdf")
        );
        assert_eq!(
            cover_letter_filename(true),
            cover_letter_filename_ext(true, "pdf")
        );
    }

    // ── MissingFields ─────────────────────────────────────────────────────────────
    #[test]
    fn missing_fields_any() {
        assert!(!MissingFields::new(false, false, false, false).any());
        assert!(MissingFields::new(true, false, false, false).any());
        assert!(MissingFields::new(false, false, false, true).any());
    }

    // ── fill_with_samples (R-INGEST-CLI-2) ────────────────────────────────────────
    fn empty_cv() -> MasterCv {
        MasterCv {
            schema_version: "1.0.0".to_string(),
            person: empty_person(),
            headline: None,
            summary_variants: Vec::new(),
            programming_languages: Vec::new(),
            skills: Vec::new(),
            tools_technologies: Vec::new(),
            as_a_services: Vec::new(),
            experience: Vec::new(),
            projects: Vec::new(),
            education: Vec::new(),
            certifications: Vec::new(),
            awards: Vec::new(),
            preferences: None,
        }
    }

    #[test]
    fn fill_nothing_missing_is_noop() {
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(false, false, false, false));
        assert!(!used, "no gaps → no samples inserted");
        assert!(cv.person.name.is_none());
        assert!(cv.experience.is_empty());
        assert!(cv.skills.is_empty());
    }

    #[test]
    fn fill_name_inserts_obviously_synthetic_name() {
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(true, false, false, false));
        assert!(used);
        assert_eq!(cv.person.name.as_deref(), Some("Alex Sample"));
    }

    #[test]
    fn fill_experience_synthesises_role_with_real_ids() {
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(false, true, false, false));
        assert!(used);
        assert_eq!(cv.experience.len(), 1);
        let e = &cv.experience[0];
        assert_eq!(e.id, "imp_exp_0");
        assert_eq!(e.job_title, "Sample Role");
        assert_eq!(e.business_name, "Sample Company");
        // a real synthetic achievement id keyed off the experience id (ledger-resolvable)
        assert_eq!(e.achievements_tasks.len(), 1);
        assert_eq!(e.achievements_tasks[0].id, "imp_exp_0_b0");
    }

    #[test]
    fn fill_achievement_attaches_to_existing_experience() {
        let mut cv = empty_cv();
        // a pre-existing experience with NO achievements
        cv.experience.push(Experience {
            id: "imp_exp_0".to_string(),
            job_title: "Engineer".to_string(),
            business_name: "Acme".to_string(),
            consultancy: None,
            location: None,
            employment_type: None,
            start_date: "2019".to_string(),
            end_date: None,
            domain: None,
            hide: None,
            contact: None,
            tags: Vec::new(),
            achievements_tasks: Vec::new(),
        });
        let used = fill_with_samples(&mut cv, MissingFields::new(false, false, true, false));
        assert!(used);
        // attached to the FIRST experience, id keyed off it (ledger-resolvable)
        assert_eq!(cv.experience[0].achievements_tasks.len(), 1);
        assert_eq!(cv.experience[0].achievements_tasks[0].id, "imp_exp_0_b0");
    }

    #[test]
    fn fill_achievement_with_no_experience_present_is_noop_for_that_class() {
        // achievement-missing but experience-missing FALSE and yet no experiences exist:
        // the `else if` branch's `first_mut()` is None → that class contributes nothing.
        // (In practice the CLI sees experience-missing too; this pins the None arm.)
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(false, false, true, false));
        assert!(
            !used,
            "no experience to attach an achievement to → no sample used"
        );
        assert!(cv.experience.is_empty());
    }

    #[test]
    fn fill_skill_inserts_synthetic_skill() {
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(false, false, false, true));
        assert!(used);
        assert_eq!(cv.skills.len(), 1);
        assert_eq!(cv.skills[0].name, "Sample Skill");
        assert_eq!(cv.skills[0].proficiency, 3);
    }

    #[test]
    fn fill_all_classes_sets_used_and_is_ledger_shaped() {
        let mut cv = empty_cv();
        let used = fill_with_samples(&mut cv, MissingFields::new(true, true, true, true));
        assert!(used);
        assert_eq!(cv.person.name.as_deref(), Some("Alex Sample"));
        // experience-missing wins over achievement-missing (one synthesised role covers both)
        assert_eq!(cv.experience.len(), 1);
        assert_eq!(cv.experience[0].achievements_tasks.len(), 1);
        assert_eq!(cv.skills.len(), 1);
    }

    #[test]
    fn watermark_and_blocked_message_are_the_pinned_wording() {
        assert_eq!(SAMPLE_WATERMARK, "[SAMPLE — REPLACE BEFORE SENDING]");
        assert!(BLOCKED_MESSAGE.contains("--allow-samples"));
    }

    #[test]
    fn empty_person_is_all_none() {
        let p = empty_person();
        assert!(p.name.is_none() && p.email.is_none() && p.phone.is_none());
    }
}
