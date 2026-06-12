//! C6 — Bullet ranking (§D), summary pick (§D), tailored-view assembly (§H).
//!
//! §D ranking key (descending priority):
//!   1. matches a must-have requirement
//!   2. has non-empty `metrics`
//!   3. higher `evidenceStrength`
//!   4. recency (parsed owning-experience `startDate`; later is better)
//!   5. `emphasise` flag
//!   6. final tie-break on achievement `id` (total order → reproducible)
//!
//! §D summary = the `summaryVariants` entry with the most requirement-token overlap,
//! taken verbatim, provenance `summary:<index>`.
//!
//! §H tailored view = a filtered/reordered `MasterCv` (same type → schema-conformant).
//! Selection keeps top-N achievements per role, ranked; the view is what Typst renders.

use crate::job::NormalizedJob;
use crate::normalize::{normalized_set, seed_aliases};
use crate::types::{Achievement, Experience, MasterCv};
use std::collections::HashSet;

/// Default cap on achievements rendered per role (one–two page fit, §D).
pub const DEFAULT_TOP_N: usize = 3;

/// Parse a `startDate` like "Jul 2023" / "Present" into a sortable (year, month)
/// key. Unknown/"Present" sorts as most-recent. Deterministic; never panics.
fn recency_key(date: &str) -> (i32, u32) {
    let d = date.trim();
    if d.eq_ignore_ascii_case("present") {
        return (i32::MAX, 12);
    }
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let mut month = 0u32;
    let mut year = 0i32;
    for part in d.split_whitespace() {
        let lp = part.to_lowercase();
        if let Some(idx) = months.iter().position(|m| lp.starts_with(m)) {
            month = idx as u32 + 1;
        } else if let Ok(y) = part.parse::<i32>() {
            year = y;
        }
    }
    (year, month)
}

/// True if an achievement matches any must-have requirement (§D key 1). Builds a
/// per-achievement token set from its description + tags + its experience's tags,
/// then checks any must-have requirement token intersects it.
fn matches_must(a: &Achievement, exp: &Experience, must: &[String]) -> bool {
    let aliases = seed_aliases();
    let mut toks: HashSet<String> = normalized_set(&a.description, &aliases)
        .into_iter()
        .collect();
    for tag in &a.tags {
        for t in normalized_set(tag, &aliases) {
            toks.insert(t);
        }
    }
    for tag in &exp.tags {
        for t in normalized_set(tag, &aliases) {
            toks.insert(t);
        }
    }
    must.iter()
        .any(|m| normalized_set(m, &aliases).iter().any(|t| toks.contains(t)))
}

/// The §D ranking sort key for one achievement (all descending → we negate where
/// needed and rely on the derived Ord of the tuple, reversed at the call site).
#[derive(PartialEq, Eq)]
struct RankKey {
    must: bool,
    has_metrics: bool,
    strength_milli: i64, // evidenceStrength * 1000, truncated (total order on f64)
    recency: (i32, u32),
    emphasise: bool,
    id: String,
}

impl RankKey {
    fn build(a: &Achievement, exp: &Experience, must: &[String]) -> Self {
        RankKey {
            must: matches_must(a, exp, must),
            has_metrics: !a.metrics.is_empty(),
            strength_milli: (a.evidence_strength.unwrap_or(0.0) * 1000.0) as i64,
            recency: recency_key(&exp.start_date),
            emphasise: a.emphasise.unwrap_or(false),
            id: a.id.clone(),
        }
    }

    /// Compare for DESCENDING priority: higher must/metrics/strength/recency/emphasise
    /// first; final ascending tie-break on id for a stable total order.
    fn cmp_desc(&self, other: &Self) -> std::cmp::Ordering {
        other
            .must
            .cmp(&self.must)
            .then(other.has_metrics.cmp(&self.has_metrics))
            .then(other.strength_milli.cmp(&self.strength_milli))
            .then(other.recency.cmp(&self.recency))
            .then(other.emphasise.cmp(&self.emphasise))
            // final ascending tie-break on id → stable total order (Ordering::cmp
            // returns Equal when ids are equal, no separate branch needed).
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Select + rank achievements within a single experience (top-N), per §D.
fn rank_within(exp: &Experience, must: &[String], top_n: usize) -> Vec<Achievement> {
    let mut items: Vec<&Achievement> = exp.achievements_tasks.iter().collect();
    items.sort_by(|x, y| RankKey::build(x, exp, must).cmp_desc(&RankKey::build(y, exp, must)));
    items.into_iter().take(top_n).cloned().collect()
}

/// Pick the best summary variant (§D): most requirement-token overlap, verbatim.
/// Returns (text, provenance "summary:<index>"). None if there are no variants.
pub fn pick_summary(cv: &MasterCv, job: &NormalizedJob) -> Option<(String, String)> {
    if cv.summary_variants.is_empty() {
        return None;
    }
    let aliases = seed_aliases();
    let req_tokens: HashSet<String> = job
        .requirements
        .must_have
        .iter()
        .chain(job.requirements.nice_to_have.iter())
        .flat_map(|r| normalized_set(r, &aliases))
        .collect();

    let mut best_idx = 0usize;
    let mut best_overlap = -1i64;
    for (i, v) in cv.summary_variants.iter().enumerate() {
        let vt: HashSet<String> = normalized_set(v, &aliases).into_iter().collect();
        let overlap = vt.intersection(&req_tokens).count() as i64;
        // strictly-greater keeps the lowest index on ties → deterministic
        if overlap > best_overlap {
            best_overlap = overlap;
            best_idx = i;
        }
    }
    Some((
        cv.summary_variants[best_idx].clone(),
        format!("summary:{best_idx}"),
    ))
}

/// The top-matching must-have requirement string for a selected achievement id
/// (R-ADV-12). Reuses the §D `matches_must` logic: for the achievement with this id,
/// return the FIRST must-have requirement (in job order) that the achievement matches
/// (its description/tags/experience-tags token set intersects the requirement tokens).
///
/// Deterministic no-match fallback: when the achievement matches NO single must-have
/// requirement (or the id is not found, or there are no must-haves), return the JOINED
/// must-have list (`must_have.join(", ")`). This guarantees every rewrite carries a
/// non-empty requirement string when the job has any must-haves; an empty job yields
/// an empty string (honest — there is nothing to tailor toward). Pinned by test.
pub fn requirement_for(cv: &MasterCv, job: &NormalizedJob, evidence_id: &str) -> String {
    let must = &job.requirements.must_have;
    // find the owning experience + achievement for this id
    for exp in &cv.experience {
        for a in &exp.achievements_tasks {
            if a.id == evidence_id {
                for req in must {
                    if matches_must(a, exp, std::slice::from_ref(req)) {
                        return req.clone();
                    }
                }
            }
        }
    }
    must.join(", ")
}

/// The tailored view (§H): a `MasterCv` that is a filtered/reordered copy of the
/// master, so it conforms to master-cv.schema.json and renders via `classic.typ`.
/// The chosen summary is placed at `summaryVariants[0]` so the template surfaces it
/// (the template reads `professionalDescription`; we also set that for rendering).
#[derive(Debug, Clone)]
pub struct TailoredView {
    pub cv: MasterCv,
    /// Provenance string for the chosen summary, e.g. "summary:0" (None if no variants).
    pub summary_provenance: Option<String>,
    /// Ordered (achievement id) selection that backs the view, for the ledger/UI.
    pub selected_ids: Vec<String>,
}

/// Assemble the tailored view (§D + §H) for a master CV against a job.
pub fn tailor(cv: &MasterCv, job: &NormalizedJob, top_n: usize) -> TailoredView {
    let must = &job.requirements.must_have;

    // Order experiences by recency (most recent first) for a stable, tailored read.
    let mut exps: Vec<Experience> = cv
        .experience
        .iter()
        .filter(|e| !e.hide.unwrap_or(false))
        .cloned()
        .collect();
    exps.sort_by(|a, b| {
        recency_key(&b.start_date)
            .cmp(&recency_key(&a.start_date))
            .then(a.id.cmp(&b.id))
    });

    let mut selected_ids = Vec::new();
    for e in exps.iter_mut() {
        let ranked = rank_within(e, must, top_n);
        for a in &ranked {
            selected_ids.push(a.id.clone());
        }
        e.achievements_tasks = ranked;
    }

    let mut view = cv.clone();
    view.experience = exps;

    let summary_provenance = pick_summary(cv, job).map(|(text, prov)| {
        // place chosen summary first + into the rendered description slot
        view.summary_variants = vec![text.clone()];
        view.person.professional_description = Some(text);
        prov
    });

    TailoredView {
        cv: view,
        summary_provenance,
        selected_ids,
    }
}

#[cfg(test)]
mod tests {
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
            requirements: crate::job::Requirements {
                must_have: vec!["caching".into(), "Python".into()],
                nice_to_have: vec!["Mentored".into()],
            },
            keywords: vec![],
        }
    }

    #[test]
    fn recency_present_is_max() {
        assert!(recency_key("Present") > recency_key("Jan 2025"));
        assert!(recency_key("Jul 2023") > recency_key("Feb 2021"));
        assert_eq!(recency_key("garbage"), (0, 0));
    }

    #[test]
    fn ranking_is_total_order_stable() {
        let v1 = tailor(&master(), &job(), DEFAULT_TOP_N);
        let v2 = tailor(&master(), &job(), DEFAULT_TOP_N);
        assert_eq!(v1.selected_ids, v2.selected_ids);
    }

    #[test]
    fn must_match_ranks_first() {
        // exp_1_0 has b0 (caching=must, has metrics) — must rank above b1/b2
        let v = tailor(&master(), &job(), DEFAULT_TOP_N);
        let first_exp = &v.cv.experience[0];
        assert_eq!(first_exp.id, "exp_1_0"); // most recent (Present)
        assert_eq!(first_exp.achievements_tasks[0].id, "exp_1_0_b0");
    }

    #[test]
    fn metrics_break_tie_before_no_metrics() {
        // construct two achievements, equal except metrics
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
              {"id":"e0_b0","description":"alpha","metrics":[],"evidenceStrength":0.5},
              {"id":"e0_b1","description":"beta","metrics":["10%"],"evidenceStrength":0.5}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let j = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let v = tailor(&cv, &j, DEFAULT_TOP_N);
        assert_eq!(v.cv.experience[0].achievements_tasks[0].id, "e0_b1");
    }

    #[test]
    fn id_tie_break_deterministic() {
        // fully identical except id → ascending id wins
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
              {"id":"e0_b9","description":"same","metrics":[],"evidenceStrength":0.5},
              {"id":"e0_b1","description":"same","metrics":[],"evidenceStrength":0.5}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let j = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let v = tailor(&cv, &j, DEFAULT_TOP_N);
        assert_eq!(v.cv.experience[0].achievements_tasks[0].id, "e0_b1");
    }

    #[test]
    fn top_n_caps_per_role() {
        let v = tailor(&master(), &job(), 1);
        assert!(v
            .cv
            .experience
            .iter()
            .all(|e| e.achievements_tasks.len() <= 1));
    }

    #[test]
    fn summary_pick_verbatim_with_provenance() {
        let (text, prov) = pick_summary(&master(), &job()).unwrap();
        assert_eq!(prov, "summary:0");
        assert!(text.starts_with("Backend engineer"));
    }

    #[test]
    fn summary_pick_chooses_max_overlap() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "summaryVariants":["nothing relevant here","python and caching expert"],
            "experience":[]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let (text, prov) = pick_summary(&cv, &job()).unwrap();
        assert_eq!(prov, "summary:1");
        assert!(text.contains("python"));
    }

    #[test]
    fn summary_none_when_no_variants() {
        let cv = MasterCv::from_json(r#"{"schemaVersion":"1.0.0","person":{},"experience":[]}"#)
            .unwrap();
        assert!(pick_summary(&cv, &job()).is_none());
    }

    #[test]
    fn view_is_schema_conformant_master_cv() {
        let v = tailor(&master(), &job(), DEFAULT_TOP_N);
        // serialize the view, re-parse as a MasterCv: it IS a master CV (§H)
        let s = v.cv.to_json().unwrap();
        let reparsed = MasterCv::from_json(&s).unwrap();
        assert_eq!(reparsed.schema_version, "1.0.0");
        assert!(v.summary_provenance.is_some());
    }

    #[test]
    fn must_match_via_achievement_and_experience_tags() {
        // covers matches_must achievement-`tags` + experience-`tags` paths (tailor 54-58)
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","tags":["Kafka"],
             "achievementsTasks":[
               {"id":"e0_b0","description":"x","tags":["streaming"],"metrics":[],"evidenceStrength":0.5},
               {"id":"e0_b1","description":"y","tags":[],"metrics":[],"evidenceStrength":0.5}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        // must-have "streaming" matches via achievement tag → b0 ranks first
        let j = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec!["streaming".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let v = tailor(&cv, &j, DEFAULT_TOP_N);
        assert_eq!(v.cv.experience[0].achievements_tasks[0].id, "e0_b0");
        // must-have "Kafka" matches via experience tag → both achievements get the must bonus
        let j2 = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec!["Kafka".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let v2 = tailor(&cv, &j2, DEFAULT_TOP_N);
        assert_eq!(v2.cv.experience[0].achievements_tasks.len(), 2);
    }

    #[test]
    fn requirement_for_returns_top_match() {
        // R-ADV-12: exp_1_0_b0 matches "caching" (a must-have) → that requirement is returned.
        let m = master();
        let j = job(); // must_have = ["caching", "Python"]
        let req = requirement_for(&m, &j, "exp_1_0_b0");
        assert_eq!(
            req, "caching",
            "top-matching must-have is returned verbatim"
        );
    }

    #[test]
    fn requirement_for_falls_back_to_joined_must() {
        // R-ADV-12: an achievement matching NO single must-have → joined must-have list.
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
              {"id":"e0_b0","description":"organised the office party"}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let j = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec!["Rust".into(), "Kubernetes".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        assert_eq!(requirement_for(&cv, &j, "e0_b0"), "Rust, Kubernetes");
    }

    #[test]
    fn requirement_for_unknown_id_falls_back() {
        // id not present in the CV → joined must-have list (deterministic, non-empty).
        let m = master();
        let j = job();
        assert_eq!(requirement_for(&m, &j, "GHOST_id"), "caching, Python");
    }

    #[test]
    fn requirement_for_empty_job_is_empty() {
        // no must-haves → empty string (honest: nothing to tailor toward).
        let m = master();
        let empty = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        assert_eq!(requirement_for(&m, &empty, "exp_1_0_b0"), "");
    }

    #[test]
    fn hidden_experience_excluded() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","hide":true,
             "achievementsTasks":[{"id":"e0_b0","description":"x"}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let j = NormalizedJob {
            title: "".into(),
            company: "".into(),
            location: "".into(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: vec![],
                nice_to_have: vec![],
            },
            keywords: vec![],
        };
        let v = tailor(&cv, &j, DEFAULT_TOP_N);
        assert!(v.cv.experience.is_empty());
    }
}
