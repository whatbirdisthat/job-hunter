//! L5 STORY — the résumé-import user journey, end to end, on real files generated
//! at test time (I4: no committed binary fixtures). For persona-001:
//!   (a) render to PDF via `templates/cv/classic.typ` (typst CLI);
//!   (b) synthesise a DOCX via `docx-rs`.
//! Run each through `import_resume`; assert recovered key fields + schema-valid.
//! Perf-delta gated against a recorded baseline (mirror slice-1 I6 posture).
//!
//! DOCX is the higher-fidelity assertion (exact name/title/skills/experience). PDF
//! tolerates the spike's line-join (R3b): containment/presence, not byte-equality.

mod support;

#[path = "perf_gate.rs"]
mod perf_gate;

use aa_cvimport::{import_resume, ResumeKind};
use std::io::Write;
use std::time::Instant;

/// Absolute budget: the import path is trivially under the < 60 s journey budget (I6).
const BUDGET_SECS: f64 = 60.0;
/// Perf-delta tolerance vs the TRACKED import-story baseline (`doc/perf/...`). Independent
/// of the absolute budget (Finding 3): a >3× regression fails the delta arm even though it
/// is far under the 60 s budget.
const DELTA_FACTOR: f64 = 3.0;

use std::sync::atomic::{AtomicU64, Ordering};
static SEQ: AtomicU64 = AtomicU64::new(0);

fn validates(json: &str) -> bool {
    let root = support::repo_root();
    let tmp = root.join(format!(
        "cvimport-story-{}-{}.cv.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(json.as_bytes())
        .unwrap();
    let ok = std::process::Command::new("node")
        .arg(root.join("tools/fake-data/validate.js"))
        .arg(&tmp)
        .status()
        .expect("node runs")
        .success();
    let _ = std::fs::remove_file(&tmp);
    ok
}

#[test]
fn story_persona_round_trip_pdf_and_docx_perf_delta_gated() {
    let start = Instant::now();

    // ── DOCX: the exact-recovery path ─────────────────────────────────────────
    let docx = support::synth_persona_docx("persona-001.cv.json");
    let from_docx = import_resume(&docx, ResumeKind::Docx).expect("docx import");
    assert_eq!(
        from_docx.person.name.as_deref(),
        Some("Devin Voss"),
        "DOCX recovers exact name"
    );
    assert_eq!(
        from_docx.person.professional_title.as_deref(),
        Some("Senior Backend Engineer"),
        "DOCX recovers exact professional title"
    );
    assert_eq!(
        from_docx.headline.as_deref(),
        Some("Senior Backend Engineer")
    );
    // exact skill recovery (DOCX preserves the comma list)
    let langs: Vec<&str> = from_docx
        .programming_languages
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        langs.contains(&"Python") && langs.contains(&"Rust"),
        "DOCX recovers individual languages: {langs:?}"
    );
    // experience + achievements
    assert_eq!(
        from_docx.experience.len(),
        5,
        "DOCX recovers all five experience blocks"
    );
    let first = &from_docx.experience[0];
    assert_eq!(first.job_title, "Backend Engineer");
    assert_eq!(first.business_name, "Acme Co");
    assert!(
        !first.achievements_tasks.is_empty(),
        "DOCX recovers achievements"
    );
    // synthetic ids (R-CVI-6)
    assert_eq!(first.id, "imp_exp_0");
    assert_eq!(first.achievements_tasks[0].id, "imp_exp_0_b0");
    assert!(
        validates(&from_docx.to_json().unwrap()),
        "DOCX output schema-valid"
    );

    // ── PDF: containment path (R3b line-join tolerated) ───────────────────────
    let pdf = support::render_persona_pdf("persona-001.cv.json");
    let from_pdf = import_resume(&pdf, ResumeKind::Pdf).expect("pdf import");
    assert_eq!(
        from_pdf.person.name.as_deref(),
        Some("Devin Voss"),
        "PDF recovers the name line"
    );
    // PDF: at least one experience recovered, and achievement text is PRESENT
    // somewhere (containment, not exact per-field equality).
    assert!(
        !from_pdf.experience.is_empty(),
        "PDF recovers at least one experience block"
    );
    let pdf_text = from_pdf
        .experience
        .iter()
        .flat_map(|e| e.achievements_tasks.iter())
        .map(|a| a.description.clone())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        pdf_text.contains("Cut p99 API latency"),
        "PDF recovers achievement text (containment): {pdf_text}"
    );
    assert!(
        validates(&from_pdf.to_json().unwrap()),
        "PDF output schema-valid"
    );

    let elapsed = start.elapsed().as_secs_f64();

    // ── perf-delta gate (I6, Finding 3) ───────────────────────────────────────
    // TWO independent obligations, read from a TRACKED baseline (never self-overwritten):
    //   (a) absolute I6 budget   — `elapsed < BUDGET_SECS`
    //   (b) regression delta     — `elapsed <= baseline * DELTA_FACTOR` (can actually fire)
    // The shared `perf_gate` enforces both; the gate's non-vacuity is proven in perf_gate_l1.
    let baseline_path = support::repo_root().join("doc/perf/cvimport-import-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "résumé-import STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    // SAMPLE emission (kept) — observability, NOT a self-overwriting baseline write.
    eprintln!(
        "[L5 STORY perf] résumé-import round-trip: {elapsed:.3}s (budget {BUDGET_SECS}s, \
         baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
