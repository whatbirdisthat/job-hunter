//! C4 — Coverage report (§B) and C5 — Fit score (§C).
//!
//! §B: `must_have_coverage = matched_must / total_must`,
//!     `nice_have_coverage = matched_nice / total_nice`.
//!     The report enumerates each requirement with covered + matching evidence ids.
//!     Empty bucket → coverage = 1.0 (documented sentinel, tested).
//! §C: `fit_score = 0.6 * must_have_coverage + 0.4 * nice_have_coverage`.

use crate::job::NormalizedJob;
use crate::matching::Candidate;
use crate::types::MasterCv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequirementCoverage {
    pub requirement: String,
    pub covered: bool,
    #[serde(rename = "evidenceIds")]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    #[serde(rename = "mustHave")]
    pub must_have: Vec<RequirementCoverage>,
    #[serde(rename = "niceToHave")]
    pub nice_to_have: Vec<RequirementCoverage>,
    #[serde(rename = "mustHaveCoverage")]
    pub must_have_coverage: f64,
    #[serde(rename = "niceHaveCoverage")]
    pub nice_have_coverage: f64,
    #[serde(rename = "fitScore")]
    pub fit_score: f64,
}

/// Empty-bucket sentinel: a bucket with no requirements is fully covered (§B edge).
const EMPTY_BUCKET_COVERAGE: f64 = 1.0;

fn cover_bucket(
    cv: &MasterCv,
    cand: &Candidate,
    reqs: &[String],
) -> (Vec<RequirementCoverage>, f64) {
    let rows: Vec<RequirementCoverage> = reqs
        .iter()
        .map(|r| RequirementCoverage {
            requirement: r.clone(),
            covered: cand.matches(r),
            evidence_ids: cand.matching_evidence_ids(cv, r),
        })
        .collect();
    let coverage = if rows.is_empty() {
        EMPTY_BUCKET_COVERAGE
    } else {
        rows.iter().filter(|r| r.covered).count() as f64 / rows.len() as f64
    };
    (rows, coverage)
}

/// Build the coverage report (§B) and fit score (§C) for a master CV against a job.
pub fn coverage_report(cv: &MasterCv, job: &NormalizedJob) -> CoverageReport {
    let cand = Candidate::from_master(cv);
    let (must_have, must_cov) = cover_bucket(cv, &cand, &job.requirements.must_have);
    let (nice_to_have, nice_cov) = cover_bucket(cv, &cand, &job.requirements.nice_to_have);
    CoverageReport {
        must_have,
        nice_to_have,
        must_have_coverage: must_cov,
        nice_have_coverage: nice_cov,
        fit_score: 0.6 * must_cov + 0.4 * nice_cov,
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

    fn job(must: &[&str], nice: &[&str]) -> NormalizedJob {
        NormalizedJob {
            title: "T".into(),
            company: "C".into(),
            location: String::new(),
            responsibilities: vec![],
            requirements: crate::job::Requirements {
                must_have: must.iter().map(|s| s.to_string()).collect(),
                nice_to_have: nice.iter().map(|s| s.to_string()).collect(),
            },
            keywords: vec![],
        }
    }

    #[test]
    fn all_covered_gives_full_coverage() {
        let r = coverage_report(&cv(), &job(&["Python", "PostgreSQL"], &[]));
        assert_eq!(r.must_have_coverage, 1.0);
        assert!(r.must_have.iter().all(|c| c.covered));
    }

    #[test]
    fn none_covered_gives_zero() {
        let r = coverage_report(&cv(), &job(&["Cobol", "Fortran"], &[]));
        assert_eq!(r.must_have_coverage, 0.0);
    }

    #[test]
    fn mixed_coverage() {
        let r = coverage_report(&cv(), &job(&["Python", "Cobol"], &[]));
        assert_eq!(r.must_have_coverage, 0.5);
    }

    #[test]
    fn empty_must_bucket_sentinel_is_one() {
        let r = coverage_report(&cv(), &job(&[], &["Python"]));
        assert_eq!(r.must_have_coverage, 1.0);
    }

    #[test]
    fn empty_nice_bucket_sentinel_is_one() {
        let r = coverage_report(&cv(), &job(&["Python"], &[]));
        assert_eq!(r.nice_have_coverage, 1.0);
    }

    #[test]
    fn evidence_ids_listed_for_covered() {
        let r = coverage_report(&cv(), &job(&["caching"], &[]));
        assert!(!r.must_have[0].evidence_ids.is_empty());
    }

    #[test]
    fn evidence_ids_empty_for_uncovered() {
        let r = coverage_report(&cv(), &job(&["Cobol"], &[]));
        assert!(r.must_have[0].evidence_ids.is_empty());
    }

    #[test]
    fn fit_score_zero() {
        let r = coverage_report(&cv(), &job(&["Cobol"], &["Fortran"]));
        assert_eq!(r.fit_score, 0.0);
    }

    #[test]
    fn fit_score_one() {
        let r = coverage_report(&cv(), &job(&["Python"], &["Go"]));
        assert_eq!(r.fit_score, 1.0);
    }

    #[test]
    fn fit_score_blend_exact() {
        // must = 1.0 (Python covered), nice = 0.0 (Cobol uncovered)
        let r = coverage_report(&cv(), &job(&["Python"], &["Cobol"]));
        assert!((r.fit_score - 0.6).abs() < 1e-9);
    }

    #[test]
    fn both_empty_buckets_fit_is_one() {
        let r = coverage_report(&cv(), &job(&[], &[]));
        assert_eq!(r.fit_score, 1.0);
    }
}
