//! C1 — Master-CV serde types (parse-don't-validate).
//!
//! These mirror `doc/schemas/master-cv.schema.json`. The tailored view (§H) is a
//! filtered/reordered `MasterCv` — the SAME type — so it conforms to the master-CV
//! schema and `classic.typ` renders it unchanged. Deserialization is the validation
//! boundary; once a `MasterCv` exists, its required fields are guaranteed present.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Typed errors for the core engine. Surfaced across the Tauri command boundary
/// without panicking (parse-don't-validate, I5).
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("failed to parse master CV: {0}")]
    MasterCvParse(String),
    #[error("failed to parse normalized job: {0}")]
    NormalizedJobParse(String),
    #[error("evidence-ledger guard blocked export: {0}")]
    LedgerBlocked(String),
    #[error("typst render failed: {0}")]
    Render(String),
}

/// The canonical, immutable master CV (I1). A tailored view is the same shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterCv {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub person: Person,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    #[serde(
        rename = "summaryVariants",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub summary_variants: Vec<String>,
    #[serde(
        rename = "programmingLanguages",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub programming_languages: Vec<Skill>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<Skill>,
    #[serde(
        rename = "toolsTechnologies",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub tools_technologies: Vec<Skill>,
    #[serde(rename = "asAServices", default, skip_serializing_if = "Vec::is_empty")]
    pub as_a_services: Vec<Skill>,
    #[serde(default)]
    pub experience: Vec<Experience>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub education: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certifications: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub awards: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "professionalTitle",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub professional_title: Option<String>,
    #[serde(
        rename = "professionalDescription",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub professional_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linkedin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub proficiency: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(rename = "evidenceIds", default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    #[serde(rename = "jobTitle")]
    pub job_title: String,
    #[serde(rename = "businessName")]
    pub business_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consultancy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(
        rename = "employmentType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub employment_type: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate", default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(
        rename = "achievementsTasks",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub achievements_tasks: Vec<Achievement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emphasise: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<String>,
    #[serde(
        rename = "evidenceStrength",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence_strength: Option<f64>,
}

impl MasterCv {
    /// Parse-don't-validate entry point. A successful parse guarantees the schema's
    /// required fields are present (serde enforces them).
    pub fn from_json(s: &str) -> Result<Self, CoreError> {
        serde_json::from_str(s).map_err(|e| CoreError::MasterCvParse(e.to_string()))
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        serde_json::to_string(self).map_err(|e| CoreError::MasterCvParse(e.to_string()))
    }

    /// All achievements across all (non-hidden-agnostic) experiences, with owning
    /// experience index, for ranking and ledger resolution.
    pub fn all_achievements(&self) -> Vec<(&Experience, &Achievement)> {
        self.experience
            .iter()
            .flat_map(|e| e.achievements_tasks.iter().map(move |a| (e, a)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: &str = r#"{"schemaVersion":"1.0.0","person":{},"experience":[]}"#;

    #[test]
    fn parses_minimal_document() {
        let cv = MasterCv::from_json(MIN).unwrap();
        assert_eq!(cv.schema_version, "1.0.0");
        assert!(cv.experience.is_empty());
    }

    #[test]
    fn rejects_invalid_json() {
        let err = MasterCv::from_json("{not json").unwrap_err();
        assert!(matches!(err, CoreError::MasterCvParse(_)));
        // exercise Display
        assert!(err.to_string().contains("failed to parse master CV"));
    }

    #[test]
    fn rejects_missing_required_field() {
        let err = MasterCv::from_json(r#"{"person":{},"experience":[]}"#).unwrap_err();
        assert!(matches!(err, CoreError::MasterCvParse(_)));
    }

    #[test]
    fn round_trips_to_json_and_back() {
        let cv = MasterCv::from_json(MIN).unwrap();
        let s = cv.to_json().unwrap();
        let cv2 = MasterCv::from_json(&s).unwrap();
        assert_eq!(cv, cv2);
    }

    #[test]
    fn all_achievements_flattens() {
        let doc = r#"{"schemaVersion":"1.0.0","person":{},"experience":[
            {"id":"e0","jobTitle":"T","businessName":"B","startDate":"Jan 2020",
             "achievementsTasks":[{"id":"e0_b0","description":"d0"},{"id":"e0_b1","description":"d1"}]},
            {"id":"e1","jobTitle":"T","businessName":"B","startDate":"Jan 2021",
             "achievementsTasks":[{"id":"e1_b0","description":"d2"}]}]}"#;
        let cv = MasterCv::from_json(doc).unwrap();
        let all = cv.all_achievements();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].1.id, "e0_b0");
        assert_eq!(all[2].0.id, "e1");
    }

    #[test]
    fn parses_full_fixture_persona_001() {
        let s = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/personas/persona-001.cv.json"
        ))
        .unwrap();
        let cv = MasterCv::from_json(&s).unwrap();
        assert_eq!(cv.person.name.as_deref(), Some("Devin Voss"));
        assert_eq!(cv.experience.len(), 5);
        // serialize back and re-parse: schema-conformant round trip (no data loss on required parts)
        let again = MasterCv::from_json(&cv.to_json().unwrap()).unwrap();
        assert_eq!(again.experience.len(), 5);
    }

    #[test]
    fn error_display_variants() {
        assert!(CoreError::NormalizedJobParse("x".into())
            .to_string()
            .contains("normalized job"));
        assert!(CoreError::LedgerBlocked("n".into())
            .to_string()
            .contains("blocked export"));
        assert!(CoreError::Render("r".into())
            .to_string()
            .contains("render failed"));
    }
}
