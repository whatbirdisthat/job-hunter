//! Item #6, capability C — keyword-coverage panel (PURE, no IO).
//!
//! `keyword_coverage(view, job)` reports, per job keyword, whether it is FOUND or
//! MISSING in the TAILORED view, and for a FOUND keyword WHERE it surfaces (R-KWC-1/2).
//! It keys off `job.requirements.must_have` / `nice_to_have` (DISCUSS-KWC-KEY RESOLVED;
//! `job.keywords` stays reserved), distinguishing the two classes (R-KWC-3).
//!
//! Distinct from `coverage_report`, which runs over the MASTER CV: this runs over the
//! tailored view (selected/reordered evidence), so a keyword found ONLY in a dropped
//! ACHIEVEMENT BULLET reads as not-surfaced (R-KWC-4). The "where" is computed via
//! `Candidate::matching_evidence_ids_kinded(&view.cv, kw)`: achievement-bullet ids are
//! gated on `view.selected_ids` (only kept bullets count), while skill-evidence and
//! experience ids are ALWAYS surfaced — the Skills/Summary sections and experience
//! entries render unconditionally in both templates, so their evidence is never pruned
//! by tailoring (R-KWC-2, DISCUSS-KWC-SEL RESOLVED). Evidence ids across multiple
//! contributing sections are deduped and deterministically ordered (R-KWC-6).
//!
//! VISIBILITY-ONLY (R-KWC-5): `&` borrows throughout, no mutation, no insertion,
//! reordering, or fabrication of keywords/content. Deterministic (R-KWC-8).

use crate::job::NormalizedJob;
use crate::matching::{Candidate, EvidenceKind};
use crate::tailor::TailoredView;
use serde::{Deserialize, Serialize};

/// Whether a keyword is a hard requirement or a nice-to-have (R-KWC-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeywordClass {
    MustHave,
    NiceToHave,
}

/// One keyword's coverage result over the tailored view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordHit {
    pub keyword: String,
    pub class: KeywordClass,
    /// Surfaced evidence ids (deduped, sorted). Empty for a MISSING keyword (R-KWC-7).
    #[serde(rename = "evidenceIds")]
    pub evidence_ids: Vec<String>,
}

/// The keyword-coverage report: found vs missing, each carrying its class (R-KWC-1/3/7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordCoverage {
    pub found: Vec<KeywordHit>,
    pub missing: Vec<KeywordHit>,
}

/// Surfaced evidence ids for `keyword` in the tailored view (R-KWC-2): the KINDED
/// `matching_evidence_ids_kinded` over the view's CV, where ONLY achievement-bullet ids
/// are gated on the surfaced `selected_ids` — skill-evidence and experience ids are
/// ALWAYS surfaced, because the Skills/Summary sections and the experience entries
/// themselves render unconditionally in both templates (classic.typ + compact.typ),
/// independent of which bullets the tailored view kept. The `selected_ids` gate exists
/// only to drop a keyword whose sole evidence is a LOWER-RANKED ACHIEVEMENT BULLET that
/// tailoring pruned (R-KWC-4). Deduped + sorted (R-KWC-6) — the kinded method already
/// sorts by id + dedupes; filtering preserves that order.
fn surfaced_evidence(view: &TailoredView, candidate: &Candidate, keyword: &str) -> Vec<String> {
    let selected: std::collections::HashSet<&String> = view.selected_ids.iter().collect();
    candidate
        .matching_evidence_ids_kinded(&view.cv, keyword)
        .into_iter()
        .filter(|e| match e.kind {
            // an achievement bullet only counts if the tailored view actually kept it
            EvidenceKind::Achievement => selected.contains(&e.id),
            // skills + experiences are always rendered → always surfaced
            EvidenceKind::Skill | EvidenceKind::Experience => true,
        })
        .map(|e| e.id)
        .collect()
}

/// The PURE keyword-coverage report over the TAILORED view (R-KWC-1). Read-only,
/// deterministic (R-KWC-5/8). Found keywords carry their surfaced evidence ids; absent
/// keywords land in `missing` with an empty evidence list (R-KWC-7).
pub fn keyword_coverage(view: &TailoredView, job: &NormalizedJob) -> KeywordCoverage {
    let candidate = Candidate::from_master(&view.cv);
    let mut found = Vec::new();
    let mut missing = Vec::new();

    let classes = [
        (KeywordClass::MustHave, &job.requirements.must_have),
        (KeywordClass::NiceToHave, &job.requirements.nice_to_have),
    ];
    for (class, keywords) in classes {
        for kw in keywords {
            let ids = surfaced_evidence(view, &candidate, kw);
            let hit = KeywordHit {
                keyword: kw.clone(),
                class,
                evidence_ids: ids.clone(),
            };
            if ids.is_empty() {
                missing.push(hit);
            } else {
                found.push(hit);
            }
        }
    }

    KeywordCoverage { found, missing }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Requirements;
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

    fn job_with(must: Vec<&str>, nice: Vec<&str>) -> NormalizedJob {
        NormalizedJob {
            title: "Senior Backend Engineer".into(),
            company: "Acme".into(),
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: must.into_iter().map(String::from).collect(),
                nice_to_have: nice.into_iter().map(String::from).collect(),
            },
            keywords: vec![],
        }
    }

    fn found_kw<'a>(c: &'a KeywordCoverage, kw: &str) -> Option<&'a KeywordHit> {
        c.found.iter().find(|h| h.keyword == kw)
    }
    fn missing_kw<'a>(c: &'a KeywordCoverage, kw: &str) -> Option<&'a KeywordHit> {
        c.missing.iter().find(|h| h.keyword == kw)
    }

    #[test]
    fn must_have_found_with_non_empty_surfaced_evidence() {
        // "caching" appears in exp_1_0_b0 — a top-ranked, surfaced bullet.
        let job = job_with(vec!["caching"], vec![]);
        let view = tailor(&master(), &job, 3);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "caching").expect("caching is found");
        assert_eq!(hit.class, KeywordClass::MustHave);
        assert!(!hit.evidence_ids.is_empty());
        assert!(hit.evidence_ids.iter().any(|i| i == "exp_1_0_b0"));
    }

    #[test]
    fn absent_must_have_is_missing_with_empty_evidence() {
        // "Cobol" is nowhere in the persona → MISSING, empty evidence (R-KWC-7).
        let job = job_with(vec!["Cobol"], vec![]);
        let view = tailor(&master(), &job, 3);
        let cov = keyword_coverage(&view, &job);
        let hit = missing_kw(&cov, "Cobol").expect("Cobol is missing");
        assert_eq!(hit.class, KeywordClass::MustHave);
        assert!(hit.evidence_ids.is_empty());
        assert!(found_kw(&cov, "Cobol").is_none());
    }

    #[test]
    fn nice_to_have_class_is_distinguished() {
        // "Mentored" surfaces via the mentored-engineers achievement descriptions (the
        // "Mentoring" SKILL has empty evidenceIds, so we key on the bullet token here).
        let job = job_with(vec![], vec!["Mentored"]);
        let view = tailor(&master(), &job, 3);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "Mentored").expect("Mentored found");
        assert_eq!(hit.class, KeywordClass::NiceToHave);
        assert!(!hit.evidence_ids.is_empty());
    }

    #[test]
    fn multi_section_keyword_dedupes_and_orders_evidence() {
        // "caching" appears in exp_1_0_b0, exp_1_1_b2, exp_1_4_b0 (multiple roles). With a
        // generous top-N all are surfaced → all contributing ids, deduped + sorted.
        let job = job_with(vec!["caching"], vec![]);
        let view = tailor(&master(), &job, 100);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "caching").unwrap();
        // sorted + deduped
        let mut sorted = hit.evidence_ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(hit.evidence_ids, sorted);
        // at least two distinct contributing bullets across sections
        let bullet_ids: Vec<&String> = hit
            .evidence_ids
            .iter()
            .filter(|i| i.contains("_b"))
            .collect();
        assert!(
            bullet_ids.len() >= 2,
            "caching surfaces in multiple sections"
        );
    }

    #[test]
    fn must_have_via_declared_skill_name_is_found() {
        // REGRESSION (item #6 C, HIGH correctness): a must-have that matches via a
        // DECLARED SKILL NAME must read as FOUND — the Skills section renders
        // unconditionally in both templates, independent of the surfaced bullets. In
        // persona-001 "Python" is a declared programmingLanguage whose evidence resolves
        // to an EXPERIENCE id (exp_1_0 tag); experience ids are ALWAYS surfaced. Even at
        // top_n=1 (most bullets pruned) it must not be filtered out.
        let job = job_with(vec!["Python"], vec![]);
        let view = tailor(&master(), &job, 1);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "Python").expect("Python is a declared skill → found");
        assert_eq!(hit.class, KeywordClass::MustHave);
        assert!(
            !hit.evidence_ids.is_empty(),
            "skill/experience evidence is always surfaced"
        );
        assert!(found_kw(&cov, "Python").is_some());
    }

    #[test]
    fn skill_evidence_id_is_always_surfaced() {
        // A keyword matching a skill that declares NON-EMPTY skill evidenceIds must read
        // as FOUND with that skill-evidence id surfaced, even though the id is not an
        // achievement bullet in `selected_ids`. The Skills section renders the skill
        // unconditionally.
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "skills":[{"name":"Rust","proficiency":3,"aliases":[],"evidenceIds":["skill_ev_rust"]}],
            "experience":[
              {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
                {"id":"e0_b0","description":"unrelated bullet","metrics":["1"],"evidenceStrength":0.9}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let job = job_with(vec!["Rust"], vec![]);
        let view = tailor(&cv, &job, 1);
        // selected_ids holds only the achievement bullet, NOT the skill evidence id.
        assert_eq!(view.selected_ids, vec!["e0_b0".to_string()]);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "Rust").expect("declared skill with evidenceIds → found");
        assert!(hit.evidence_ids.contains(&"skill_ev_rust".to_string()));
    }

    #[test]
    fn must_have_via_experience_token_is_found() {
        // A keyword matching only at the EXPERIENCE level (an experience tag) resolves to
        // an experience id, which is always surfaced (the experience entry itself renders).
        // "PostgreSQL" is an experience tag (not an achievement bullet token) in persona-001.
        let job = job_with(vec!["PostgreSQL"], vec![]);
        let view = tailor(&master(), &job, 1);
        let cov = keyword_coverage(&view, &job);
        let hit = found_kw(&cov, "PostgreSQL").expect("experience-tag keyword → found");
        assert!(hit.evidence_ids.iter().any(|i| !i.contains("_b")));
    }

    #[test]
    fn keyword_found_only_in_dropped_bullet_is_not_surfaced() {
        // With top-N = 1, only the single top bullet per role survives. A must-have that
        // matches only a dropped lower-ranked bullet must read as MISSING (R-KWC-2/4).
        // Build a CV where "obscuretoken" lives only on a low-ranked bullet.
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
              {"id":"e0_b0","description":"top ranked","metrics":["1"],"evidenceStrength":0.9},
              {"id":"e0_b1","description":"mentions obscuretoken here","metrics":[],"evidenceStrength":0.1}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        // Tailor with an EMPTY-must job so ranking is metric/strength based → e0_b0 (metrics
        // + 0.9 strength) outranks e0_b1 and is the sole survivor at top_n = 1. (Tailoring
        // with must=["obscuretoken"] would instead PROMOTE e0_b1 via the must-bonus, which
        // is not the scenario we want to prove.)
        let tailor_job = job_with(vec![], vec![]);
        let view = tailor(&cv, &tailor_job, 1);
        assert_eq!(view.selected_ids, vec!["e0_b0".to_string()]);
        // Now QUERY coverage for the obscuretoken must-have: its only evidence (e0_b1) was
        // dropped → not surfaced → MISSING (R-KWC-2/4).
        let query_job = job_with(vec!["obscuretoken"], vec![]);
        let cov = keyword_coverage(&view, &query_job);
        let hit = missing_kw(&cov, "obscuretoken").expect("dropped → missing");
        assert!(hit.evidence_ids.is_empty());
    }

    #[test]
    fn coverage_is_deterministic() {
        let job = job_with(vec!["caching", "Python"], vec!["Mentoring"]);
        let view = tailor(&master(), &job, 3);
        let a = keyword_coverage(&view, &job);
        let b = keyword_coverage(&view, &job);
        assert_eq!(a, b);
    }

    #[test]
    fn coverage_is_read_only() {
        // R-KWC-5: the view is unchanged after the call.
        let job = job_with(vec!["caching"], vec![]);
        let view = tailor(&master(), &job, 3);
        let before = view.cv.to_json().unwrap();
        let _ = keyword_coverage(&view, &job);
        assert_eq!(view.cv.to_json().unwrap(), before);
    }

    #[test]
    fn empty_job_yields_empty_report() {
        let job = job_with(vec![], vec![]);
        let view = tailor(&master(), &job, 3);
        let cov = keyword_coverage(&view, &job);
        assert!(cov.found.is_empty() && cov.missing.is_empty());
    }

    #[test]
    fn enum_serde_round_trips() {
        let j = serde_json::to_string(&KeywordClass::MustHave).unwrap();
        assert_eq!(
            serde_json::from_str::<KeywordClass>(&j).unwrap(),
            KeywordClass::MustHave
        );
    }
}
