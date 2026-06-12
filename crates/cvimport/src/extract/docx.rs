//! DOCX text extraction via `zip` + `quick-xml` (R-CVI-2). Opens `word/document.xml`
//! and walks `w:p` paragraphs, concatenating each paragraph's `w:t` runs into one
//! line. DOCX preserves paragraph structure, so it is the higher-fidelity input
//! (spike). Text is decoded with `BytesText::decode()` (NOT `unescape()` — quick-xml
//! 0.40 API).

use super::ExtractedText;
use crate::error::ImportError;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};

/// Hard cap on the decompressed size of `word/document.xml` (R-CVI-8, abuse axis). The
/// zip stream is attacker-controlled, so an unbounded `read_to_string` is a
/// decompression-bomb (OOM/hang) vector. A real résumé's document part is well under
/// this; anything larger is rejected as a [`ImportError::Decode`] without allocating the
/// full payload.
const MAX_DOCUMENT_XML_BYTES: u64 = 32 * 1024 * 1024;

/// Extract per-paragraph text from DOCX bytes. A bad/truncated zip or a missing
/// `word/document.xml` maps to [`ImportError::Decode`]; malformed XML maps to
/// [`ImportError::Decode`] too — never panics (R-CVI-8).
pub(crate) fn extract_docx(bytes: &[u8]) -> Result<ExtractedText, ImportError> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| ImportError::Decode(e.to_string()))?;

    let entry = archive
        .by_name("word/document.xml")
        .map_err(|e| ImportError::Decode(format!("word/document.xml: {e}")))?;

    // Bounded read: `take(MAX + 1)` so a decompression bomb can never inflate past the cap
    // in memory. If we actually read MAX + 1 bytes there were *more* to come → reject,
    // WITHOUT allocating the full payload.
    let mut buf = Vec::new();
    // P-COV-cvimport-2: reading a successfully-opened in-memory zip entry into a buffer
    // fails only on a transient OS/decompression I/O fault that cannot be triggered
    // deterministically offline (cf. slice-1 P-COV-2). The missing-part arm above and the
    // oversized arm below ARE covered.
    entry
        .take(MAX_DOCUMENT_XML_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| ImportError::Decode(e.to_string()))?;
    if buf.len() as u64 > MAX_DOCUMENT_XML_BYTES {
        return Err(ImportError::Decode(
            "document.xml exceeds size limit".to_string(),
        ));
    }
    let xml = String::from_utf8(buf).map_err(|e| ImportError::Decode(e.to_string()))?;

    let mut reader = Reader::from_str(&xml);
    // Reject mismatched/ill-formed markup (a corrupt document.xml) as a Decode error
    // rather than silently producing junk text.
    reader.config_mut().check_end_names = true;
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_text = false;

    loop {
        match reader
            .read_event()
            .map_err(|e| ImportError::Decode(e.to_string()))?
        {
            Event::Start(e) if e.name().as_ref() == b"w:t" => in_text = true,
            Event::End(e) if e.name().as_ref() == b"w:t" => in_text = false,
            Event::End(e) if e.name().as_ref() == b"w:p" => {
                lines.push(std::mem::take(&mut current).trim_end().to_string());
            }
            Event::Text(t) if in_text => {
                // P-COV-cvimport-1: `BytesText::decode` errors only on invalid encoding;
                // `Reader::from_str` already guarantees valid UTF-8, so this arm is
                // infallible-by-construction (cf. slice-1 P-COV-1). Kept for a correct
                // total error surface. The malformed-XML read_event error IS covered.
                let decoded = t.decode().map_err(|e| ImportError::Decode(e.to_string()))?;
                current.push_str(&decoded);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(ExtractedText { lines })
}
