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

use aa_advocate::{
    redact, redact_kind, AdvocateConfig, AdvocateProvider, RewriteKind, StubProvider,
};
use aa_core::{
    assemble_application, build_cover_letter, coverage_report, cv_ledger, guard,
    render_cover_letter, render_cv, requirement_for, tailor, CoverageReport, MasterCv,
    NormalizedJob as CoreJob, TailoredView, DEFAULT_TOP_N,
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
    /// The advocate flag was ON but the provider could not run (R-ADV-9). Surfaced
    /// explicitly — NEVER a silent fallback to the deterministic text.
    #[error("advocate failed: {0}")]
    Advocate(String),
}

/// R-ADV-9: an advocate provider failure surfaces as an explicit command error, never a
/// silent fallback to the deterministic path.
impl From<aa_advocate::AdvocateError> for CommandError {
    fn from(e: aa_advocate::AdvocateError) -> Self {
        CommandError::Advocate(e.to_string())
    }
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
///
/// Item #3: the session holds the advocate config (default DISABLED) + a provider. The
/// provider is a boxed trait object so the live adapters (feature-gated) and the
/// deterministic stub (CI) are interchangeable behind one seam. The default provider is
/// the honest [`StubProvider`]; with the flag OFF it is never invoked.
pub struct Session {
    master: Option<MasterCv>,
    job: Option<CoreJob>,
    /// per-achievement approve/reject decisions (true = approved/kept).
    decisions: BTreeMap<String, bool>,
    /// Advocate feature flag (default `enabled == false`).
    advocate: AdvocateConfig,
    /// The advocate provider. Default = honest deterministic stub (no network).
    provider: Box<dyn AdvocateProvider + Send + Sync>,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            master: None,
            job: None,
            decisions: BTreeMap::new(),
            advocate: AdvocateConfig::default(),
            provider: Box::new(StubProvider::new()),
        }
    }
}

/// The export artefacts surfaced to the UI / STORY (two PDFs + coverage + provenance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    #[serde(rename = "cvPdfLen")]
    pub cv_pdf_len: usize,
    #[serde(rename = "coverLetterPdfLen")]
    pub cover_letter_pdf_len: usize,
    pub coverage: CoverageReport,
    /// R-ADV-10: whether the advocate LLM rewrite ran for this export. Surface-only
    /// (no SQLCipher persistence this slice) — drives the UI "AI was used" badge.
    #[serde(rename = "aiUsed")]
    pub ai_used: bool,
    /// The provider name when `ai_used` (e.g. "stub" / "ollama"); `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// R-ADV-10: the evidence ids whose text the advocate rewrote, for a per-bullet
    /// "rewritten" badge in the review UI. Empty when the flag is off.
    #[serde(
        rename = "rewrittenIds",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub rewritten_ids: Vec<String>,
}

/// The deterministic, pre-render artefact of an export (R-ADV-11 anchor): the guarded
/// tailored view + cover letter + provenance, BEFORE the non-deterministic typst step.
struct PreparedExport {
    view: TailoredView,
    letter: aa_core::CoverLetter,
    rewritten_ids: Vec<String>,
    ai_used: bool,
    provider_name: &'static str,
    coverage: CoverageReport,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a session with an explicit advocate provider (used by the L4/L5 tests
    /// to inject the honest / fabricating / unreachable stub). The flag still defaults
    /// to OFF — call [`Session::set_advocate_enabled`] to opt in.
    pub fn with_provider(provider: Box<dyn AdvocateProvider + Send + Sync>) -> Self {
        Session {
            provider,
            ..Session::default()
        }
    }

    /// Command (R-ADV-13): toggle the advocate feature on/off. Default is OFF.
    pub fn set_advocate_enabled(&mut self, enabled: bool) {
        self.advocate.enabled = enabled;
    }

    /// Whether the advocate flag is currently on (for the UI / tests).
    pub fn advocate_enabled(&self) -> bool {
        self.advocate.enabled
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

    /// The DETERMINISTIC part of an export: build the decisions-applied tailored view +
    /// cover letter, run the Applicant Advocate rewrite (when enabled), and run the §E
    /// ledger guard — but stop BEFORE the (non-deterministic, timestamped) typst render.
    /// Returns the guarded render inputs + provenance. Shared by `export_application`
    /// (which renders) and `render_inputs` (which the flag-off equivalence test compares
    /// — PDF bytes are not byte-stable across typst invocations per R-D2, so the
    /// determinism anchor is this pre-render artefact, not the PDF).
    fn prepare_export(&self) -> Result<PreparedExport, CommandError> {
        let cv = self.master.as_ref().ok_or(CommandError::NoMasterCv)?;
        let job = self.job.as_ref().ok_or(CommandError::NoJob)?;

        let mut view = self.apply_decisions(self.tailored_view()?);

        // Build the cover letter from the ORIGINAL (pre-rewrite) view text. The letter's
        // strength paragraphs are sourced from the same achievements as the CV bullets, so it
        // MUST be assembled BEFORE the in-place CV bullet rewrite below — otherwise the
        // strength loop would rewrite already-rewritten text (a double-prefix with the stub;
        // a rewrite-of-rewrite drift with a live model). Each strength is therefore rewritten
        // EXACTLY ONCE, from its original evidence text.
        let mut letter = build_cover_letter(&view, job, cv);

        // ── Item #3: the Applicant Advocate rewrite (R-ADV-7) ───────────────────────
        // Runs BETWEEN apply_decisions and the EXISTING §E guard. When the flag is OFF
        // this whole block is skipped → the render inputs are identical to the
        // deterministic path (R-ADV-11). When ON, each bullet's text is rewritten by the
        // provider; the bullet keeps its honest id ONLY when the provider cites it back,
        // otherwise the bullet ADOPTS the (possibly fabricated) cited id so the guard
        // below NAMES and BLOCKS it (R-ADV-8) — never a silent swap.
        let mut rewritten_ids: Vec<String> = Vec::new();
        if self.advocate.enabled {
            for e in view.cv.experience.iter_mut() {
                for a in e.achievements_tasks.iter_mut() {
                    let requirement = requirement_for(cv, job, &a.id);
                    let req = redact(a, &requirement);
                    let resp = self.provider.rewrite(&req)?; // R-ADV-9: error surfaces
                    if resp.cited_evidence_id == a.id {
                        rewritten_ids.push(a.id.clone());
                        a.description = resp.rewritten_text;
                    } else {
                        // adopt the cited (possibly fabricated) id → the guard will block it
                        a.id = resp.cited_evidence_id;
                    }
                }
            }
        }

        // §E guard on the CV ledger before render/export. After the advocate rewrite a
        // dangling/absent cited id is checked against the IMMUTABLE master `cv` (not the
        // view) — so a fabricated id is named and blocked here (R-ADV-8).
        guard(&cv_ledger(&view), cv)?;

        // Cover-letter strength paragraphs get the SAME advocate re-entry + re-guard. The
        // `letter` was built above from the ORIGINAL view text, so this rewrites each strength
        // EXACTLY ONCE (not a rewrite-of-an-already-rewritten bullet).
        if self.advocate.enabled {
            for s in letter.strengths.iter_mut() {
                // wrap the strength as an achievement-shaped evidence atom for redaction
                let atom = aa_core::Achievement {
                    id: s.source_evidence_id.clone(),
                    description: s.text.clone(),
                    emphasise: None,
                    tags: vec![],
                    metrics: vec![],
                    evidence_strength: None,
                };
                let requirement = requirement_for(cv, job, &s.source_evidence_id);
                let req = redact_kind(&atom, &requirement, RewriteKind::CoverLetterStrength);
                let resp = self.provider.rewrite(&req)?;
                if resp.cited_evidence_id == s.source_evidence_id {
                    s.text = resp.rewritten_text;
                } else {
                    s.source_evidence_id = resp.cited_evidence_id;
                }
            }
            // re-guard the (possibly rewritten) letter strengths against the master CV.
            let mut letter_nodes: Vec<aa_core::LedgerNode> = Vec::new();
            for (i, s) in letter.strengths.iter().enumerate() {
                letter_nodes.push(aa_core::LedgerNode::claim(
                    format!("letter.strength[{i}]"),
                    s.source_evidence_id.clone(),
                ));
            }
            guard(&letter_nodes, cv)?;
        }

        Ok(PreparedExport {
            view,
            letter,
            rewritten_ids,
            ai_used: self.advocate.enabled,
            provider_name: self.provider.name(),
            coverage: coverage_report(cv, job),
        })
    }

    /// The DETERMINISTIC render inputs the renderer would consume (R-ADV-11 anchor): the
    /// guarded tailored-view CV JSON + the cover-letter JSON, plus `ai_used`. PDF bytes
    /// are NOT byte-stable across typst invocations (R-D2), so the flag-off equivalence
    /// test compares THESE inputs, not the output PDFs.
    pub fn render_inputs(&self) -> Result<(String, String, bool), CommandError> {
        let p = self.prepare_export()?;
        let cv_json = p.view.cv.to_json().map_err(CommandError::from)?;
        let letter_json =
            serde_json::to_string(&p.letter).map_err(|e| CommandError::Render(e.to_string()))?;
        Ok((cv_json, letter_json, p.ai_used))
    }

    /// Command: export — ledger-guarded (§E/I2) render of two PDFs. Honours
    /// approve/reject decisions + the Applicant Advocate rewrite (when enabled). Returns
    /// lengths + coverage + provenance (the UI shows these; the STORY harness asserts
    /// non-empty + the perf budget).
    pub fn export_application(&self) -> Result<(Vec<u8>, Vec<u8>, ExportResult), CommandError> {
        let p = self.prepare_export()?;

        let cv_pdf = render_cv(&p.view).map_err(CommandError::from)?;
        let cover_letter_pdf = render_cover_letter(&p.letter).map_err(CommandError::from)?;

        let result = ExportResult {
            cv_pdf_len: cv_pdf.len(),
            cover_letter_pdf_len: cover_letter_pdf.len(),
            coverage: p.coverage,
            ai_used: p.ai_used,
            provider: p.ai_used.then(|| p.provider_name.to_string()),
            rewritten_ids: p.rewritten_ids,
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
