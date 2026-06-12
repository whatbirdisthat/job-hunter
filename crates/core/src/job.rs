//! NormalizedJob — core's INPUT type on the jobparse->core seam (R-D1).
//!
//! Core owns this type and validates its input against
//! `doc/schemas/normalized-job.schema.json` (the data-not-code contract). The shape
//! is symmetric with jobparse's emit type; the crates never share Rust code.

use crate::types::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedJob {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responsibilities: Vec<String>,
    pub requirements: Requirements,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requirements {
    #[serde(rename = "mustHave", default)]
    pub must_have: Vec<String>,
    #[serde(rename = "niceToHave", default)]
    pub nice_to_have: Vec<String>,
}

impl NormalizedJob {
    pub fn from_json(s: &str) -> Result<Self, CoreError> {
        serde_json::from_str(s).map_err(|e| CoreError::NormalizedJobParse(e.to_string()))
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        serde_json::to_string(self).map_err(|e| CoreError::NormalizedJobParse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normalized_job() {
        let j =
            r#"{"title":"X","company":"Y","requirements":{"mustHave":["a"],"niceToHave":["b"]}}"#;
        let nj = NormalizedJob::from_json(j).unwrap();
        assert_eq!(nj.requirements.must_have, vec!["a"]);
        assert_eq!(nj.requirements.nice_to_have, vec!["b"]);
    }

    #[test]
    fn rejects_invalid() {
        assert!(matches!(
            NormalizedJob::from_json("nope").unwrap_err(),
            CoreError::NormalizedJobParse(_)
        ));
    }

    #[test]
    fn round_trips_losslessly() {
        let j = r#"{"title":"T","company":"C","location":"L","responsibilities":["r"],"requirements":{"mustHave":["m"],"niceToHave":["n"]},"keywords":["k"]}"#;
        let nj = NormalizedJob::from_json(j).unwrap();
        let nj2 = NormalizedJob::from_json(&nj.to_json().unwrap()).unwrap();
        assert_eq!(nj, nj2);
    }

    #[test]
    fn empty_buckets_parse() {
        let j = r#"{"title":"","company":"","requirements":{"mustHave":[],"niceToHave":[]}}"#;
        let nj = NormalizedJob::from_json(j).unwrap();
        assert!(nj.requirements.must_have.is_empty());
    }
}
