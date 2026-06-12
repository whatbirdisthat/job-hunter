//! PDF text extraction via `pdf-extract` (R-CVI-1). The extractor returns a flat
//! character stream; we split on newlines for the segmenter but it must NOT treat
//! the absence of a newline as the absence of a boundary (spike R3b: adjacent
//! layout lines are joined).

use super::ExtractedText;
use crate::error::ImportError;

/// Extract text from PDF bytes. Maps any extractor failure (malformed stream,
/// undecodable content) to [`ImportError::Extract`] — never panics (R-CVI-8).
pub(crate) fn extract_pdf(bytes: &[u8]) -> Result<ExtractedText, ImportError> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| ImportError::Extract(e.to_string()))?;
    Ok(ExtractedText::from_flat(&text))
}
