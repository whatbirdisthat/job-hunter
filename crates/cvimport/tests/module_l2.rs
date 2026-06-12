//! L2 — module (public surface) tests for `import_resume` over synthetic inputs:
//! the DOCX path, the PDF path, and EVERY `ImportError` arm (R-CVI-8). Non-vacuous:
//! a garbage-bytes case must return `Err`, never a default `MasterCv`.

mod support;

use aa_cvimport::{import_resume, ImportError, ResumeKind};
use std::time::Instant;

#[test]
fn docx_path_recovers_a_master_cv() {
    // R-CVI-2: the DOCX public path returns Ok(MasterCv) with recovered fields.
    let bytes = support::synth_persona_docx("persona-001.cv.json");
    let t0 = Instant::now();
    let cv = import_resume(&bytes, ResumeKind::Docx).expect("docx imports");
    eprintln!("[L2 perf] import_resume(docx): {:?}", t0.elapsed());
    assert_eq!(cv.person.name.as_deref(), Some("Devin Voss"));
    assert!(!cv.experience.is_empty());
}

#[test]
fn pdf_path_recovers_a_master_cv() {
    // R-CVI-1: the PDF public path returns Ok(MasterCv) (containment-level, R3b).
    let bytes = support::render_persona_pdf("persona-001.cv.json");
    let t0 = Instant::now();
    let cv = import_resume(&bytes, ResumeKind::Pdf).expect("pdf imports");
    eprintln!("[L2 perf] import_resume(pdf): {:?}", t0.elapsed());
    assert_eq!(cv.person.name.as_deref(), Some("Devin Voss"));
}

#[test]
fn unsupported_kind_is_a_typed_error() {
    // R-CVI-8 — UnsupportedKind via ResumeKind::parse.
    let err = ResumeKind::parse("xlsx").unwrap_err();
    assert!(matches!(err, ImportError::UnsupportedKind(ref k) if k == "xlsx"));
    assert!(err.to_string().contains("unsupported résumé kind"));
}

#[test]
fn resume_kind_parse_accepts_known_kinds_case_insensitively() {
    assert_eq!(ResumeKind::parse("PDF").unwrap(), ResumeKind::Pdf);
    assert_eq!(ResumeKind::parse(" docx ").unwrap(), ResumeKind::Docx);
}

#[test]
fn garbage_pdf_bytes_return_extract_error_not_panic() {
    // R-CVI-8 — non-vacuous: clearly-not-a-PDF bytes → Err(Extract), never a default CV.
    let result = import_resume(b"this is definitely not a pdf file", ResumeKind::Pdf);
    assert!(
        matches!(result, Err(ImportError::Extract(_))),
        "got {result:?}"
    );
}

#[test]
fn truncated_docx_zip_returns_decode_error() {
    // R-CVI-8 — a non-zip / truncated container → Err(Decode).
    let err = import_resume(b"PK\x03\x04 truncated garbage", ResumeKind::Docx).unwrap_err();
    assert!(matches!(err, ImportError::Decode(_)), "got {err:?}");
    assert!(err.to_string().contains("could not decode"));
}

#[test]
fn structureless_docx_returns_empty_error() {
    // R-CVI-8 — a well-formed DOCX whose text carries no résumé structure → Err(Empty).
    // (A single blank paragraph: not a header, not skills, not experience.)
    use docx_rs::*;
    use std::io::Cursor;
    let mut buf = Cursor::new(Vec::new());
    Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("   ")))
        .build()
        .pack(&mut buf)
        .unwrap();
    let err = import_resume(&buf.into_inner(), ResumeKind::Docx).unwrap_err();
    assert!(matches!(err, ImportError::Empty), "got {err:?}");
    assert!(err.to_string().contains("no recognisable content"));
}

#[test]
fn valid_zip_missing_document_part_returns_decode_error() {
    // R-CVI-8 — a well-formed zip that is NOT a DOCX (no word/document.xml) → Decode.
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        w.start_file("notes.txt", SimpleFileOptions::default())
            .unwrap();
        w.write_all(b"not a docx").unwrap();
        w.finish().unwrap();
    }
    let err = import_resume(&buf.into_inner(), ResumeKind::Docx).unwrap_err();
    assert!(matches!(err, ImportError::Decode(_)), "got {err:?}");
    assert!(err.to_string().contains("word/document.xml"));
}

#[test]
fn malformed_document_xml_returns_decode_error() {
    // R-CVI-8 — a valid zip whose word/document.xml is ill-formed XML → Decode. A
    // mismatched end tag is a hard quick-xml parse error (IllFormed), surfaced as Decode.
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        w.start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        // a mismatched close tag → quick-xml yields Err(IllFormed) on read_event
        w.write_all(b"<w:document></w:body></w:document>").unwrap();
        w.finish().unwrap();
    }
    let err = import_resume(&buf.into_inner(), ResumeKind::Docx).unwrap_err();
    assert!(matches!(err, ImportError::Decode(_)), "got {err:?}");
}

#[test]
fn invalid_utf8_document_xml_returns_decode_error() {
    // Finding 2 fix introduces a bounded raw-byte read + explicit UTF-8 conversion (the
    // old `read_to_string` did this implicitly). A `word/document.xml` carrying invalid
    // UTF-8 must surface as Err(Decode), not a panic — pin the new conversion arm.
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        w.start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        // 0xFF is never valid UTF-8
        w.write_all(b"<w:document>\xFF</w:document>").unwrap();
        w.finish().unwrap();
    }
    let err = import_resume(&buf.into_inner(), ResumeKind::Docx).unwrap_err();
    assert!(matches!(err, ImportError::Decode(_)), "got {err:?}");
}

#[test]
fn oversized_document_xml_returns_decode_error_without_oom() {
    // Finding 2 (HIGH) — decompression bomb: a zip whose `word/document.xml` decompresses
    // to MORE than the importer's cap must be rejected as Err(Decode), NOT read fully into
    // memory. The payload here is highly compressible (a single repeated byte) so the zip on
    // disk stays tiny while the inflated stream exceeds the cap. Must return promptly.
    use std::io::{Cursor, Write};
    use std::time::{Duration, Instant};
    use zip::write::SimpleFileOptions;

    // 33 MiB of a single byte: > the 32 MiB cap, but compresses to a handful of KiB.
    const PAYLOAD_LEN: usize = 33 * 1024 * 1024;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        w.start_file(
            "word/document.xml",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        // valid XML head so it is unambiguously the "too big" path, not a parse error
        w.write_all(b"<w:document><w:body>").unwrap();
        w.write_all(&vec![b'a'; PAYLOAD_LEN]).unwrap();
        w.finish().unwrap();
    }
    let bytes = buf.into_inner();
    assert!(
        bytes.len() < 1024 * 1024,
        "compressed bomb should stay tiny on disk, was {} bytes",
        bytes.len()
    );

    let t0 = Instant::now();
    let err = import_resume(&bytes, ResumeKind::Docx).unwrap_err();
    let elapsed = t0.elapsed();
    assert!(matches!(err, ImportError::Decode(_)), "got {err:?}");
    assert!(err.to_string().contains("exceeds size limit"), "got {err}");
    assert!(
        elapsed < Duration::from_secs(10),
        "bomb rejection must be prompt, took {elapsed:?}"
    );
}

/// Build a minimal résumé DOCX in memory whose experience job-title line begins with a
/// caller-supplied prefix, so we can drive the Finding-1 UTF-8 offset path through the
/// full public `import_resume` pipeline (not just the segmenter unit).
fn docx_with_job_title_prefix(prefix: &str) -> Vec<u8> {
    use docx_rs::*;
    use std::io::Cursor;
    let para = |t: &str| Paragraph::new().add_run(Run::new().add_text(t));
    let mut buf = Cursor::new(Vec::new());
    Docx::new()
        .add_paragraph(para("Devin Voss"))
        .add_paragraph(para("Engineer"))
        .add_paragraph(para("Experience"))
        .add_paragraph(para(&format!("{prefix}Engineer Jan 2020 – Present")))
        .add_paragraph(para("Acme Co · Sydney"))
        .add_paragraph(para("Did a thing"))
        .build()
        .pack(&mut buf)
        .unwrap();
    buf.into_inner()
}

#[test]
fn import_resume_does_not_panic_on_expanding_lowercase_title_chars() {
    // Finding 1 (CRITICAL): a job-title line starting with `ẞ` (U+1E9E → "ss") or `İ`
    // (U+0130 → "i̇") whose lowercase has a different byte length must not slice the
    // original line at a lowercased-copy offset (panic / corruption). import_resume → Ok.
    for prefix in ["ẞé ", "İ "] {
        let bytes = docx_with_job_title_prefix(prefix);
        let cv = import_resume(&bytes, ResumeKind::Docx).unwrap_or_else(|e| {
            panic!("import_resume must be Ok for prefix {prefix:?}, got {e:?}")
        });
        assert_eq!(cv.person.name.as_deref(), Some("Devin Voss"));
        assert_eq!(
            cv.experience.len(),
            1,
            "experience recovered for {prefix:?}"
        );
    }
}

#[test]
fn import_resume_ascii_title_still_parses_correctly_regression() {
    // Regression alongside Finding 1: an ASCII title must still recover title + date.
    let bytes = docx_with_job_title_prefix("Backend ");
    let cv = import_resume(&bytes, ResumeKind::Docx).expect("ascii imports");
    let e = &cv.experience[0];
    assert_eq!(e.job_title, "Backend Engineer");
    assert_eq!(e.start_date, "Jan 2020");
    assert_eq!(e.end_date, None);
}

#[test]
fn import_is_deterministic_same_bytes_same_output() {
    // I5 / R-CVI-6 — same input bytes → byte-identical serialised output.
    let bytes = support::synth_persona_docx("persona-001.cv.json");
    let a = import_resume(&bytes, ResumeKind::Docx)
        .unwrap()
        .to_json()
        .unwrap();
    let b = import_resume(&bytes, ResumeKind::Docx)
        .unwrap()
        .to_json()
        .unwrap();
    assert_eq!(a, b);
}
