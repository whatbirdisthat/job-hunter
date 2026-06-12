//! C8/C9 — Typst render (§H) + cover-letter render (§G), behind a `Renderer` seam.
//!
//! §H contract: render the tailored-view JSON (exposed at `/view.json`, with
//! `sys.inputs.data` pointing at it) through the SAME `classic.typ` that the CLI
//! uses, with bundled Liberation fonts. Two render backends implement one seam:
//!
//!   * [`CliRenderer`] (DEFAULT) — invokes the `typst` binary (`typst compile`,
//!     `--input data=<path> --root <dir>`). Deterministic and offline. This is the
//!     working path in this environment.
//!   * [`EmbeddedRenderer`] (feature `embedded-typst`) — the in-process §H design:
//!     a custom `typst-as-lib` World with bundled fonts + an in-memory VFS exposing
//!     the view at `/view.json`. Compiles once a compatible `time` crate is vendored
//!     (see DISCUSS-RENDER). NO shell-out — the literal §H contract.
//!
//! R-D2: render assertions are non-empty PDF + valid PDF structure + the ledger
//! invariant — NEVER raw-byte equality. The classic templates avoid `today()`, so
//! document CONTENT is timestamp-independent and reproducible (I5).

use crate::tailor::TailoredView;
use crate::types::CoreError;
use serde::Serialize;

// Bundled fonts ship with the crate (no system font dependency, §H). They are used
// by the embedded backend and are part of the crate's render contract regardless of
// backend (the CLI backend relies on the same Liberation faces being resolvable).
pub const BUNDLED_FONTS: &[(&str, &[u8])] = &[
    (
        "LiberationSans-Regular.ttf",
        include_bytes!("../fonts/LiberationSans-Regular.ttf"),
    ),
    (
        "LiberationSans-Bold.ttf",
        include_bytes!("../fonts/LiberationSans-Bold.ttf"),
    ),
    (
        "LiberationSans-Italic.ttf",
        include_bytes!("../fonts/LiberationSans-Italic.ttf"),
    ),
    (
        "LiberationSans-BoldItalic.ttf",
        include_bytes!("../fonts/LiberationSans-BoldItalic.ttf"),
    ),
    (
        "LiberationMono-Regular.ttf",
        include_bytes!("../fonts/LiberationMono-Regular.ttf"),
    ),
    (
        "LiberationMono-Bold.ttf",
        include_bytes!("../fonts/LiberationMono-Bold.ttf"),
    ),
];

/// Repo-root-relative template paths (§H: classic.typ stays CLI-renderable).
const CV_TEMPLATE_REL: &str = "templates/cv/classic.typ";
const LETTER_TEMPLATE_REL: &str = "templates/letter/classic-letter.typ";

/// The cover-letter model (§G), serialized for the letter template. Strength
/// paragraphs each carry the evidence id of the achievement they wrap.
#[derive(Debug, Clone, Serialize)]
pub struct CoverLetter {
    pub greeting: String,
    #[serde(rename = "whyRole")]
    pub why_role: String,
    pub strengths: Vec<StrengthParagraph>,
    pub closing: String,
    #[serde(rename = "candidateName")]
    pub candidate_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrengthParagraph {
    pub text: String,
    #[serde(rename = "sourceEvidenceId")]
    pub source_evidence_id: String,
}

/// The render seam (§H). One trait, two backends.
pub trait Renderer {
    fn render_cv(&self, view: &TailoredView) -> Result<Vec<u8>, CoreError>;
    fn render_cover_letter(&self, letter: &CoverLetter) -> Result<Vec<u8>, CoreError>;
}

/// Locate the repository root (the dir containing `templates/`) from the crate.
fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Default backend: subprocess `typst compile`. Offline, deterministic. Renders the
/// JSON data via `--input data=<path>` exactly as the CI `foundation` smoke does, so
/// the embedded and CLI paths consume the identical template + data contract (§H).
pub struct CliRenderer {
    root: std::path::PathBuf,
}

impl Default for CliRenderer {
    fn default() -> Self {
        CliRenderer { root: repo_root() }
    }
}

impl CliRenderer {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        CliRenderer { root: root.into() }
    }

    fn compile(&self, template_rel: &str, data_json: &str) -> Result<Vec<u8>, CoreError> {
        use std::io::Write;
        // Write the data JSON to a temp file UNDER root so --root resolves it.
        let mut data_path = self.root.clone();
        let unique = format!(
            "aa-render-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        data_path.push(&unique);
        {
            let mut f = std::fs::File::create(&data_path)
                .map_err(|e| CoreError::Render(format!("temp data file: {e}")))?;
            f.write_all(data_json.as_bytes())
                .map_err(|e| CoreError::Render(format!("write data: {e}")))?;
        }
        let out_path = data_path.with_extension("pdf");

        let status = std::process::Command::new("typst")
            .arg("compile")
            .arg(self.root.join(template_rel))
            .arg(&out_path)
            .arg("--input")
            .arg(format!("data=/{unique}"))
            .arg("--root")
            .arg(&self.root)
            .arg("--font-path")
            .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts"))
            .output()
            .map_err(|e| CoreError::Render(format!("spawn typst: {e}")))?;

        let _ = std::fs::remove_file(&data_path);
        if !status.status.success() {
            let _ = std::fs::remove_file(&out_path);
            return Err(CoreError::Render(format!(
                "typst compile failed: {}",
                String::from_utf8_lossy(&status.stderr)
            )));
        }
        let bytes =
            std::fs::read(&out_path).map_err(|e| CoreError::Render(format!("read pdf: {e}")))?;
        let _ = std::fs::remove_file(&out_path);
        Ok(bytes)
    }
}

impl Renderer for CliRenderer {
    fn render_cv(&self, view: &TailoredView) -> Result<Vec<u8>, CoreError> {
        let json = view.cv.to_json()?;
        self.compile(CV_TEMPLATE_REL, &json)
    }
    fn render_cover_letter(&self, letter: &CoverLetter) -> Result<Vec<u8>, CoreError> {
        let json = serde_json::to_string(letter).map_err(|e| CoreError::Render(e.to_string()))?;
        self.compile(LETTER_TEMPLATE_REL, &json)
    }
}

// ── The §H embedded backend — feature-gated until a compatible `time` is vendored ──
#[cfg(feature = "embedded-typst")]
mod embedded {
    use super::*;
    use typst::foundations::Dict;
    use typst::layout::PagedDocument;
    use typst_as_lib::TypstEngine;

    static CV_TEMPLATE: &str = include_str!("../../../templates/cv/classic.typ");
    static LETTER_TEMPLATE: &str = include_str!("../../../templates/letter/classic-letter.typ");
    const CV_MAIN: &str = "/cv.typ";
    const LETTER_MAIN: &str = "/letter.typ";
    const VIEW_PATH: &str = "/view.json";
    const LETTER_DATA_PATH: &str = "/letter.json";

    /// The §H in-process World: bundled fonts + in-memory VFS, no shell-out.
    pub struct EmbeddedRenderer;

    fn dict_data(path: &str) -> Dict {
        let mut d = Dict::new();
        d.insert("data".into(), typst::foundations::Value::Str(path.into()));
        d
    }

    fn engine(main_id: &str, src: &str, data_path: &str, data: &[u8]) -> TypstEngine {
        let fonts: Vec<&[u8]> = BUNDLED_FONTS.iter().map(|(_, b)| *b).collect();
        TypstEngine::builder()
            .fonts(fonts)
            .with_static_source_file_resolver([(main_id, src)])
            .with_static_file_resolver([(data_path, data.to_vec())])
            .build()
    }

    fn to_pdf(engine: &TypstEngine, main_id: &str, data_path: &str) -> Result<Vec<u8>, CoreError> {
        let warned =
            engine.compile_with_input::<_, _, PagedDocument>(main_id, dict_data(data_path));
        let doc = warned
            .output
            .map_err(|e| CoreError::Render(format!("{e:?}")))?;
        typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
            .map_err(|e| CoreError::Render(format!("pdf export: {e:?}")))
    }

    impl Renderer for EmbeddedRenderer {
        fn render_cv(&self, view: &TailoredView) -> Result<Vec<u8>, CoreError> {
            let json = view.cv.to_json()?;
            to_pdf(
                &engine(CV_MAIN, CV_TEMPLATE, VIEW_PATH, json.as_bytes()),
                CV_MAIN,
                VIEW_PATH,
            )
        }
        fn render_cover_letter(&self, letter: &CoverLetter) -> Result<Vec<u8>, CoreError> {
            let json =
                serde_json::to_string(letter).map_err(|e| CoreError::Render(e.to_string()))?;
            to_pdf(
                &engine(
                    LETTER_MAIN,
                    LETTER_TEMPLATE,
                    LETTER_DATA_PATH,
                    json.as_bytes(),
                ),
                LETTER_MAIN,
                LETTER_DATA_PATH,
            )
        }
    }
}

#[cfg(feature = "embedded-typst")]
pub use embedded::EmbeddedRenderer;

/// The active default renderer for the slice (swap to EmbeddedRenderer under the
/// `embedded-typst` feature once §H can compile here).
pub fn default_renderer() -> impl Renderer {
    CliRenderer::default()
}

/// Convenience free functions over the default renderer (used by the engine + tests).
pub fn render_cv(view: &TailoredView) -> Result<Vec<u8>, CoreError> {
    default_renderer().render_cv(view)
}
pub fn render_cover_letter(letter: &CoverLetter) -> Result<Vec<u8>, CoreError> {
    default_renderer().render_cover_letter(letter)
}

/// Structural PDF validation: starts with `%PDF-` and contains `%%EOF` near the end.
pub fn is_valid_pdf(bytes: &[u8]) -> bool {
    bytes.len() > 100
        && bytes.starts_with(b"%PDF-")
        && bytes.windows(5).rev().take(4096).any(|w| w == b"%%EOF")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{NormalizedJob, Requirements};
    use crate::tailor::tailor;
    use crate::types::MasterCv;

    fn master() -> MasterCv {
        let s = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/personas/persona-001.cv.json"
        ))
        .unwrap();
        MasterCv::from_json(&s).unwrap()
    }

    fn job() -> NormalizedJob {
        NormalizedJob {
            title: "Senior Backend Engineer".into(),
            company: "Acme".into(),
            location: String::new(),
            responsibilities: vec![],
            requirements: Requirements {
                must_have: vec!["caching".into(), "Python".into()],
                nice_to_have: vec![],
            },
            keywords: vec![],
        }
    }

    #[test]
    fn renders_non_empty_valid_pdf() {
        let view = tailor(&master(), &job(), 3);
        let pdf = render_cv(&view).expect("render must succeed");
        assert!(!pdf.is_empty());
        assert!(is_valid_pdf(&pdf), "must be a structurally valid PDF");
    }

    #[test]
    fn render_is_deterministic_in_length() {
        let view = tailor(&master(), &job(), 3);
        let a = render_cv(&view).unwrap();
        let b = render_cv(&view).unwrap();
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn empty_experience_renders() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"X"},"experience":[]}"#,
        )
        .unwrap();
        let view = tailor(&cv, &job(), 3);
        assert!(is_valid_pdf(&render_cv(&view).expect("empty renders")));
    }

    #[test]
    fn unicode_name_renders() {
        let cv = MasterCv::from_json(
            r#"{"schemaVersion":"1.0.0","person":{"name":"Café Ünïcøde"},"experience":[]}"#,
        )
        .unwrap();
        let view = tailor(&cv, &job(), 3);
        assert!(is_valid_pdf(&render_cv(&view).unwrap()));
    }

    #[test]
    fn cover_letter_renders_valid_pdf() {
        let letter = CoverLetter {
            greeting: "Dear Hiring Team,".into(),
            why_role: "I'm excited about the Senior Backend Engineer role at Acme.".into(),
            strengths: vec![StrengthParagraph {
                text: "Cut p99 API latency by 38% by reworking the caching and query layer".into(),
                source_evidence_id: "exp_1_0_b0".into(),
            }],
            closing: "Kind regards, Devin Voss".into(),
            candidate_name: "Devin Voss".into(),
        };
        assert!(is_valid_pdf(
            &render_cover_letter(&letter).expect("letter renders")
        ));
    }

    #[test]
    fn bundled_fonts_present() {
        assert_eq!(BUNDLED_FONTS.len(), 6);
        assert!(BUNDLED_FONTS.iter().all(|(_, b)| b.len() > 1000));
    }

    #[test]
    fn is_valid_pdf_rejects_garbage() {
        assert!(!is_valid_pdf(b"not a pdf"));
        assert!(!is_valid_pdf(&[]));
    }

    #[test]
    fn cli_renderer_reports_typst_compile_failure() {
        // Valid root + a deliberately broken template → typst spawns, compiles, and
        // exits nonzero → exercises the compile-failure branch (render.rs 127-131).
        let dir = std::env::temp_dir().join(format!("aa-broken-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("templates/cv")).unwrap();
        // syntactically broken Typst — guarantees a nonzero compile exit
        std::fs::write(
            dir.join("templates/cv/classic.typ"),
            "#let x = (((  // unbalanced — compile error",
        )
        .unwrap();
        let r = CliRenderer::new(&dir);
        let view = tailor(&master(), &job(), 3);
        let err = r.render_cv(&view).expect_err("broken template must error");
        assert!(err.to_string().contains("typst compile failed"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cli_renderer_errors_when_root_missing() {
        // Nonexistent root → temp-file create fails before spawn (covers the
        // temp-data-file create error arm, render.rs ~104).
        let r = CliRenderer::new("/nonexistent-root-xyz-aa");
        let view = tailor(&master(), &job(), 3);
        let err = r.render_cv(&view).expect_err("missing root errors");
        assert!(err.to_string().contains("temp data file") || err.to_string().contains("Render"));
    }

    #[test]
    fn cli_renderer_cover_letter_path_errors_on_bad_root() {
        // exercises CliRenderer::render_cover_letter (the letter free-fn + impl).
        let r = CliRenderer::new("/nonexistent-root-xyz-bb");
        let letter = CoverLetter {
            greeting: "g".into(),
            why_role: "w".into(),
            strengths: vec![],
            closing: "c".into(),
            candidate_name: "n".into(),
        };
        assert!(r.render_cover_letter(&letter).is_err());
    }

    #[test]
    fn repo_root_returns_existing_dir() {
        // covers repo_root success path (the canonicalize-Ok arm)
        assert!(repo_root().join("templates/cv/classic.typ").exists());
    }
}
