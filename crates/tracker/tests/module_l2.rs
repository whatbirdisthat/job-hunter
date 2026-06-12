//! L2 — module: a small assembled tracker scenario in `aa-tracker`. Create an application,
//! advance it through Discovered -> Tailored -> Applied, set `submitted`, then build the call
//! sheet at several `today` values and assert the row's window / next-action track the aging
//! rules. Exercises the four cores together (lifecycle + scheduler + callsheet + crm).

use aa_tracker::{
    add_note, aging_action, application_id, build_call_sheet, contact_for, contact_id, transition,
    AgingAction, AppState, Application, Channel, Contact, Date, NextAction, Note, Outcome,
};

fn job(company: &str, title: &str) -> aa_core::NormalizedJob {
    aa_core::NormalizedJob {
        title: title.to_string(),
        company: company.to_string(),
        location: String::new(),
        responsibilities: vec![],
        requirements: aa_core::Requirements {
            must_have: vec![],
            nice_to_have: vec![],
        },
        keywords: vec![],
    }
}

#[test]
fn assembled_tracker_scenario() {
    let submitted = Date::new(2026, 3, 1);

    // Create a Discovered application linked to a contact.
    let mut app = Application {
        id: application_id(0),
        job: job("Northwind Archives", "Senior Archivist"),
        document_ids: vec!["cv_001".into(), "letter_001".into()],
        state: AppState::Discovered,
        submitted: None,
        contact_id: Some(contact_id(0)),
        notes: vec![],
    };
    let contacts = vec![Contact {
        id: contact_id(0),
        name: "Robin Quill".into(),
        org: "Northwind Archives".into(),
        role: "Talent Lead".into(),
        channel: Channel::LinkedIn,
    }];

    // Advance Discovered -> Tailored -> Applied through the real lifecycle core.
    app.state = transition(app.state, AppState::Tailored).expect("discover->tailor");
    app.state = transition(app.state, AppState::Applied).expect("tailor->apply");
    app.submitted = Some(submitted); // entering Applied stamps the submission date (R-TRK-CMD-2 semantics)

    // The linked contact resolves.
    assert_eq!(contact_for(&app, &contacts).unwrap().name, "Robin Quill");

    // Record a touchpoint on the timeline.
    let app = add_note(
        app,
        Note {
            at: Date::new(2026, 3, 4),
            outcome: Outcome::Contacted,
            text: "left a voicemail with the front desk".into(),
        },
    );
    assert_eq!(app.notes.len(), 1);

    let apps = vec![app];

    // Day 2 after submission: not yet actionable -> empty sheet.
    let day2 = Date::new(2026, 3, 3);
    assert_eq!(aging_action(submitted, day2), AgingAction::None);
    assert!(build_call_sheet(&apps, &contacts, day2).is_empty());

    // Day 4: first follow-up window; one row whose window/next-action track the rules.
    let day4 = Date::new(2026, 3, 5);
    assert_eq!(aging_action(submitted, day4), AgingAction::FirstFollowUp);
    let sheet = build_call_sheet(&apps, &contacts, day4);
    assert_eq!(sheet.len(), 1);
    assert_eq!(sheet[0].next_action, NextAction::FirstFollowUp);
    assert_eq!(sheet[0].follow_up_window.opens_day, 3);
    assert_eq!(sheet[0].follow_up_window.closes_day, 5);
    assert_eq!(sheet[0].suggested_channel, Channel::LinkedIn); // from the linked contact

    // Day 9: second follow-up window.
    let day9 = Date::new(2026, 3, 10);
    assert_eq!(aging_action(submitted, day9), AgingAction::SecondFollowUp);
    let sheet = build_call_sheet(&apps, &contacts, day9);
    assert_eq!(sheet[0].next_action, NextAction::SecondFollowUp);

    // Day 20: archived -> excluded from the call sheet.
    let day20 = Date::new(2026, 3, 21);
    assert_eq!(aging_action(submitted, day20), AgingAction::Archive);
    assert!(build_call_sheet(&apps, &contacts, day20).is_empty());
}

#[test]
fn illegal_advance_is_rejected_in_an_assembled_flow() {
    // A skip-ahead in the assembled flow is a typed error (non-vacuous twin of the happy path).
    let state = AppState::Discovered;
    assert!(transition(state, AppState::Applied).is_err()); // cannot skip Tailored
    let state = transition(state, AppState::Tailored).unwrap();
    assert!(transition(state, AppState::Interview).is_err()); // cannot skip to Interview
}
