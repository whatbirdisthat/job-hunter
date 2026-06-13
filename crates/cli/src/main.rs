//! `applicant-advocate` — the Applicant Advocate command-line tool.
//!
//! Tailors a CV and drafts a cover letter (two PDFs) from a master-CV JSON + a
//! plain-text job description — fully offline and deterministic. Every rendered
//! claim is checked against the master CV by the evidence-ledger guard: nothing is
//! invented, and export is BLOCKED if any claim lacks backing evidence.
//!
//! Item 8b — ADAPTIVE INGESTION + SAMPLE honesty guard:
//!   * Strict-then-mine: try strict `MasterCv::from_json`; on parse failure, mine the
//!     arbitrary JSON via `aa_cvimport::import_cv_json`.
//!   * Missing-field flow: on any empty IMPORTANT class, the default is to LEAVE BLANK.
//!     `[s]` per field (interactive) or `--use-fakes` (all) inserts obviously-synthetic
//!     SAMPLE values. `--non-interactive` (or no TTY) errors on gaps instead of prompting.
//!   * SAFE BY CONSTRUCTION: if any sample was used, normal export is BLOCKED unless
//!     `--allow-samples` (`--use-fakes` implies it); sample output is written to
//!     `cv.SAMPLE.pdf` / `cover-letter.SAMPLE.pdf` and carries a visible watermark.
//!     The whole block-vs-render decision is the single pure `aa_core::decide` call.
//!
//! Self-contained in a release bundle: it locates the shipped `typst` binary,
//! fonts, and templates next to itself (no Rust/Node/typst needed on the machine).

use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use aa_core::render::CliRenderer;
use aa_core::{
    build_cover_letter, cover_letter_filename_ext, coverage_report, cv_filename_ext, cv_ledger,
    decide, fill_with_samples, guard, tailor, CoverLetter, CvTemplate, ExportDecision, LedgerNode,
    MasterCv, MissingFields, Renderer, TailoredView, BLOCKED_MESSAGE, DEFAULT_TOP_N,
};
use aa_cvimport::{completeness, ignored_role_arrays, import_cv_json};

const USAGE: &str = "\
applicant-advocate — tailor a CV + draft a cover letter (PDF), fully offline.

USAGE:
    applicant-advocate --cv <master-cv.json> --jd <job.txt> [--out <dir>] [OPTIONS]

OPTIONS:
    --cv <PATH>         Master CV JSON. Strict schema is tried first; on parse
                        failure the adaptive miner recovers what it can. Required.
    --jd <PATH>         Job description as a plain-text file. Required.
    --out <DIR>         Output directory for the PDFs (default: current directory).
    --template <NAME>   CV template: 'classic' (default) or 'compact' (ATS-friendly).
    --format <FMT>      Output format: 'pdf' (default), 'docx', or 'both'.
    --use-fakes         Fill EVERY missing IMPORTANT field with an obvious SAMPLE
                        value (non-interactive). Implies --allow-samples; writes
                        *.SAMPLE.pdf with a visible watermark. The 'see it working' path.
    --allow-samples     Permit exporting a document that contains SAMPLE values
                        (otherwise export is BLOCKED). Output is still watermarked
                        and written to *.SAMPLE.pdf.
    --non-interactive   Never prompt; error on any missing IMPORTANT field. (Also the
                        behaviour when stdin is not a TTY.)
    -h, --help          Show this help.

OUTPUT (normal):   <out>/cv.pdf              <out>/cover-letter.pdf
OUTPUT (samples):  <out>/cv.SAMPLE.pdf       <out>/cover-letter.SAMPLE.pdf  (watermarked)
With --format docx the extension is .docx; --format both writes BOTH per document.

Every rendered claim is checked against your master CV — nothing is invented. A SAMPLE
document is blocked from normal export, renamed, and watermarked so it cannot reach an
employer unedited.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Output document format (item #10). `Pdf` keeps the pre-#10 behaviour byte-for-byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Pdf,
    Docx,
    Both,
}

impl OutputFormat {
    fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "pdf" => Ok(OutputFormat::Pdf),
            "docx" => Ok(OutputFormat::Docx),
            "both" => Ok(OutputFormat::Both),
            other => Err(format!(
                "unknown --format: {other} (expected pdf|docx|both)"
            )),
        }
    }

    fn wants_pdf(self) -> bool {
        matches!(self, OutputFormat::Pdf | OutputFormat::Both)
    }

    fn wants_docx(self) -> bool {
        matches!(self, OutputFormat::Docx | OutputFormat::Both)
    }
}

struct Args {
    cv: Option<String>,
    jd: Option<String>,
    out: PathBuf,
    template: CvTemplate,
    format: OutputFormat,
    use_fakes: bool,
    allow_samples: bool,
    non_interactive: bool,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut a = Args {
        cv: None,
        jd: None,
        out: PathBuf::from("."),
        template: CvTemplate::Classic,
        format: OutputFormat::Pdf,
        use_fakes: false,
        allow_samples: false,
        non_interactive: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--cv" => a.cv = Some(args.next().ok_or("--cv needs a path")?),
            "--jd" => a.jd = Some(args.next().ok_or("--jd needs a path")?),
            "--out" => a.out = PathBuf::from(args.next().ok_or("--out needs a directory")?),
            "--template" => {
                let t = args.next().ok_or("--template needs a name")?;
                a.template = CvTemplate::parse(&t).map_err(|e| e.to_string())?;
            }
            "--format" => {
                let f = args
                    .next()
                    .ok_or("--format needs a value (pdf|docx|both)")?;
                a.format = OutputFormat::parse(&f)?;
            }
            "--use-fakes" => a.use_fakes = true,
            "--allow-samples" => a.allow_samples = true,
            "--non-interactive" => a.non_interactive = true,
            other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
        }
    }
    Ok(Some(a))
}

fn run() -> Result<(), String> {
    let Some(args) = parse_args()? else {
        return Ok(());
    };
    let cv_path = args
        .cv
        .as_ref()
        .ok_or("missing --cv <master-cv.json> (try --help)")?;
    let jd_path = args
        .jd
        .as_ref()
        .ok_or("missing --jd <job.txt> (try --help)")?;

    let renderer = configure_renderer();

    // ── ingest: strict, then mine on parse failure (R-INGEST-CLI-1) ──────────────
    let cv_json = std::fs::read_to_string(cv_path).map_err(|e| format!("read {cv_path}: {e}"))?;
    let (mut master, ignored) = ingest(&cv_json)?;

    // ── completeness → missing-field flow (R-INGEST-CLI-2) ───────────────────────
    let report = completeness(&master, &ignored);
    if !report.ignored_role_arrays.is_empty() {
        eprintln!(
            "note: ignored other role array(s) (not merged): {}",
            report.ignored_role_arrays.join(", ")
        );
    }
    let missing = MissingFields::new(
        report.missing_person_name,
        report.missing_experience,
        report.missing_achievement,
        report.missing_skill,
    );

    let used_samples = if missing.any() {
        resolve_gaps(&mut master, missing, &args)?
    } else {
        false
    };

    // ── THE GUARD: one decision, made once (R-INGEST-CLI-3) ──────────────────────
    // --use-fakes implies --allow-samples (the explicit "see it working" path).
    let allow_samples = args.allow_samples || args.use_fakes;
    let decision = decide(used_samples, allow_samples);
    if matches!(decision, ExportDecision::Blocked) {
        return Err(BLOCKED_MESSAGE.to_string());
    }
    let watermark = decision.is_sample();

    // ── tailor + evidence guard + render (everything below is unreachable if Blocked) ─
    let jd_text = std::fs::read_to_string(jd_path).map_err(|e| format!("read {jd_path}: {e}"))?;
    let parsed = aa_jobparse::parse(&jd_text);
    let job_json = aa_jobparse::to_json(&parsed).map_err(|e| format!("normalize job: {e}"))?;
    let job = aa_core::NormalizedJob::from_json(&job_json)
        .map_err(|e| format!("parse normalized job: {e}"))?;

    let coverage = coverage_report(&master, &job);
    let view = tailor(&master, &job, DEFAULT_TOP_N);
    guard(&cv_ledger(&view), &master)
        .map_err(|e| format!("CV evidence guard blocked export (a claim lacks backing): {e}"))?;

    let letter = build_cover_letter(&view, &job, &master);
    let mut nodes = vec![
        LedgerNode::scaffold("letter.greeting"),
        LedgerNode::scaffold("letter.whyRole"),
    ];
    for (i, s) in letter.strengths.iter().enumerate() {
        nodes.push(LedgerNode::claim(
            format!("letter.strength[{i}]"),
            s.source_evidence_id.clone(),
        ));
    }
    guard(&nodes, &master)
        .map_err(|e| format!("cover-letter evidence guard blocked export: {e}"))?;

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("create {}: {e}", args.out.display()))?;

    // ── render + write, branching ONLY on the output format (item #10) ───────────
    // The PDF path is byte-for-byte the pre-#10 behaviour; the docx path authors the
    // SAME tailored view + letter via aa-docx. `--format both` writes both per document.
    let mut written: Vec<PathBuf> = Vec::new();

    if args.format.wants_pdf() {
        let cv_pdf = renderer
            .render_cv_watermarked(&view, args.template, watermark)
            .map_err(|e| format!("render CV (is `typst` available?): {e}"))?;
        let cover_pdf = renderer
            .render_cover_letter_watermarked(&letter, watermark)
            .map_err(|e| format!("render cover letter: {e}"))?;
        written.push(write_doc(
            &args.out,
            &cv_filename_ext(watermark, "pdf"),
            &cv_pdf,
        )?);
        written.push(write_doc(
            &args.out,
            &cover_letter_filename_ext(watermark, "pdf"),
            &cover_pdf,
        )?);
    }

    if args.format.wants_docx() {
        let (cv_bytes, cover_bytes) = render_docx(&view, &letter, args.template, watermark)?;
        written.push(write_doc(
            &args.out,
            &cv_filename_ext(watermark, "docx"),
            &cv_bytes,
        )?);
        written.push(write_doc(
            &args.out,
            &cover_letter_filename_ext(watermark, "docx"),
            &cover_bytes,
        )?);
    }

    print_summary(&coverage, watermark, &written);
    Ok(())
}

/// Author the CV + cover letter as DOCX bytes via `aa-docx` (item #10). Pure: consumes
/// the SAME tailored view + letter the PDF path uses; maps the typed `CoreError` to a
/// CLI string.
fn render_docx(
    view: &TailoredView,
    letter: &CoverLetter,
    template: CvTemplate,
    watermark: bool,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let cv =
        aa_docx::cv_docx(view, template, watermark).map_err(|e| format!("author CV docx: {e}"))?;
    let cover = aa_docx::cover_letter_docx(letter, watermark)
        .map_err(|e| format!("author cover-letter docx: {e}"))?;
    Ok((cv, cover))
}

/// Write `bytes` to `<out>/<name>`, returning the path written (for the summary).
fn write_doc(out: &std::path::Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = out.join(name);
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Strict-then-mine ingestion. Returns the master CV plus the ignored role arrays the
/// miner surfaces (empty for a strict-success parse). On a strict parse failure the
/// adaptive JSON miner is tried; if it also recovers nothing, the miner's typed error
/// surfaces (never a panic).
fn ingest(cv_json: &str) -> Result<(MasterCv, Vec<String>), String> {
    match MasterCv::from_json(cv_json) {
        Ok(m) => Ok((m, Vec::new())),
        Err(strict_err) => {
            let value: serde_json::Value =
                serde_json::from_str(cv_json.trim_start_matches('\u{feff}')).map_err(|e| {
                    format!(
                    "CV is neither valid master-CV JSON ({strict_err}) nor parseable JSON ({e})"
                )
                })?;
            eprintln!(
                "note: strict parse failed ({strict_err}); recovering via the adaptive miner."
            );
            let master = import_cv_json(&value)
                .map_err(|e| format!("adaptive import could not recover a CV: {e}"))?;
            Ok((master, ignored_role_arrays(&value)))
        }
    }
}

/// Resolve the missing IMPORTANT fields. Returns whether any SAMPLE value was inserted.
///
/// * `--use-fakes` → fill ALL gaps with samples (non-interactive).
/// * `--non-interactive` or no TTY → ERROR (a sample must be a deliberate choice).
/// * otherwise → prompt per gap; default LEAVE BLANK, `[s]` inserts a sample.
fn resolve_gaps(
    master: &mut MasterCv,
    missing: MissingFields,
    args: &Args,
) -> Result<bool, String> {
    if args.use_fakes {
        return Ok(fill_with_samples(master, missing));
    }
    if args.non_interactive || !std::io::stdin().is_terminal() {
        return Err(format!(
            "missing IMPORTANT field(s): {}. Re-run with --use-fakes to fill them with \
             clearly-labelled SAMPLE values, or fix the CV. (Refusing to prompt: not a TTY / \
             --non-interactive.)",
            describe_missing(missing)
        ));
    }
    // Interactive: prompt per gap. Default = leave blank; `s` = insert a sample.
    let mut chosen = MissingFields::new(false, false, false, false);
    if missing.person_name {
        chosen.person_name = prompt_sample("name")?;
    }
    if missing.experience {
        chosen.experience = prompt_sample("work experience")?;
    }
    if missing.achievement {
        chosen.achievement = prompt_sample("an achievement")?;
    }
    if missing.skill {
        chosen.skill = prompt_sample("skills")?;
    }
    Ok(fill_with_samples(master, chosen))
}

/// Ask whether to insert a SAMPLE value for one gap. Default (empty / anything but `s`)
/// = leave blank. Returns true iff the user explicitly chose `s`.
fn prompt_sample(label: &str) -> Result<bool, String> {
    print!("Missing {label}. [Enter]=leave blank, [s]=insert a SAMPLE value: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(line.trim().eq_ignore_ascii_case("s"))
}

fn describe_missing(m: MissingFields) -> String {
    let mut v = Vec::new();
    if m.person_name {
        v.push("name");
    }
    if m.experience {
        v.push("experience");
    }
    if m.achievement {
        v.push("achievement");
    }
    if m.skill {
        v.push("skills");
    }
    v.join(", ")
}

fn print_summary(coverage: &aa_core::CoverageReport, watermark: bool, written: &[PathBuf]) {
    let must_covered = coverage.must_have.iter().filter(|r| r.covered).count();
    let nice_covered = coverage.nice_to_have.iter().filter(|r| r.covered).count();
    println!("Applicant Advocate — tailored application ready.");
    if watermark {
        println!("  ⚠ contains SAMPLE data — output is watermarked and named *.SAMPLE.pdf.");
        println!("    REPLACE the sample values before sending to any employer.");
    }
    println!(
        "  fit score:        {:.0}%",
        (coverage.fit_score * 100.0).round()
    );
    println!(
        "  must-have cover:  {}/{}  ({:.0}%)",
        must_covered,
        coverage.must_have.len(),
        (coverage.must_have_coverage * 100.0).round()
    );
    println!(
        "  nice-to-have:     {}/{}  ({:.0}%)",
        nice_covered,
        coverage.nice_to_have.len(),
        (coverage.nice_have_coverage * 100.0).round()
    );
    let gaps: Vec<&str> = coverage
        .must_have
        .iter()
        .filter(|r| !r.covered)
        .map(|r| r.requirement.as_str())
        .collect();
    if !gaps.is_empty() {
        println!("  gaps (must-have):  {}", gaps.join(", "));
    }
    let mut iter = written.iter();
    if let Some(first) = iter.next() {
        println!("  wrote:            {}", first.display());
        for p in iter {
            println!("                    {}", p.display());
        }
    }
}

/// Point the renderer at templates/fonts/typst. In a release bundle these sit next
/// to the binary; in a dev checkout, fall back to the repo-rooted default renderer.
fn configure_renderer() -> CliRenderer {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("templates/cv/classic.typ").exists() {
                let mut r = CliRenderer::new(dir);
                let typst = dir.join("typst");
                if typst.exists() {
                    r = r.with_typst_bin(typst);
                }
                let fonts = dir.join("fonts");
                if fonts.exists() {
                    r = r.with_font_path(fonts);
                }
                return r;
            }
        }
    }
    CliRenderer::default()
}

// Note: this binary (crates/cli/) is EXCLUDED from the coverage floor (P-COV-4). The
// load-bearing SAMPLE-guard decision logic + watermark threading live in aa_core
// (`samples.rs`, `render.rs`) where they ARE pinned to 100%-of-reachable, and the
// end-to-end journey is asserted by the CLI STORY test (tests/ingestion_story_l5.rs).
