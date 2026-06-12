//! Text extraction (R-CVI-1, R-CVI-2). Turns résumé bytes into an [`ExtractedText`]
//! — an ordered list of text lines — without interpreting structure. PDF yields a
//! flat stream split on newlines (spike: pdf-extract joins layout lines, so the
//! segmenter must NOT assume newlines = structure); DOCX yields one entry per
//! `w:p` paragraph (higher fidelity).

pub(crate) mod docx;
pub(crate) mod pdf;

use crate::error::ImportError;
use crate::ResumeKind;

/// Extracted résumé text as an ordered list of lines/paragraphs. `pub(crate)`:
/// only `import_resume` is the public contract. Tested via the module (L1/L2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExtractedText {
    /// One entry per source line (PDF) or paragraph (DOCX), in document order,
    /// already trimmed of trailing whitespace. Empty entries are preserved so the
    /// segmenter can use blank lines as soft boundaries.
    pub lines: Vec<String>,
}

impl ExtractedText {
    /// Build from a single flat string by splitting on `\n` (PDF path).
    pub(crate) fn from_flat(s: &str) -> Self {
        Self {
            lines: s.lines().map(|l| l.trim_end().to_string()).collect(),
        }
    }
}

/// Dispatch on kind. The `kind` string → [`ResumeKind`] parse happens in `lib.rs`;
/// here the enum is already resolved.
pub(crate) fn extract(bytes: &[u8], kind: ResumeKind) -> Result<ExtractedText, ImportError> {
    match kind {
        ResumeKind::Pdf => pdf::extract_pdf(bytes),
        ResumeKind::Docx => docx::extract_docx(bytes),
    }
}
