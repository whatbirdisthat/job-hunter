//! C7 — Evidence-ledger guard (§E). The headline wedge.
//!
//! Every **claim-bearing output node** carries a `sourceEvidenceId`:
//!   - CV bullet            → the achievement `id` it was copied from (verbatim)
//!   - CV summary           → `summary:<index>`
//!   - Cover-letter strength → the achievement `id` it wraps
//!   - Cover-letter scaffold → marked `scaffold`, EXEMPT
//!
//! The guard asserts every claim-bearing node has a `sourceEvidenceId` resolvable in
//! the loaded master CV; on failure it BLOCKS export (hard fail) and NAMES the node.
//! Non-vacuous test (mandatory): an injected dangling-id node MUST be blocked.

use crate::tailor::TailoredView;
use crate::types::{CoreError, MasterCv};
use std::collections::HashSet;

/// A node in the evidence ledger — one line of generated output.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerNode {
    /// Stable label used when naming a blocked node, e.g. "cv.bullet[exp_1_0_b0]".
    pub node: String,
    /// The claimed source evidence id, or `summary:<index>`. None only for scaffold.
    pub source_evidence_id: Option<String>,
    /// Scaffold nodes (greeting / why-company) assert no experience claim → exempt.
    pub scaffold: bool,
}

impl LedgerNode {
    pub fn claim(node: impl Into<String>, source: impl Into<String>) -> Self {
        LedgerNode {
            node: node.into(),
            source_evidence_id: Some(source.into()),
            scaffold: false,
        }
    }
    pub fn scaffold(node: impl Into<String>) -> Self {
        LedgerNode {
            node: node.into(),
            source_evidence_id: None,
            scaffold: true,
        }
    }
}

/// The set of resolvable evidence ids in a master CV: every achievement id, plus
/// `summary:<index>` for each summary variant.
pub fn resolvable_ids(cv: &MasterCv) -> HashSet<String> {
    let mut ids: HashSet<String> = HashSet::new();
    for (_, a) in cv.all_achievements() {
        ids.insert(a.id.clone());
    }
    for i in 0..cv.summary_variants.len() {
        ids.insert(format!("summary:{i}"));
    }
    ids
}

/// Build the ledger for a tailored CV view: one node per rendered bullet + the
/// summary node. `master` supplies the resolvable id universe (I2: resolvable in
/// the LOADED master CV, not the filtered view).
pub fn cv_ledger(view: &TailoredView) -> Vec<LedgerNode> {
    let mut nodes = Vec::new();
    if let Some(prov) = &view.summary_provenance {
        nodes.push(LedgerNode::claim("cv.summary", prov.clone()));
    }
    for e in &view.cv.experience {
        for a in &e.achievements_tasks {
            nodes.push(LedgerNode::claim(
                format!("cv.bullet[{}]", a.id),
                a.id.clone(),
            ));
        }
    }
    nodes
}

/// The guard (§E): every claim-bearing node must resolve in `master`. Returns Ok on
/// pass; on failure returns `CoreError::LedgerBlocked` naming the first offending node
/// (and counting all). Scaffold nodes are skipped.
pub fn guard(nodes: &[LedgerNode], master: &MasterCv) -> Result<(), CoreError> {
    let universe = resolvable_ids(master);
    let mut offenders: Vec<String> = Vec::new();
    for n in nodes {
        if n.scaffold {
            continue;
        }
        match &n.source_evidence_id {
            None => offenders.push(format!("{} (missing sourceEvidenceId)", n.node)),
            Some(id) if !universe.contains(id) => {
                offenders.push(format!("{} (dangling sourceEvidenceId '{}')", n.node, id))
            }
            Some(_) => {}
        }
    }
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(CoreError::LedgerBlocked(offenders.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{NormalizedJob, Requirements};
    use crate::tailor::tailor;

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
            title: "T".into(),
            company: "C".into(),
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: vec!["Python".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        }
    }

    #[test]
    fn all_resolvable_passes() {
        let view = tailor(&master(), &job(), 3);
        let nodes = cv_ledger(&view);
        assert!(!nodes.is_empty());
        guard(&nodes, &master()).expect("clean view must pass the guard");
    }

    #[test]
    fn scaffold_node_is_exempt() {
        let nodes = vec![LedgerNode::scaffold("letter.greeting")];
        guard(&nodes, &master()).expect("scaffold-only ledger passes");
    }

    // ── THE MANDATORY NON-VACUOUS TEST (§E) ─────────────────────────────────────
    // A tailored node with a DANGLING sourceEvidenceId AND a summary variant matching
    // no master-CV achievement is injected; the guard MUST block export and NAME it.
    // Non-vacuous: this test would FAIL if the guard were a no-op.
    #[test]
    fn dangling_source_evidence_id_is_blocked_and_named() {
        let mut view = tailor(&master(), &job(), 3);
        // inject a fabricated bullet whose evidence id exists nowhere in the master
        let injected = crate::types::Achievement {
            id: "FABRICATED_b999".into(),
            description: "Single-handedly increased revenue 1000% (unsupported)".into(),
            emphasise: None,
            tags: vec![],
            metrics: vec![],
            evidence_strength: None,
        };
        view.cv.experience[0].achievements_tasks.push(injected);
        // also inject a summary variant matching no achievement, with a dangling prov
        view.summary_provenance = Some("summary:99".into());

        let nodes = cv_ledger(&view);
        let err = guard(&nodes, &master()).expect_err("guard MUST block the dangling node");
        let msg = err.to_string();
        assert!(
            msg.contains("FABRICATED_b999"),
            "guard must NAME the dangling bullet: {msg}"
        );
        assert!(
            msg.contains("summary:99"),
            "guard must NAME the dangling summary: {msg}"
        );
    }

    #[test]
    fn non_vacuous_proof_clean_then_dirty() {
        // Prove the guard distinguishes: same ledger passes clean, fails when a
        // single dangling id is added (so the dangling test above is not vacuous).
        let view = tailor(&master(), &job(), 3);
        let mut nodes = cv_ledger(&view);
        guard(&nodes, &master()).expect("clean passes");
        nodes.push(LedgerNode::claim("cv.bullet[ghost]", "ghost_id_not_in_cv"));
        assert!(
            guard(&nodes, &master()).is_err(),
            "adding one dangling id must flip pass->block"
        );
    }

    #[test]
    fn missing_source_evidence_id_blocked() {
        let nodes = vec![LedgerNode {
            node: "cv.bullet[x]".into(),
            source_evidence_id: None,
            scaffold: false,
        }];
        let err = guard(&nodes, &master()).unwrap_err();
        assert!(err.to_string().contains("missing sourceEvidenceId"));
    }

    #[test]
    fn summary_provenance_resolves() {
        // summary:0 exists (persona-001 has one variant) → resolvable
        let nodes = vec![LedgerNode::claim("cv.summary", "summary:0")];
        guard(&nodes, &master()).expect("summary:0 resolves");
    }

    #[test]
    fn resolvable_ids_includes_achievements_and_summaries() {
        let ids = resolvable_ids(&master());
        assert!(ids.contains("exp_1_0_b0"));
        assert!(ids.contains("summary:0"));
    }
}
