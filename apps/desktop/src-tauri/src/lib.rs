//! aa-desktop — the command layer that wires the jobparse → core seam (R-D1).
//!
//! This is the ONLY crate depending on both `aa-core` and `aa-jobparse`. It exposes
//! the command surface the React UI invokes AND the STORY harness (R-D3) drives,
//! fully offline. The jobparse output is validated against the Normalized-Job JSON
//! shape, then handed to core's mirror type — the crates never share Rust code.
//!
//! Commands (the UI/STORY journey):
//!   import_master_cv → parse_job → compute_coverage → tailor → set_decisions →
//!   export_application (ledger-guarded, two PDFs).

use aa_core::{
    assemble_application, build_cover_letter, coverage_report, cv_ledger, guard,
    render_cover_letter, render_cv, tailor, CoverageReport, MasterCv, NormalizedJob as CoreJob,
    TailoredView, DEFAULT_TOP_N,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum CommandError {
    #[error("import failed: {0}")]
    Import(String),
    #[error("no master CV imported")]
    NoMasterCv,
    #[error("no job parsed")]
    NoJob,
    #[error("export blocked: {0}")]
    ExportBlocked(String),
    #[error("render failed: {0}")]
    Render(String),
}

impl From<aa_core::CoreError> for CommandError {
    fn from(e: aa_core::CoreError) -> Self {
        match e {
            aa_core::CoreError::LedgerBlocked(m) => CommandError::ExportBlocked(m),
            aa_core::CoreError::Render(m) => CommandError::Render(m),
            other => CommandError::Import(other.to_string()),
        }
    }
}

/// A résumé import error surfaces to the UI as an import failure (R-CVI-8/10): the
/// typed `ImportError` is carried verbatim in the message, never a panic.
impl From<aa_cvimport::ImportError> for CommandError {
    fn from(e: aa_cvimport::ImportError) -> Self {
        CommandError::Import(e.to_string())
    }
}

/// The jobparse → core seam (R-D1): jobparse emits its type, we serialize to the
/// Normalized-Job JSON shape, then core deserializes its mirror type. If the JSON
/// shapes diverge this fails loudly (the schema is the contract).
fn seam(parsed: &aa_jobparse::NormalizedJob) -> Result<CoreJob, CommandError> {
    let json = aa_jobparse::to_json(parsed).map_err(|e| CommandError::Import(e.to_string()))?;
    CoreJob::from_json(&json).map_err(|e| CommandError::Import(e.to_string()))
}

/// In-memory application session — the state a Tauri command handler holds (the
/// SQLCipher-backed store persists the imported master CV; for the command logic and
/// the STORY harness this in-memory session is the unit of behaviour).
#[derive(Default)]
pub struct Session {
    master: Option<MasterCv>,
    job: Option<CoreJob>,
    /// per-achievement approve/reject decisions (true = approved/kept).
    decisions: BTreeMap<String, bool>,
}

/// The export artefacts surfaced to the UI / STORY (two PDFs + coverage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    #[serde(rename = "cvPdfLen")]
    pub cv_pdf_len: usize,
    #[serde(rename = "coverLetterPdfLen")]
    pub cover_letter_pdf_len: usize,
    pub coverage: CoverageReport,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Command: import + validate a master-CV JSON (I1: stored immutable).
    pub fn import_master_cv(&mut self, json: &str) -> Result<(), CommandError> {
        let cv = MasterCv::from_json(json).map_err(|e| CommandError::Import(e.to_string()))?;
        self.master = Some(cv);
        Ok(())
    }

    /// Command (R-CVI-10): import a PDF/DOCX résumé's bytes into a NEW master-CV
    /// document and return its JSON for the user to review. This NEVER mutates the
    /// installed master CV (I1, R-CVI-9) — `self` is `&self`; the review JSON is
    /// only installed when the user explicitly calls `import_master_cv` with it
    /// (reusing slice-1 validation, no duplicate validation here). `kind` is the
    /// `"pdf"`/`"docx"` string from the boundary; an unknown kind → typed error.
    pub fn import_resume(&self, bytes: &[u8], kind: &str) -> Result<String, CommandError> {
        let kind = aa_cvimport::ResumeKind::parse(kind)?;
        let cv = aa_cvimport::import_resume(bytes, kind)?;
        cv.to_json().map_err(CommandError::from)
    }

    /// Command: parse a pasted JD (§F) and stage it via the validated seam (R-D1).
    pub fn parse_job(&mut self, raw_jd: &str) -> Result<(), CommandError> {
        let parsed = aa_jobparse::parse(raw_jd);
        self.job = Some(seam(&parsed)?);
        Ok(())
    }

    /// Command: coverage report (§B/§C) for the imported CV against the parsed job.
    pub fn compute_coverage(&self) -> Result<CoverageReport, CommandError> {
        let cv = self.master.as_ref().ok_or(CommandError::NoMasterCv)?;
        let job = self.job.as_ref().ok_or(CommandError::NoJob)?;
        Ok(coverage_report(cv, job))
    }

    /// Command: tailored view (§D/§H) — selection/ordering over the master CV.
    pub fn tailored_view(&self) -> Result<TailoredView, CommandError> {
        let cv = self.master.as_ref().ok_or(CommandError::NoMasterCv)?;
        let job = self.job.as_ref().ok_or(CommandError::NoJob)?;
        Ok(tailor(cv, job, DEFAULT_TOP_N))
    }

    /// Command: record an approve(true)/reject(false) decision for a bullet id.
    pub fn set_decision(&mut self, evidence_id: &str, approved: bool) {
        self.decisions.insert(evidence_id.to_string(), approved);
    }

    /// Apply rejections to a view: drop any achievement explicitly rejected. A
    /// rejected bullet is REMOVED (never fabricated back) — honesty over polish.
    fn apply_decisions(&self, mut view: TailoredView) -> TailoredView {
        for e in view.cv.experience.iter_mut() {
            e.achievements_tasks
                .retain(|a| *self.decisions.get(&a.id).unwrap_or(&true));
        }
        view.selected_ids
            .retain(|id| *self.decisions.get(id).unwrap_or(&true));
        view
    }

    /// Command: export — ledger-guarded (§E/I2) render of two PDFs. Honours
    /// approve/reject decisions. Returns lengths + coverage (the UI shows these; the
    /// STORY harness asserts non-empty + the perf budget).
    pub fn export_application(&self) -> Result<(Vec<u8>, Vec<u8>, ExportResult), CommandError> {
        let cv = self.master.as_ref().ok_or(CommandError::NoMasterCv)?;
        let job = self.job.as_ref().ok_or(CommandError::NoJob)?;

        let view = self.apply_decisions(self.tailored_view()?);

        // §E guard on the CV ledger before render/export.
        guard(&cv_ledger(&view), cv)?;

        let letter = build_cover_letter(&view, job, cv);
        let cv_pdf = render_cv(&view).map_err(CommandError::from)?;
        let cover_letter_pdf = render_cover_letter(&letter).map_err(CommandError::from)?;

        let coverage = coverage_report(cv, job);
        let result = ExportResult {
            cv_pdf_len: cv_pdf.len(),
            cover_letter_pdf_len: cover_letter_pdf.len(),
            coverage,
        };
        Ok((cv_pdf, cover_letter_pdf, result))
    }

    /// Convenience: full unguarded-by-decision assemble (used by the L4 system test).
    pub fn assemble(&self) -> Result<(), CommandError> {
        let cv = self.master.as_ref().ok_or(CommandError::NoMasterCv)?;
        let job = self.job.as_ref().ok_or(CommandError::NoJob)?;
        assemble_application(cv, job)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn persona() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/personas/persona-001.cv.json"
        ))
        .unwrap()
    }

    const JD: &str = "We are hiring a Senior Backend Engineer at Acme Group. You will own delivery end to end. Required: Strong TypeScript or Python; Stakeholder management; AWS or GCP experience. Nice to have: GraphQL; Fintech domain knowledge.";

    #[test]
    fn import_requires_valid_cv() {
        let mut s = Session::new();
        assert!(s.import_master_cv("garbage").is_err());
        assert!(s.import_master_cv(&persona()).is_ok());
    }

    #[test]
    fn coverage_requires_cv_and_job() {
        let s = Session::new();
        assert!(matches!(
            s.compute_coverage(),
            Err(CommandError::NoMasterCv)
        ));
    }

    #[test]
    fn parse_job_requires_cv_for_coverage() {
        let mut s = Session::new();
        s.import_master_cv(&persona()).unwrap();
        assert!(matches!(s.compute_coverage(), Err(CommandError::NoJob)));
    }

    #[test]
    fn full_command_journey() {
        let mut s = Session::new();
        s.import_master_cv(&persona()).unwrap();
        s.parse_job(JD).unwrap();
        let cov = s.compute_coverage().unwrap();
        assert!(!cov.must_have.is_empty());
        let view = s.tailored_view().unwrap();
        assert!(!view.cv.experience.is_empty());
        let (cv_pdf, letter_pdf, result) = s.export_application().unwrap();
        assert!(aa_core::render::is_valid_pdf(&cv_pdf));
        assert!(aa_core::render::is_valid_pdf(&letter_pdf));
        assert_eq!(result.cv_pdf_len, cv_pdf.len());
    }

    #[test]
    fn reject_removes_bullet_from_export() {
        let mut s = Session::new();
        s.import_master_cv(&persona()).unwrap();
        s.parse_job(JD).unwrap();
        let view = s.tailored_view().unwrap();
        let first_id = view.cv.experience[0].achievements_tasks[0].id.clone();
        s.set_decision(&first_id, false);
        let pruned = s.apply_decisions(s.tailored_view().unwrap());
        let still_present = pruned
            .cv
            .experience
            .iter()
            .flat_map(|e| e.achievements_tasks.iter())
            .any(|a| a.id == first_id);
        assert!(!still_present, "rejected bullet must be removed");
    }

    #[test]
    fn approve_keeps_bullet() {
        let mut s = Session::new();
        s.import_master_cv(&persona()).unwrap();
        s.parse_job(JD).unwrap();
        let view = s.tailored_view().unwrap();
        let first_id = view.cv.experience[0].achievements_tasks[0].id.clone();
        s.set_decision(&first_id, true);
        let kept = s.apply_decisions(s.tailored_view().unwrap());
        assert!(kept
            .cv
            .experience
            .iter()
            .flat_map(|e| e.achievements_tasks.iter())
            .any(|a| a.id == first_id));
    }

    #[test]
    fn seam_roundtrips_jobparse_to_core() {
        let parsed = aa_jobparse::parse(JD);
        let core_job = seam(&parsed).unwrap();
        assert_eq!(core_job.requirements.must_have.len(), 3);
        assert_eq!(core_job.requirements.nice_to_have.len(), 2);
    }

    #[test]
    fn export_requires_state() {
        let s = Session::new();
        assert!(s.export_application().is_err());
    }

    #[test]
    fn assemble_system_path() {
        let mut s = Session::new();
        s.import_master_cv(&persona()).unwrap();
        s.parse_job(JD).unwrap();
        s.assemble().unwrap();
    }

    #[test]
    fn command_error_display() {
        assert!(CommandError::NoMasterCv
            .to_string()
            .contains("no master CV"));
        assert!(CommandError::NoJob.to_string().contains("no job"));
        assert!(CommandError::ExportBlocked("x".into())
            .to_string()
            .contains("export blocked"));
        assert!(CommandError::Render("x".into())
            .to_string()
            .contains("render failed"));
        let conv: CommandError = aa_core::CoreError::LedgerBlocked("z".into()).into();
        assert!(matches!(conv, CommandError::ExportBlocked(_)));
        let conv2: CommandError = aa_core::CoreError::Render("z".into()).into();
        assert!(matches!(conv2, CommandError::Render(_)));
        let conv3: CommandError = aa_core::CoreError::MasterCvParse("z".into()).into();
        assert!(matches!(conv3, CommandError::Import(_)));
    }
}
