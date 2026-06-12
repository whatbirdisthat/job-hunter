//! aa-cvimport — deterministic PDF/DOCX résumé import → master-CV schema (item #2).
//!
//! Parses an existing résumé into a **NEW** [`aa_core::MasterCv`] document for the
//! user to review (I1: never mutates a loaded master CV). Deterministic; **no LLM,
//! no network** (the LLM layer is item #3). Honours invariants I1/I4/I5
//! (SUBJECT_MATTER_UNDERSTANDING.md §12.4).
//!
//! Pipeline: `extract` (pdf-extract | zip+quick-xml) → `segment` (cue tokens) →
//! `map` (→ MasterCv, synthetic ids). The output is guaranteed to deserialize as a
//! `MasterCv` (parse-don't-validate); the L3 boundary test additionally asserts it
//! validates against `doc/schemas/master-cv.schema.json` via `tools/fake-data/
//! validate.js`.
//!
//! Public surface (the whole crate's contract — L2):
//!   - [`import_resume`] — the entry point
//!   - [`ResumeKind`] — `Pdf` | `Docx` (parsed from `"pdf"`/`"docx"` at the boundary)
//!   - [`ImportError`] — typed errors, never panics on bad input (R-CVI-8, I5)
//!
//! Crate graph (one-way, §12.3): depends on `aa-core` ONLY — never `jobparse`,
//! `aa-desktop`, or the render path.

mod error;
mod extract;
mod map;
mod segment;

pub use error::ImportError;

/// The résumé file format. Parsed from the `"pdf"`/`"docx"` string at the Tauri
/// boundary via [`ResumeKind::parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeKind {
    Pdf,
    Docx,
}

impl ResumeKind {
    /// Parse a kind string (case-insensitive). Unknown → [`ImportError::UnsupportedKind`]
    /// (R-CVI-8).
    pub fn parse(kind: &str) -> Result<Self, ImportError> {
        match kind.trim().to_lowercase().as_str() {
            "pdf" => Ok(ResumeKind::Pdf),
            "docx" => Ok(ResumeKind::Docx),
            other => Err(ImportError::UnsupportedKind(other.to_string())),
        }
    }
}

/// Import a résumé's bytes into a NEW [`aa_core::MasterCv`] for review.
///
/// Deterministic; no LLM; no network (I5). Produces a *new* document (I1) — the
/// caller (the Tauri command) returns its JSON for the user to confirm before
/// installing via the existing `import_master_cv` validation path.
///
/// Errors (R-CVI-8): [`ImportError::Extract`]/[`ImportError::Decode`] on undecodable
/// bytes; [`ImportError::Empty`] when no résumé structure is recognised.
pub fn import_resume(bytes: &[u8], kind: ResumeKind) -> Result<aa_core::MasterCv, ImportError> {
    let extracted = extract::extract(bytes, kind)?;
    // A single Empty gate: the segmenter yields an empty result for blank/structureless
    // text (it takes the first non-empty line as the name), so a separate blank pre-check
    // would be an unreachable double-guard. Empty extraction → empty segments → Empty.
    let segments = segment::segment(&extracted);
    if segments.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(map::to_master_cv(segments))
}
