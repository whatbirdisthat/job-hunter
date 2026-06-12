//! L3 — boundary (output ↔ schema). The persisted `TrackerDoc` JSON round-trips
//! (serialize → deserialize → equal) AND validates against the NEW
//! `doc/schemas/tracker-doc.schema.json` via `tools/fake-data/validate-tracker.js`
//! (R-STO-3). One source of truth for "valid tracker document". A negative twin proves
//! the validator is non-vacuous (a hand-broken doc yields a non-empty error list).

use aa_tracker::{
    application_id, contact_id, AppState, Application, Channel, Contact, Date, Note, Outcome,
    TrackerDoc,
};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Run the Node validator over a tracker-doc JSON string. Each call writes a UNIQUE temp
/// file (pid + atomic seq) so concurrent test threads never race on the same path.
fn validate_with_node(json: &str) -> (bool, String) {
    let root = repo_root();
    let tmp = root.join(format!(
        "tracker-boundary-{}-{}.tracker.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    {
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }
    let out = std::process::Command::new("node")
        .arg(root.join("tools/fake-data/validate-tracker.js"))
        .arg(&tmp)
        .output()
        .expect("node validator runs");
    let _ = std::fs::remove_file(&tmp);
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn sample_doc() -> TrackerDoc {
    TrackerDoc {
        applications: vec![Application {
            id: application_id(0),
            job: aa_core::NormalizedJob {
                title: "Senior Archivist".into(),
                company: "Northwind Archives".into(),
                location: "Remote".into(),
                responsibilities: vec!["curate the collection".into()],
                requirements: aa_core::Requirements {
                    must_have: vec!["xml".into()],
                    nice_to_have: vec!["latin".into()],
                },
                keywords: vec!["archives".into()],
            },
            document_ids: vec!["cv_1".into(), "letter_1".into()],
            state: AppState::Applied,
            submitted: Some(Date::new(2026, 3, 1)),
            contact_id: Some(contact_id(0)),
            notes: vec![Note {
                at: Date::new(2026, 3, 4),
                outcome: Outcome::Contacted,
                text: "left a voicemail".into(),
            }],
        }],
        contacts: vec![Contact {
            id: contact_id(0),
            name: "Robin Quill".into(),
            org: "Northwind Archives".into(),
            role: "Talent Lead".into(),
            channel: Channel::LinkedIn,
        }],
    }
}

#[test]
fn tracker_doc_round_trips() {
    // R-STO-3 — serialize → deserialize → equal.
    let doc = sample_doc();
    let json = serde_json::to_string(&doc).unwrap();
    let back: TrackerDoc = serde_json::from_str(&json).unwrap();
    assert_eq!(doc, back);
}

#[test]
fn tracker_doc_validates_against_schema() {
    // R-STO-3 — the persisted JSON validates against tracker-doc.schema.json.
    let doc = sample_doc();
    let json = serde_json::to_string_pretty(&doc).unwrap();
    let (ok, msg) = validate_with_node(&json);
    assert!(ok, "tracker doc must validate: {msg}");
}

#[test]
fn empty_doc_validates() {
    // An empty-but-valid document (no applications/contacts) validates clean.
    let doc = TrackerDoc {
        applications: vec![],
        contacts: vec![],
    };
    let (ok, msg) = validate_with_node(&serde_json::to_string(&doc).unwrap());
    assert!(ok, "empty tracker doc must validate: {msg}");
}

#[test]
fn broken_doc_is_rejected_non_vacuous() {
    // Non-vacuous twin — a hand-broken doc (bad enum + extra key) MUST fail validation,
    // proving the validator is not vacuously passing.
    let broken = r#"{
        "applications": [{
            "id": "ap_0",
            "job": {"title": "T", "company": "C", "requirements": {"mustHave": [], "niceToHave": []}},
            "documentIds": [],
            "state": "Teleported",
            "submitted": null,
            "contactId": null,
            "notes": [],
            "surprise": true
        }],
        "contacts": []
    }"#;
    let (ok, msg) = validate_with_node(broken);
    assert!(
        !ok,
        "a hand-broken doc must be rejected (non-vacuous): {msg}"
    );
}
