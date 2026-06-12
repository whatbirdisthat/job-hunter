//! Item #6, capability B — ATS-readability checker (PURE, no IO, no PDF parsing).
//!
//! `ats_report(template, view)` frames each readability concern over TEMPLATE
//! PROPERTIES + the TAILORED content (never over rendered PDF bytes — there is no PDF
//! parse here). Every check is a pinnable coordinate: a stable [`AtsCheckId`] + a
//! [`AtsStatus`] (`Pass`/`Warn`) + a human message, emitted in a deterministic order
//! (R-ATS-7/8). The function takes `&TailoredView` and NEVER mutates it (R-ATS-2).
//!
//! Checks (DISCUSS-ATS-* all RESOLVED in the plan §6.0):
//!   * ColumnReliance — WARN iff `template.is_multi_column()` (Classic warns; Compact
//!     passes). Multi-column layouts confuse linear ATS text extraction (R-ATS-3).
//!   * OverlyLong — WARN iff the total achievement count across visible experiences
//!     exceeds [`OVERLY_LONG_ACHIEVEMENTS`] (a deterministic ≈2-page content proxy;
//!     we do NOT parse the PDF) (R-ATS-4).
//!   * NonStandardHeadings — the template's fixed heading vocabulary must be a subset
//!     of [`STANDARD_HEADINGS`]; both shipped templates pass. A guard against a future
//!     template introducing an odd heading (R-ATS-5, option a).
//!   * MissingExtractableText — WARN on an empty document, or an experience with no
//!     description-bearing achievement (R-ATS-6).
//!   * UnusualFont — always Pass, keyed off the fixed bundled Liberation stack. The
//!     stack is fixed for ALL templates, so an unusual font is unreachable by
//!     construction; kept as an auditable always-Pass coordinate (R-ATS-9).

use crate::render::CvTemplate;
use crate::tailor::TailoredView;
use serde::{Deserialize, Serialize};

/// Content-length proxy: WARN when a tailored document carries more than this many
/// achievements across its visible experiences (R-ATS-4, DISCUSS-ATS-LEN RESOLVED).
/// Rationale: a single-page CV holds ~15 strong bullets; ~30 is the ≈2-page line past
/// which an ATS (and a human screener) starts skimming. Deterministic, content-derived,
/// and computed WITHOUT rendering — we never parse the PDF.
pub const OVERLY_LONG_ACHIEVEMENTS: usize = 30;

/// The fixed standard section-heading allow-list (R-ATS-5). A template's
/// [`CvTemplate::heading_vocabulary`] must be a subset of this set to pass.
pub const STANDARD_HEADINGS: &[&str] = &[
    "Summary",
    "Professional Summary",
    "Experience",
    "Work Experience",
    "Skills",
    "Languages",
    "Tools & Technologies",
    "Platforms & Services",
    "Education",
    "Projects",
    "Certifications",
];

/// The fixed bundled font stack (R-ATS-9). All templates render with Liberation faces,
/// so "unusual font" is unreachable; this names the audited stack for the always-Pass
/// coordinate.
pub const BUNDLED_FONT_FAMILY: &str = "Liberation Sans";

/// A stable, pinnable identifier for each ATS check (R-ATS-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtsCheckId {
    ColumnReliance,
    OverlyLong,
    NonStandardHeadings,
    MissingExtractableText,
    UnusualFont,
}

/// The status of a single ATS check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtsStatus {
    Pass,
    Warn,
}

/// One pinnable ATS check coordinate (R-ATS-7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtsCheck {
    pub id: AtsCheckId,
    pub status: AtsStatus,
    pub message: String,
}

/// The full ATS-readability report: an ordered list of check coordinates (R-ATS-7/8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtsReport {
    pub checks: Vec<AtsCheck>,
}

impl AtsReport {
    /// Convenience: look up a check by id (used by the UI + tests).
    pub fn check(&self, id: AtsCheckId) -> Option<&AtsCheck> {
        self.checks.iter().find(|c| c.id == id)
    }
}

/// Count description-bearing achievements across the view's VISIBLE experiences. An
/// achievement is "description-bearing" when its description is non-empty after trim.
fn description_bearing_count(view: &TailoredView) -> usize {
    view.cv
        .experience
        .iter()
        .filter(|e| !e.hide.unwrap_or(false))
        .flat_map(|e| e.achievements_tasks.iter())
        .filter(|a| !a.description.trim().is_empty())
        .count()
}

/// True iff the document has NO extractable text: no visible experience carries a
/// description-bearing achievement (R-ATS-6). An empty experience list satisfies this.
fn missing_extractable_text(view: &TailoredView) -> bool {
    description_bearing_count(view) == 0
}

/// The non-standard-headings coordinate (R-ATS-5): WARN iff any heading in `vocab` is
/// outside [`STANDARD_HEADINGS`]. Both shipped templates pass; the WARN arm guards a
/// future template introducing an odd heading. Factored out so BOTH arms are unit-
/// testable directly (the public `ats_report` only ever sees standard vocabularies).
fn non_standard_headings_check(vocab: &[&str]) -> AtsCheck {
    let standard: std::collections::HashSet<&str> = STANDARD_HEADINGS.iter().copied().collect();
    let offenders: Vec<&str> = vocab
        .iter()
        .copied()
        .filter(|h| !standard.contains(h))
        .collect();
    if offenders.is_empty() {
        AtsCheck {
            id: AtsCheckId::NonStandardHeadings,
            status: AtsStatus::Pass,
            message: "All section headings are standard and ATS-recognised.".to_string(),
        }
    } else {
        AtsCheck {
            id: AtsCheckId::NonStandardHeadings,
            status: AtsStatus::Warn,
            message: format!("Non-standard section headings: {}.", offenders.join(", ")),
        }
    }
}

/// The PURE ATS-readability report (R-ATS-1). No IO, no PDF parse, read-only over the
/// view (R-ATS-2). Deterministic: identical `(template, view)` → identical report
/// (R-ATS-8); checks are emitted in a fixed order (R-ATS-7).
pub fn ats_report(template: CvTemplate, view: &TailoredView) -> AtsReport {
    let mut checks = Vec::with_capacity(5);

    // 1. Column-reliance (R-ATS-3).
    let multi = template.is_multi_column();
    checks.push(AtsCheck {
        id: AtsCheckId::ColumnReliance,
        status: if multi {
            AtsStatus::Warn
        } else {
            AtsStatus::Pass
        },
        message: if multi {
            "Multi-column layout can scramble ATS text extraction; a single-column \
             template (Compact) is safer."
                .to_string()
        } else {
            "Single-column layout — ATS-friendly text flow.".to_string()
        },
    });

    // 2. Overly-long document (R-ATS-4) — deterministic content proxy, no PDF parse.
    let n = description_bearing_count(view);
    let long = n > OVERLY_LONG_ACHIEVEMENTS;
    checks.push(AtsCheck {
        id: AtsCheckId::OverlyLong,
        status: if long {
            AtsStatus::Warn
        } else {
            AtsStatus::Pass
        },
        message: if long {
            format!(
                "{n} achievements exceeds the {OVERLY_LONG_ACHIEVEMENTS}-bullet \
                 (~2-page) guideline; trim to keep the CV scannable."
            )
        } else {
            format!("{n} achievements — within the {OVERLY_LONG_ACHIEVEMENTS}-bullet guideline.")
        },
    });

    // 3. Non-standard headings (R-ATS-5, option a) — template-property guard.
    checks.push(non_standard_headings_check(template.heading_vocabulary()));

    // 4. Missing extractable text (R-ATS-6).
    let empty = missing_extractable_text(view);
    checks.push(AtsCheck {
        id: AtsCheckId::MissingExtractableText,
        status: if empty {
            AtsStatus::Warn
        } else {
            AtsStatus::Pass
        },
        message: if empty {
            "No description-bearing achievements — the document may have no extractable \
             text for an ATS to read."
                .to_string()
        } else {
            "Document carries extractable achievement text.".to_string()
        },
    });

    // 5. Unusual font (R-ATS-9) — always Pass, keyed off the fixed bundled stack.
    checks.push(AtsCheck {
        id: AtsCheckId::UnusualFont,
        status: AtsStatus::Pass,
        message: format!(
            "Uses the bundled {BUNDLED_FONT_FAMILY} stack — a standard, ATS-safe font."
        ),
    });

    AtsReport { checks }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{NormalizedJob, Requirements};
    use crate::tailor::tailor;
    use crate::types::MasterCv;

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
        tailor(&master(), &job(), 3)
    }

    fn status_of(template: CvTemplate, v: &TailoredView, id: AtsCheckId) -> AtsStatus {
        ats_report(template, v).check(id).unwrap().status
    }

    #[test]
    fn column_reliance_warns_for_classic_passes_for_compact() {
        let v = view();
        assert_eq!(
            status_of(CvTemplate::Classic, &v, AtsCheckId::ColumnReliance),
            AtsStatus::Warn
        );
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::ColumnReliance),
            AtsStatus::Pass
        );
    }

    #[test]
    fn overly_long_passes_under_threshold() {
        // persona-001 tailored to top-3 per role across 5 roles = 15 bullets < 30.
        let v = view();
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::OverlyLong),
            AtsStatus::Pass
        );
    }

    #[test]
    fn overly_long_warns_when_over_threshold() {
        // A synthetic CV with > 30 description-bearing achievements (one role, 31 bullets).
        // persona-001 holds only 17 total, so we build an explicit over-threshold doc.
        let bullets: Vec<String> = (0..(OVERLY_LONG_ACHIEVEMENTS + 1))
            .map(|i| format!(r#"{{"id":"e0_b{i}","description":"achievement number {i}"}}"#))
            .collect();
        let doc = format!(
            r#"{{"schemaVersion":"1.0.0","person":{{}},"experience":[
                {{"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020",
                 "achievementsTasks":[{}]}}]}}"#,
            bullets.join(",")
        );
        let cv = MasterCv::from_json(&doc).unwrap();
        // top_n high enough to keep all 31 bullets surfaced.
        let v = tailor(&cv, &job(), 1000);
        let count = description_bearing_count(&v);
        assert!(
            count > OVERLY_LONG_ACHIEVEMENTS,
            "synthetic doc must exceed threshold (got {count})"
        );
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::OverlyLong),
            AtsStatus::Warn
        );
    }

    #[test]
    fn non_standard_headings_check_warns_on_odd_vocabulary() {
        // Directly exercise the WARN arm (the public ats_report only sees standard
        // vocabularies; this guards a future template introducing an odd heading).
        let warn = non_standard_headings_check(&["Experience", "My Cool Section"]);
        assert_eq!(warn.status, AtsStatus::Warn);
        assert!(warn.message.contains("My Cool Section"));
        let pass = non_standard_headings_check(&["Experience", "Skills"]);
        assert_eq!(pass.status, AtsStatus::Pass);
    }

    #[test]
    fn non_standard_headings_pass_for_both_shipped_templates() {
        let v = view();
        assert_eq!(
            status_of(CvTemplate::Classic, &v, AtsCheckId::NonStandardHeadings),
            AtsStatus::Pass
        );
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::NonStandardHeadings),
            AtsStatus::Pass
        );
    }

    #[test]
    fn missing_extractable_text_warns_on_empty_doc() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"X"},"experience":[]}"#,
        )
        .unwrap();
        let v = tailor(&cv, &job(), 3);
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::MissingExtractableText),
            AtsStatus::Warn
        );
    }

    #[test]
    fn missing_extractable_text_warns_when_only_blank_descriptions() {
        // An experience with a single whitespace-only description bears no text.
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{},"experience":[
                {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020",
                 "achievementsTasks":[{"id":"e0_b0","description":"   "}]}]}"#,
        )
        .unwrap();
        let v = tailor(&cv, &job(), 3);
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::MissingExtractableText),
            AtsStatus::Warn
        );
    }

    #[test]
    fn missing_extractable_text_passes_with_real_content() {
        let v = view();
        assert_eq!(
            status_of(CvTemplate::Compact, &v, AtsCheckId::MissingExtractableText),
            AtsStatus::Pass
        );
    }

    #[test]
    fn unusual_font_always_passes() {
        let v = view();
        for t in [CvTemplate::Classic, CvTemplate::Compact] {
            assert_eq!(status_of(t, &v, AtsCheckId::UnusualFont), AtsStatus::Pass);
        }
    }

    #[test]
    fn report_is_deterministic_in_order_and_content() {
        let v = view();
        let a = ats_report(CvTemplate::Compact, &v);
        let b = ats_report(CvTemplate::Compact, &v);
        assert_eq!(a, b);
        // fixed coordinate order (R-ATS-7)
        let ids: Vec<AtsCheckId> = a.checks.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![
                AtsCheckId::ColumnReliance,
                AtsCheckId::OverlyLong,
                AtsCheckId::NonStandardHeadings,
                AtsCheckId::MissingExtractableText,
                AtsCheckId::UnusualFont,
            ]
        );
    }

    #[test]
    fn ats_report_is_read_only() {
        // R-ATS-2: the view is unchanged after the call.
        let v = view();
        let before = v.cv.to_json().unwrap();
        let _ = ats_report(CvTemplate::Classic, &v);
        assert_eq!(v.cv.to_json().unwrap(), before);
    }

    #[test]
    fn check_lookup_returns_none_for_absent() {
        // AtsReport::check None arm is reachable only on a hand-built empty report.
        let empty = AtsReport { checks: vec![] };
        assert!(empty.check(AtsCheckId::UnusualFont).is_none());
    }

    #[test]
    fn enum_serde_round_trips() {
        for id in [
            AtsCheckId::ColumnReliance,
            AtsCheckId::OverlyLong,
            AtsCheckId::NonStandardHeadings,
            AtsCheckId::MissingExtractableText,
            AtsCheckId::UnusualFont,
        ] {
            let j = serde_json::to_string(&id).unwrap();
            let back: AtsCheckId = serde_json::from_str(&j).unwrap();
            assert_eq!(id, back);
        }
        let j = serde_json::to_string(&AtsStatus::Warn).unwrap();
        assert_eq!(
            serde_json::from_str::<AtsStatus>(&j).unwrap(),
            AtsStatus::Warn
        );
    }
}
