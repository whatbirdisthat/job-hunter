//! Recruiter/contact CRM model (R-CRM-*). Pure data + pure transitions over notes.
//! On-device only; NO clock, NO IO — `at`/`today` dates are always supplied by the caller.

use crate::date::Date;
use crate::lifecycle::{AppState, ParseEnumError};
use serde::{Deserialize, Serialize};

/// How a contact is reached (R-CRM-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    Email,
    Phone,
    LinkedIn,
    Other,
}

impl Channel {
    /// Parse a `Channel` from a lowercase string (R-TRK-CMD-3) — never panics.
    pub fn parse(s: &str) -> Result<Channel, ParseEnumError> {
        match s.trim().to_lowercase().as_str() {
            "email" => Ok(Channel::Email),
            "phone" => Ok(Channel::Phone),
            "linkedin" => Ok(Channel::LinkedIn),
            "other" => Ok(Channel::Other),
            other => Err(ParseEnumError {
                kind: "Channel",
                value: other.to_string(),
            }),
        }
    }
}

/// A recruiter / hiring contact (R-CRM-1). `id` is a deterministic synthetic `ct_<n>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub org: String,
    pub role: String,
    pub channel: Channel,
}

/// The outcome of a touchpoint recorded in a note (R-CRM-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Contacted,
    Replied,
    Voicemail,
    NextStep,
}

impl Outcome {
    /// Parse an `Outcome` from a lowercase string (R-TRK-CMD-3) — never panics.
    pub fn parse(s: &str) -> Result<Outcome, ParseEnumError> {
        match s.trim().to_lowercase().as_str() {
            "contacted" => Ok(Outcome::Contacted),
            "replied" => Ok(Outcome::Replied),
            "voicemail" => Ok(Outcome::Voicemail),
            "nextstep" | "next_step" | "next-step" => Ok(Outcome::NextStep),
            other => Err(ParseEnumError {
                kind: "Outcome",
                value: other.to_string(),
            }),
        }
    }
}

/// A timeline event for an application (R-CRM-2/5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub at: Date,
    pub outcome: Outcome,
    pub text: String,
}

/// A tracked job application (R-TRK-6, R-CRM-4/5). `id` is a deterministic `ap_<n>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub job: aa_core::NormalizedJob,
    #[serde(rename = "documentIds")]
    pub document_ids: Vec<String>,
    pub state: AppState,
    pub submitted: Option<Date>,
    #[serde(rename = "contactId")]
    pub contact_id: Option<String>,
    pub notes: Vec<Note>,
}

/// Append a note to the application's timeline (R-CRM-3) — value in, value out, no clock.
pub fn add_note(mut app: Application, note: Note) -> Application {
    app.notes.push(note);
    app
}

/// Resolve the contact linked to an application (R-CRM-4), or `None` when unset/unresolved.
pub fn contact_for<'a>(app: &Application, contacts: &'a [Contact]) -> Option<&'a Contact> {
    let id = app.contact_id.as_deref()?;
    contacts.iter().find(|c| c.id == id)
}

/// Build a deterministic synthetic application id `ap_<n>` (R-TRK-6).
pub fn application_id(n: usize) -> String {
    format!("ap_{n}")
}

/// Build a deterministic synthetic contact id `ct_<n>` (R-CRM-1).
pub fn contact_id(n: usize) -> String {
    format!("ct_{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny synthetic NormalizedJob (PII-free, fictional). Used across crm/callsheet tests.
    pub(crate) fn job(company: &str, title: &str) -> aa_core::NormalizedJob {
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

    fn app(n: usize) -> Application {
        Application {
            id: application_id(n),
            job: job("Northwind Archives", "Senior Archivist"),
            document_ids: vec!["doc_a".into(), "doc_b".into()],
            state: AppState::Discovered,
            submitted: None,
            contact_id: None,
            notes: vec![],
        }
    }

    #[test]
    fn application_links_job_and_ids() {
        // R-TRK-6 — links a NormalizedJob (by value) + caller document_ids; ap_<n> id.
        let a = app(3);
        assert_eq!(a.id, "ap_3");
        assert_eq!(a.job.company, "Northwind Archives");
        assert_eq!(a.document_ids, vec!["doc_a", "doc_b"]);
    }

    #[test]
    fn synthetic_ids() {
        // R-TRK-6 / R-CRM-1 — deterministic synthetic ids.
        assert_eq!(application_id(0), "ap_0");
        assert_eq!(contact_id(7), "ct_7");
    }

    #[test]
    fn contact_fields_and_synthetic_ct_id() {
        // R-CRM-1 — contact carries name/org/role/channel; ct_<n> id.
        let c = Contact {
            id: contact_id(0),
            name: "Robin Quill".into(),
            org: "Northwind Archives".into(),
            role: "Talent Lead".into(),
            channel: Channel::Email,
        };
        assert_eq!(c.id, "ct_0");
        assert_eq!(c.channel, Channel::Email);
    }

    #[test]
    fn channel_parse() {
        // R-TRK-CMD-3 — channel strings parse; bad value is a typed error.
        assert_eq!(Channel::parse("email"), Ok(Channel::Email));
        assert_eq!(Channel::parse("LinkedIn"), Ok(Channel::LinkedIn));
        assert_eq!(Channel::parse("phone"), Ok(Channel::Phone));
        assert_eq!(Channel::parse(" other "), Ok(Channel::Other));
        assert!(Channel::parse("smoke-signal").is_err());
    }

    #[test]
    fn note_outcomes() {
        // R-CRM-2 — outcome strings parse across the four variants.
        assert_eq!(Outcome::parse("contacted"), Ok(Outcome::Contacted));
        assert_eq!(Outcome::parse("replied"), Ok(Outcome::Replied));
        assert_eq!(Outcome::parse("voicemail"), Ok(Outcome::Voicemail));
        assert_eq!(Outcome::parse("next_step"), Ok(Outcome::NextStep));
        let e = Outcome::parse("ghosted").unwrap_err();
        assert_eq!(e.kind, "Outcome");
    }

    #[test]
    fn add_note_appends() {
        // R-CRM-3 — add_note appends, value in / value out.
        let a = app(1);
        assert!(a.notes.is_empty());
        let n = Note {
            at: crate::date::Date::new(2026, 6, 13),
            outcome: Outcome::Contacted,
            text: "left a message".into(),
        };
        let a = add_note(a, n.clone());
        assert_eq!(a.notes, vec![n]);
    }

    #[test]
    fn notes_preserve_insertion_order() {
        // R-CRM-5 — the notes vec IS the timeline, newest appended last.
        let mut a = app(1);
        a = add_note(
            a,
            Note {
                at: crate::date::Date::new(2026, 6, 10),
                outcome: Outcome::Contacted,
                text: "first".into(),
            },
        );
        a = add_note(
            a,
            Note {
                at: crate::date::Date::new(2026, 6, 12),
                outcome: Outcome::Replied,
                text: "second".into(),
            },
        );
        assert_eq!(a.notes.len(), 2);
        assert_eq!(a.notes[0].text, "first");
        assert_eq!(a.notes[1].text, "second");
    }

    #[test]
    fn contact_for_resolves_and_none() {
        // R-CRM-4 — resolves a linked contact; None when unset or unresolved.
        let contacts = vec![Contact {
            id: contact_id(0),
            name: "Robin Quill".into(),
            org: "Northwind Archives".into(),
            role: "Talent Lead".into(),
            channel: Channel::Phone,
        }];
        let mut a = app(1);
        assert!(contact_for(&a, &contacts).is_none()); // unset

        a.contact_id = Some("ct_0".into());
        assert_eq!(contact_for(&a, &contacts).unwrap().name, "Robin Quill"); // resolved

        a.contact_id = Some("ct_missing".into());
        assert!(contact_for(&a, &contacts).is_none()); // unresolved
    }
}
