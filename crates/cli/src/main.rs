//! `applicant-advocate` — the Applicant Advocate command-line tool.
//!
//! Tailors a CV and drafts a cover letter (two PDFs) from a master-CV JSON + a
//! plain-text job description — fully offline and deterministic. Every rendered
//! claim is checked against the master CV by the evidence-ledger guard: nothing is
//! invented, and export is BLOCKED if any claim lacks backing evidence.
//!
//! Self-contained in a release bundle: it locates the shipped `typst` binary,
//! fonts, and templates next to itself (no Rust/Node/typst needed on the machine).

use std::path::PathBuf;
use std::process::ExitCode;

use aa_core::render::CliRenderer;
use aa_core::{
    build_cover_letter, coverage_report, cv_ledger, guard, tailor, CvTemplate, LedgerNode,
    MasterCv, Renderer, DEFAULT_TOP_N,
};

const USAGE: &str = "\
applicant-advocate — tailor a CV + draft a cover letter (PDF), fully offline.

USAGE:
    applicant-advocate --cv <master-cv.json> --jd <job.txt> [--out <dir>] [--template <name>]

OPTIONS:
    --cv <PATH>         Master CV JSON (the canonical schema). Required.
    --jd <PATH>         Job description as a plain-text file. Required.
    --out <DIR>         Output directory for the PDFs (default: current directory).
    --template <NAME>   CV template: 'classic' (default) or 'compact' (ATS-friendly).
    -h, --help          Show this help.

OUTPUT:
    <out>/cv.pdf              the tailored CV
    <out>/cover-letter.pdf    the draft cover letter

Every rendered claim is checked against your master CV — nothing is invented.
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

fn run() -> Result<(), String> {
    let mut cv: Option<String> = None;
    let mut jd: Option<String> = None;
    let mut out = PathBuf::from(".");
    let mut template = CvTemplate::Classic;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "--cv" => cv = Some(args.next().ok_or("--cv needs a path")?),
            "--jd" => jd = Some(args.next().ok_or("--jd needs a path")?),
            "--out" => out = PathBuf::from(args.next().ok_or("--out needs a directory")?),
            "--template" => {
                let t = args.next().ok_or("--template needs a name")?;
                template = CvTemplate::parse(&t).map_err(|e| e.to_string())?;
            }
            other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
        }
    }
    let cv_path = cv.ok_or("missing --cv <master-cv.json> (try --help)")?;
    let jd_path = jd.ok_or("missing --jd <job.txt> (try --help)")?;

    let renderer = configure_renderer();

    let cv_json = std::fs::read_to_string(&cv_path).map_err(|e| format!("read {cv_path}: {e}"))?;
    let master = MasterCv::from_json(&cv_json).map_err(|e| format!("parse master CV: {e}"))?;
    let jd_text = std::fs::read_to_string(&jd_path).map_err(|e| format!("read {jd_path}: {e}"))?;
    // The jobparse → core seam: jobparse emits its own type; serialize to the
    // Normalized-Job JSON contract and re-read as core's type (same as the app).
    let parsed = aa_jobparse::parse(&jd_text);
    let job_json = aa_jobparse::to_json(&parsed).map_err(|e| format!("normalize job: {e}"))?;
    let job = aa_core::NormalizedJob::from_json(&job_json)
        .map_err(|e| format!("parse normalized job: {e}"))?;

    // Deterministic fit + tailored view, evidence-guarded before any render.
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

    let cv_pdf = renderer
        .render_cv_with_template(&view, template)
        .map_err(|e| format!("render CV (is `typst` available?): {e}"))?;
    let cover_pdf = renderer
        .render_cover_letter(&letter)
        .map_err(|e| format!("render cover letter: {e}"))?;

    std::fs::create_dir_all(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let cv_out = out.join("cv.pdf");
    let cover_out = out.join("cover-letter.pdf");
    std::fs::write(&cv_out, &cv_pdf).map_err(|e| format!("write {}: {e}", cv_out.display()))?;
    std::fs::write(&cover_out, &cover_pdf)
        .map_err(|e| format!("write {}: {e}", cover_out.display()))?;

    // Summary to stdout.
    let must_covered = coverage.must_have.iter().filter(|r| r.covered).count();
    let nice_covered = coverage.nice_to_have.iter().filter(|r| r.covered).count();
    println!("Applicant Advocate — tailored application ready.");
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
    let missing: Vec<&str> = coverage
        .must_have
        .iter()
        .filter(|r| !r.covered)
        .map(|r| r.requirement.as_str())
        .collect();
    if !missing.is_empty() {
        println!("  gaps (must-have):  {}", missing.join(", "));
    }
    println!("  wrote:            {}", cv_out.display());
    println!("                    {}", cover_out.display());
    Ok(())
}

/// Point the renderer at templates/fonts/typst. In a release bundle these sit next
/// to the binary; in a dev checkout, fall back to the repo-rooted default renderer.
fn configure_renderer() -> CliRenderer {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("templates/cv/classic.typ").exists() {
                let typst = dir.join("typst");
                if typst.exists() {
                    std::env::set_var("AA_TYPST_BIN", &typst);
                }
                let fonts = dir.join("fonts");
                if fonts.exists() {
                    std::env::set_var("AA_FONT_PATH", &fonts);
                }
                return CliRenderer::new(dir);
            }
        }
    }
    CliRenderer::default()
}
