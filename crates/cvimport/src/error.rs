//! Typed errors for résumé import (R-CVI-8). Surfaced across the Tauri boundary
//! without panicking (parse-don't-validate, I5).

use thiserror::Error;

/// The error surface of the whole crate. Every arm is reachable from a real bad
/// input — exercised by the L2 error tests.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The `kind` string was not `"pdf"` or `"docx"` (R-CVI-8).
    #[error("unsupported résumé kind: {0}")]
    UnsupportedKind(String),
    /// Text extraction failed (e.g. the PDF stream could not be decoded). (R-CVI-1/8)
    #[error("could not extract text: {0}")]
    Extract(String),
    /// Extraction succeeded but yielded no recognisable résumé content (R-CVI-8).
    #[error("résumé produced no recognisable content")]
    Empty,
    /// The container could not be decoded (e.g. a truncated/invalid DOCX zip). (R-CVI-2/8)
    #[error("could not decode file: {0}")]
    Decode(String),
}
