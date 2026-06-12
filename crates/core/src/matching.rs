//! C3 — Match primitive (§A).
//!
//! A requirement is **matched** iff any of its normalized tokens equals
//! (case-insensitive) a token or alias of any of: any skill (all four categories),
//! any experience `tag`, or any token of an achievement `description`/`tags`.
//! Aliases declared on `skill.aliases[]` participate in normalization too.

use crate::normalize::{normalized_set, seed_aliases, tokenize};
use crate::types::MasterCv;
use std::collections::{HashMap, HashSet};

/// Which CV namespace an evidence id belongs to (R-KWC-2). Distinguishes
/// always-rendered sections (`Skill`, `Experience`) from achievement bullets, which
/// the tailored view may prune — so a caller can gate only `Achievement` ids on the
/// surfaced selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    /// A skill's declared `evidenceIds` — the Skills section renders unconditionally.
    Skill,
    /// An experience `id` — the experience entry renders unconditionally.
    Experience,
    /// An achievement bullet `id` — may be dropped by tailoring (top-N).
    Achievement,
}

/// An evidence id tagged with the namespace it came from (R-KWC-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceId {
    pub id: String,
    pub kind: EvidenceKind,
}

/// The candidate pool: every normalized, alias-expanded token the master CV exposes
/// for matching, plus the per-skill declared aliases folded into the alias map.
pub struct Candidate {
    tokens: HashSet<String>,
    /// Seed aliases extended with every `skill.aliases[]` declaration, so a
    /// requirement token (e.g. "ecmascript") expands toward the skill name on the
    /// REQUIREMENT side too (§A: declared aliases participate in normalization).
    aliases: HashMap<String, String>,
}

impl Candidate {
    /// Build the candidate token universe from a master CV.
    pub fn from_master(cv: &MasterCv) -> Self {
        let mut aliases = seed_aliases();
        // Fold skill.aliases[] into the alias map: each declared alias maps to the
        // skill name (so e.g. a skill "JavaScript" with alias "ECMAScript" lets a
        // requirement token "ecmascript" expand toward "javascript").
        for list in [
            &cv.programming_languages,
            &cv.skills,
            &cv.tools_technologies,
            &cv.as_a_services,
        ] {
            for s in list {
                for al in &s.aliases {
                    for t in tokenize(al) {
                        aliases.insert(t, s.name.to_lowercase());
                    }
                }
            }
        }

        let mut tokens: HashSet<String> = HashSet::new();
        let mut add = |phrase: &str, aliases: &HashMap<String, String>| {
            for t in normalized_set(phrase, aliases) {
                tokens.insert(t);
            }
        };

        for list in [
            &cv.programming_languages,
            &cv.skills,
            &cv.tools_technologies,
            &cv.as_a_services,
        ] {
            for s in list {
                add(&s.name, &aliases);
            }
        }
        for e in &cv.experience {
            for tag in &e.tags {
                add(tag, &aliases);
            }
            for a in &e.achievements_tasks {
                add(&a.description, &aliases);
                for tag in &a.tags {
                    add(tag, &aliases);
                }
            }
        }

        Candidate { tokens, aliases }
    }

    /// True iff `requirement` is matched (§A): any normalized requirement token is
    /// present in the candidate universe.
    pub fn matches(&self, requirement: &str) -> bool {
        normalized_set(requirement, &self.aliases)
            .iter()
            .any(|t| self.tokens.contains(t))
    }

    /// Evidence ids whose owning record contributes a token that satisfies this
    /// requirement — used by the coverage report (§B). Scans skills' evidenceIds,
    /// experience ids, and achievement ids. Deduped + sorted by id.
    ///
    /// Delegates to [`matching_evidence_ids_kinded`](Self::matching_evidence_ids_kinded)
    /// and drops the kind, so the two methods can never diverge.
    pub fn matching_evidence_ids(&self, cv: &MasterCv, requirement: &str) -> Vec<String> {
        self.matching_evidence_ids_kinded(cv, requirement)
            .into_iter()
            .map(|e| e.id)
            .collect()
    }

    /// Like [`matching_evidence_ids`](Self::matching_evidence_ids) but each id is tagged
    /// with the namespace it came from (`Skill` / `Experience` / `Achievement`), so a
    /// caller can gate only achievement-bullet ids on a surfaced selection while treating
    /// skill-evidence and experience ids as always rendered (R-KWC-2). Deduped (by id) +
    /// sorted by id — identical id ordering to the untagged method.
    pub fn matching_evidence_ids_kinded(
        &self,
        cv: &MasterCv,
        requirement: &str,
    ) -> Vec<EvidenceId> {
        let aliases = &self.aliases;
        let req: HashSet<String> = normalized_set(requirement, aliases).into_iter().collect();
        let mut ids: Vec<EvidenceId> = Vec::new();
        let push = |id: &str, kind: EvidenceKind, ids: &mut Vec<EvidenceId>| {
            // dedupe on id only: the namespaces are disjoint, so the first kind seen for
            // an id is authoritative (matches the untagged method's id-only dedupe).
            if !ids.iter().any(|e| e.id == id) {
                ids.push(EvidenceId {
                    id: id.to_string(),
                    kind,
                });
            }
        };

        for list in [
            &cv.programming_languages,
            &cv.skills,
            &cv.tools_technologies,
            &cv.as_a_services,
        ] {
            for s in list {
                let mut toks: HashSet<String> =
                    normalized_set(&s.name, aliases).into_iter().collect();
                for al in &s.aliases {
                    for t in normalized_set(al, aliases) {
                        toks.insert(t);
                    }
                }
                if toks.intersection(&req).next().is_some() {
                    for ev in &s.evidence_ids {
                        push(ev, EvidenceKind::Skill, &mut ids);
                    }
                }
            }
        }
        for e in &cv.experience {
            let mut exp_match = e
                .tags
                .iter()
                .flat_map(|t| normalized_set(t, aliases))
                .any(|t| req.contains(&t));
            for a in &e.achievements_tasks {
                let mut toks: HashSet<String> = normalized_set(&a.description, aliases)
                    .into_iter()
                    .collect();
                for tag in &a.tags {
                    for t in normalized_set(tag, aliases) {
                        toks.insert(t);
                    }
                }
                if toks.intersection(&req).next().is_some() {
                    push(&a.id, EvidenceKind::Achievement, &mut ids);
                    exp_match = true;
                }
            }
            if exp_match {
                push(&e.id, EvidenceKind::Experience, &mut ids);
            }
        }
        ids.sort_by(|x, y| x.id.cmp(&y.id));
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cv() -> MasterCv {
        let s = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/personas/persona-001.cv.json"
        ))
        .unwrap();
        MasterCv::from_json(&s).unwrap()
    }

    #[test]
    fn matches_via_skill() {
        let c = Candidate::from_master(&cv());
        assert!(c.matches("Python"));
        assert!(c.matches("PostgreSQL"));
    }

    #[test]
    fn matches_case_insensitive() {
        let c = Candidate::from_master(&cv());
        assert!(c.matches("pYThOn"));
    }

    #[test]
    fn matches_via_experience_tag() {
        let c = Candidate::from_master(&cv());
        // "Redis" appears only as an experience tag in persona-001
        assert!(c.matches("Redis"));
    }

    #[test]
    fn matches_via_achievement_description() {
        let c = Candidate::from_master(&cv());
        // "caching" appears in an achievement description, not as a skill/tag
        assert!(c.matches("caching"));
    }

    #[test]
    fn alias_only_match_via_seed() {
        let c = Candidate::from_master(&cv());
        // persona has TypeScript skill; requirement "ts" must match via alias
        assert!(c.matches("ts"));
    }

    #[test]
    fn no_match_returns_false() {
        let c = Candidate::from_master(&cv());
        assert!(!c.matches("Cobol"));
        assert!(!c.matches("Stakeholder management"));
    }

    #[test]
    fn skill_declared_alias_participates() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "programmingLanguages":[{"name":"JavaScript","proficiency":5,"aliases":["ECMAScript"],"evidenceIds":["exp_x"]}],
            "experience":[]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let c = Candidate::from_master(&cv);
        assert!(c.matches("ecmascript"));
    }

    #[test]
    fn matching_evidence_ids_collects_achievement_and_experience() {
        let c = Candidate::from_master(&cv());
        let cvv = cv();
        let ids = c.matching_evidence_ids(&cvv, "caching");
        assert!(ids.iter().any(|i| i.starts_with("exp_1_0_b0")));
        assert!(ids.iter().any(|i| i == "exp_1_0"));
    }

    #[test]
    fn matching_evidence_ids_collects_skill_evidence() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "skills":[{"name":"Rust","proficiency":3,"aliases":[],"evidenceIds":["exp_z_b1"]}],
            "experience":[]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let c = Candidate::from_master(&cv);
        assert_eq!(
            c.matching_evidence_ids(&cv, "Rust"),
            vec!["exp_z_b1".to_string()]
        );
    }

    #[test]
    fn kinded_classifies_each_namespace() {
        // One doc that exercises all three kinds for one requirement: a skill with a
        // declared evidenceId, plus an achievement (and its owning experience) that
        // matches the same token. Skill→Skill, experience id→Experience, bullet→Achievement.
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "skills":[{"name":"Caching","proficiency":3,"aliases":[],"evidenceIds":["skill_ev"]}],
            "experience":[
              {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","achievementsTasks":[
                {"id":"e0_b0","description":"built a caching layer","metrics":[],"evidenceStrength":0.5}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let c = Candidate::from_master(&cv);
        let kinded = c.matching_evidence_ids_kinded(&cv, "caching");
        let kind_of = |id: &str| kinded.iter().find(|e| e.id == id).map(|e| e.kind);
        assert_eq!(kind_of("skill_ev"), Some(EvidenceKind::Skill));
        assert_eq!(kind_of("e0"), Some(EvidenceKind::Experience));
        assert_eq!(kind_of("e0_b0"), Some(EvidenceKind::Achievement));
        // sorted by id, deduped — same id ordering as the untagged method
        let plain: Vec<String> = kinded.iter().map(|e| e.id.clone()).collect();
        assert_eq!(plain, c.matching_evidence_ids(&cv, "caching"));
    }

    #[test]
    fn empty_requirement_no_match() {
        let c = Candidate::from_master(&cv());
        assert!(!c.matches(""));
        assert!(c.matching_evidence_ids(&cv(), "").is_empty());
    }

    /// Covers the achievement-`tags` candidate path + the skill-alias and
    /// achievement-tag evidence-id collection branches (matching.rs 66-69, 105-130).
    #[test]
    fn matches_and_collects_via_achievement_tags_and_skill_aliases() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},
            "skills":[{"name":"Observability","proficiency":4,"aliases":["o11y"],"evidenceIds":["skill_ev_1"]}],
            "experience":[
              {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020","tags":["Platform"],
               "achievementsTasks":[
                 {"id":"e0_b0","description":"built dashboards","tags":["grafana","alerting"]}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let c = Candidate::from_master(&cv);
        // achievement-tag candidate path
        assert!(c.matches("grafana"));
        // skill-declared alias on the requirement side
        assert!(c.matches("o11y"));
        // evidence-id collection via achievement tag → achievement id + experience id
        let ids = c.matching_evidence_ids(&cv, "alerting");
        assert!(ids.contains(&"e0_b0".to_string()));
        assert!(ids.contains(&"e0".to_string()));
        // evidence-id collection via skill alias → skill evidenceIds
        let sids = c.matching_evidence_ids(&cv, "o11y");
        assert!(sids.contains(&"skill_ev_1".to_string()));
    }
}
