//! C3 — Match primitive (§A).
//!
//! A requirement is **matched** iff any of its normalized tokens equals
//! (case-insensitive) a token or alias of any of: any skill (all four categories),
//! any experience `tag`, or any token of an achievement `description`/`tags`.
//! Aliases declared on `skill.aliases[]` participate in normalization too.

use crate::normalize::{normalized_set, seed_aliases, tokenize};
use crate::types::MasterCv;
use std::collections::{HashMap, HashSet};

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
    /// experience ids, and achievement ids.
    pub fn matching_evidence_ids(&self, cv: &MasterCv, requirement: &str) -> Vec<String> {
        let aliases = &self.aliases;
        let req: HashSet<String> = normalized_set(requirement, aliases).into_iter().collect();
        let mut ids: Vec<String> = Vec::new();
        let push = |id: &str, ids: &mut Vec<String>| {
            if !ids.contains(&id.to_string()) {
                ids.push(id.to_string());
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
                        push(ev, &mut ids);
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
                    push(&a.id, &mut ids);
                    exp_match = true;
                }
            }
            if exp_match {
                push(&e.id, &mut ids);
            }
        }
        ids.sort();
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
