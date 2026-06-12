//! L5 STORY — the tracker user journey via the headless command-level harness (item #5).
//!
//! Journey: build a synthetic set of applications + contacts via the tracker commands →
//! advance their lifecycles → record notes → `daily_call_sheet(today)` → assert the sheet is
//! well-formed and deterministically ordered. Driven through the SAME command surface the UI
//! invokes, fully offline against a temp-dir `JsonFileStore`. Perf-delta gated on the < 60 s
//! offline budget (I6) AND a >3x regression against a TRACKED baseline (Finding 3).

// Shared perf-gate logic (Finding 3) — single source, reused across the L5 stories.
#[path = "../../../../crates/cvimport/tests/perf_gate.rs"]
mod perf_gate;

use aa_desktop::tracker_store::JsonFileStore;
use aa_desktop::Session;
use aa_tracker::{Date, NextAction};
use std::time::Instant;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

const BUDGET_SECS: f64 = 60.0;
const DELTA_FACTOR: f64 = 3.0;

fn job_json(company: &str, title: &str) -> String {
    format!(
        r#"{{"title":"{title}","company":"{company}","requirements":{{"mustHave":["xml"],"niceToHave":[]}}}}"#
    )
}

#[test]
fn story_tracker_journey_perf_delta_gated() {
    let path = std::env::temp_dir().join(format!("aa-tracker-story-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let start = Instant::now();

    let mut s = Session::with_tracker_store(Box::new(JsonFileStore::new(&path)));

    // 1. Track a spread of applications, each submitted at a different past date so a single
    //    `today` lands them across the aging boundaries (some First, some Second, some None).
    let today = Date::new(2026, 3, 20);
    let companies = [
        (
            "Northwind Archives",
            "Senior Archivist",
            Date::new(2026, 3, 16),
        ), // 4 days -> First
        ("Contoso Press", "Editor", Date::new(2026, 3, 11)), // 9 days -> Second
        ("Fabrikam Labs", "Researcher", Date::new(2026, 3, 18)), // 2 days -> None
        ("Tailspin Toys", "Designer", Date::new(2026, 3, 14)), // 6 days -> None gap
    ];

    let mut app_ids = Vec::new();
    for (company, title, submitted) in companies {
        let id = s
            .track_application(&job_json(company, title), vec!["cv_1".into()])
            .unwrap();
        // Advance Discovered -> Tailored -> Applied (Applied stamps `submitted`).
        s.advance_application(&id, "tailored", submitted).unwrap();
        s.advance_application(&id, "applied", submitted).unwrap();
        app_ids.push(id);
    }

    // 2. Add a contact, link it, and record a note on the first application.
    let ct = s
        .add_contact(
            "Robin Quill",
            "Northwind Archives",
            "Talent Lead",
            "linkedin",
        )
        .unwrap();
    s.link_contact(&app_ids[0], &ct).unwrap();
    s.add_note(&app_ids[0], "contacted", "messaged on LinkedIn", today)
        .unwrap();

    // 3. Build the daily call sheet for `today`.
    let sheet = s.daily_call_sheet(today).unwrap();

    // 4. Assert the sheet is well-formed: only the two actionable apps appear, ordered
    //    Second-follow-up (higher priority) before First, each row fully populated.
    assert_eq!(
        sheet.len(),
        2,
        "only the First + Second follow-up apps are actionable"
    );
    assert_eq!(sheet[0].next_action, NextAction::SecondFollowUp);
    assert_eq!(sheet[1].next_action, NextAction::FirstFollowUp);
    for row in &sheet {
        assert!(!row.company.is_empty());
        assert!(!row.role.is_empty());
        assert!(!row.draft_message.is_empty());
        assert!(row.priority_score > 0);
    }
    // The First-follow-up row (Northwind) carries the linked contact.
    let northwind = sheet
        .iter()
        .find(|r| r.company == "Northwind Archives")
        .unwrap();
    assert_eq!(northwind.contact.as_ref().unwrap().name, "Robin Quill");

    // 5. The board lists all four applications.
    assert_eq!(s.list_applications().unwrap().len(), 4);

    let elapsed = start.elapsed().as_secs_f64();
    let _ = std::fs::remove_file(&path);

    // ── perf-delta gate (I6, Finding 3) ─────────────────────────────────────────
    let baseline_path = root().join("doc/perf/desktop-tracker-story-baseline.txt");
    let baseline = perf_gate::read_baseline(&baseline_path);
    perf_gate::enforce_gate(
        "tracker-journey STORY",
        elapsed,
        baseline,
        BUDGET_SECS,
        DELTA_FACTOR,
    );
    eprintln!(
        "[L5 STORY perf] tracker journey: {elapsed:.3}s (budget {BUDGET_SECS}s, baseline {})",
        baseline
            .map(|b| format!("{b:.3}s"))
            .unwrap_or_else(|| "none".to_string())
    );
}
