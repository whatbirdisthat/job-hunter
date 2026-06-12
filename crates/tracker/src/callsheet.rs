//! Daily call-sheet builder (R-CSH-*). Pure + CLOCK INJECTED — `today` is a parameter.
//! Draft text uses DETERMINISTIC templates only (NO LLM).

use crate::crm::{Application, Channel, Contact};
use crate::date::Date;
use crate::scheduler::{aging_action, AgingAction, FollowUpWindow};
use serde::{Deserialize, Serialize};

/// The deterministic next action for a call-sheet row, derived from the aging action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NextAction {
    FirstFollowUp,
    SecondFollowUp,
}

/// A minimal contact reference embedded in a call-sheet row (R-CSH-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactRef {
    pub name: String,
    pub org: String,
    pub channel: Channel,
}

/// One row of the daily call sheet — every brief field (R-CSH-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSheetRow {
    #[serde(rename = "applicationId")]
    pub application_id: String,
    pub company: String,
    pub role: String,
    #[serde(rename = "applicationDate")]
    pub application_date: Date,
    #[serde(rename = "followUpWindow")]
    pub follow_up_window: FollowUpWindow,
    pub contact: Option<ContactRef>,
    #[serde(rename = "suggestedChannel")]
    pub suggested_channel: Channel,
    #[serde(rename = "nextAction")]
    pub next_action: NextAction,
    #[serde(rename = "draftMessage")]
    pub draft_message: String,
    #[serde(rename = "priorityScore")]
    pub priority_score: u32,
}

/// Deterministic priority: SecondFollowUp outranks First, and within an action more
/// days-overdue ranks higher. Pinned by an L1 test so ordering is reproducible (R-CSH-2).
fn priority_score(action: NextAction, days: i64) -> u32 {
    // Keyed on `NextAction` (only the two actionable variants) so there is no unreachable
    // non-actionable arm — every branch here is exercised (R-CSH-2).
    let base = match action {
        NextAction::SecondFollowUp => 100,
        NextAction::FirstFollowUp => 50,
    };
    base + days.clamp(0, 49) as u32
}

/// Deterministic draft template filled with company/role/contact (R-CSH-3) — NO LLM.
fn draft_message(
    action: NextAction,
    company: &str,
    role: &str,
    contact: Option<&ContactRef>,
) -> String {
    let greeting = match contact {
        Some(c) => format!("Hi {}", c.name),
        None => "Hello".to_string(),
    };
    let nudge = match action {
        NextAction::FirstFollowUp => "I wanted to follow up on my recent application",
        NextAction::SecondFollowUp => "I am following up once more on my application",
    };
    format!(
        "{greeting}, {nudge} for the {role} role at {company}. \
         I remain very interested and would welcome any update on next steps. Thank you."
    )
}

/// Build the daily call sheet for `today` (R-CSH-1..5). Pure; clock-injected. Only
/// applications whose `aging_action` is First/SecondFollowUp appear (R-CSH-4); rows are
/// sorted by `priority_score` desc then `application_id` asc (R-CSH-2).
pub fn build_call_sheet(
    apps: &[Application],
    contacts: &[Contact],
    today: Date,
) -> Vec<CallSheetRow> {
    let mut rows: Vec<CallSheetRow> = Vec::new();
    for app in apps {
        let Some(submitted) = app.submitted else {
            continue; // not yet submitted → no follow-up (R-CSH-4)
        };
        let action = aging_action(submitted, today);
        // Map the aging action to the actionable row fields. Non-actionable actions
        // (None / Archive) are filtered out here (R-CSH-4) — no row is produced, so the
        // downstream window/priority logic is total over the two actionable variants.
        let (next_action, window) = match action {
            AgingAction::FirstFollowUp => (
                NextAction::FirstFollowUp,
                FollowUpWindow {
                    opens_day: 3,
                    closes_day: 5,
                },
            ),
            AgingAction::SecondFollowUp => (
                NextAction::SecondFollowUp,
                FollowUpWindow {
                    opens_day: 7,
                    closes_day: 10,
                },
            ),
            AgingAction::None | AgingAction::Archive => continue, // not actionable (R-CSH-4)
        };
        let days = submitted.days_until(&today);

        let contact = app
            .contact_id
            .as_deref()
            .and_then(|id| contacts.iter().find(|c| c.id == id))
            .map(|c| ContactRef {
                name: c.name.clone(),
                org: c.org.clone(),
                channel: c.channel,
            });
        let suggested_channel = contact
            .as_ref()
            .map(|c| c.channel)
            .unwrap_or(Channel::Email);

        rows.push(CallSheetRow {
            application_id: app.id.clone(),
            company: app.job.company.clone(),
            role: app.job.title.clone(),
            application_date: submitted,
            follow_up_window: window,
            draft_message: draft_message(
                next_action,
                &app.job.company,
                &app.job.title,
                contact.as_ref(),
            ),
            contact,
            suggested_channel,
            next_action,
            priority_score: priority_score(next_action, days),
        });
    }
    rows.sort_by(|a, b| {
        b.priority_score
            .cmp(&a.priority_score)
            .then_with(|| a.application_id.cmp(&b.application_id))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crm::application_id;
    use crate::lifecycle::AppState;

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

    fn app(
        n: usize,
        company: &str,
        title: &str,
        submitted: Option<Date>,
        contact: Option<&str>,
    ) -> Application {
        Application {
            id: application_id(n),
            job: job(company, title),
            document_ids: vec![],
            state: AppState::Applied,
            submitted,
            contact_id: contact.map(|s| s.to_string()),
            notes: vec![],
        }
    }

    fn submitted() -> Date {
        Date::new(2026, 1, 1)
    }
    fn day(offset: u32) -> Date {
        Date::new(2026, 1, 1 + offset)
    }

    fn contacts() -> Vec<Contact> {
        vec![Contact {
            id: "ct_0".into(),
            name: "Robin Quill".into(),
            org: "Northwind Archives".into(),
            role: "Talent Lead".into(),
            channel: Channel::Phone,
        }]
    }

    #[test]
    fn row_carries_every_field() {
        // R-CSH-1 — a row carries every brief field, with a linked contact.
        let apps = vec![app(
            0,
            "Northwind Archives",
            "Senior Archivist",
            Some(submitted()),
            Some("ct_0"),
        )];
        let sheet = build_call_sheet(&apps, &contacts(), day(3));
        assert_eq!(sheet.len(), 1);
        let r = &sheet[0];
        assert_eq!(r.application_id, "ap_0");
        assert_eq!(r.company, "Northwind Archives");
        assert_eq!(r.role, "Senior Archivist");
        assert_eq!(r.application_date, submitted());
        assert_eq!(
            r.follow_up_window,
            FollowUpWindow {
                opens_day: 3,
                closes_day: 5
            }
        );
        assert_eq!(r.contact.as_ref().unwrap().name, "Robin Quill");
        assert_eq!(r.suggested_channel, Channel::Phone); // from the contact
        assert_eq!(r.next_action, NextAction::FirstFollowUp);
        assert!(!r.draft_message.is_empty());
        assert!(r.priority_score > 0);
    }

    #[test]
    fn suggested_channel_defaults_to_email_without_contact() {
        // R-CSH-1 — no linked contact → deterministic default channel.
        let apps = vec![app(0, "Acme", "Engineer", Some(submitted()), None)];
        let sheet = build_call_sheet(&apps, &contacts(), day(3));
        assert_eq!(sheet[0].suggested_channel, Channel::Email);
        assert!(sheet[0].contact.is_none());
    }

    #[test]
    fn ordered_by_priority_then_id() {
        // R-CSH-2 — SecondFollowUp (day 7) outranks FirstFollowUp (day 3); ties break by id.
        let apps = vec![
            app(2, "C2", "R2", Some(submitted()), None), // 2026-01-01 -> day 3 -> First (score 53)
            app(0, "C0", "R0", Some(Date::new(2025, 12, 25)), None), // -> day 10 -> Second (score 110)
            app(1, "C1", "R1", Some(Date::new(2025, 12, 25)), None), // -> day 10 -> Second (tie with ap_0)
        ];
        let today = Date::new(2026, 1, 4);
        let sheet = build_call_sheet(&apps, &contacts(), today);
        let ids: Vec<&str> = sheet.iter().map(|r| r.application_id.as_str()).collect();
        // Second-follow-ups (higher score) first, tie broken by id ascending, then the First.
        assert_eq!(ids, vec!["ap_0", "ap_1", "ap_2"]);
    }

    #[test]
    fn draft_template_fills_company_role() {
        // R-CSH-3 — deterministic template filled with company/role; NO model.
        let apps = vec![app(
            0,
            "Northwind",
            "Archivist",
            Some(submitted()),
            Some("ct_0"),
        )];
        let sheet = build_call_sheet(&apps, &contacts(), day(3));
        let msg = &sheet[0].draft_message;
        assert!(msg.contains("Northwind"));
        assert!(msg.contains("Archivist"));
        assert!(msg.contains("Robin Quill")); // contact greeting
    }

    #[test]
    fn only_actionable_rows() {
        // R-CSH-4 — at a SINGLE `today`, give each app a submitted date that places it on a
        // distinct boundary: None (day 2), gap (day 6), Archive (day 11), unsubmitted, and the
        // one actionable (day 4 -> First). today is fixed; each app's `submitted` differs.
        let today = Date::new(2026, 1, 20);
        let apps = vec![
            app(0, "C0", "R0", Some(Date::new(2026, 1, 18)), None), // 2 days -> None
            app(1, "C1", "R1", Some(Date::new(2026, 1, 14)), None), // 6 days -> None gap
            app(2, "C2", "R2", Some(Date::new(2026, 1, 9)), None),  // 11 days -> Archive
            app(3, "C3", "R3", None, None),                         // unsubmitted -> excluded
            app(4, "C4", "R4", Some(Date::new(2026, 1, 16)), None), // 4 days -> First (actionable)
        ];
        let sheet = build_call_sheet(&apps, &contacts(), today);
        let ids: Vec<&str> = sheet.iter().map(|r| r.application_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["ap_4"],
            "only the day-4 first-follow-up application appears"
        );

        // Sanity: a uniform set submitted on the same day is all-or-nothing per `today`.
        let uniform = vec![app(9, "C9", "R9", Some(submitted()), None)];
        assert!(build_call_sheet(&uniform, &contacts(), day(2)).is_empty()); // day 2 -> None
        assert!(build_call_sheet(&uniform, &contacts(), day(6)).is_empty()); // day 6 -> gap
        assert!(build_call_sheet(&uniform, &contacts(), day(11)).is_empty()); // day 11 -> Archive
    }

    #[test]
    fn clock_injected_two_days_differ() {
        // R-CSH-5 — same data, different today → different sheets.
        let apps = vec![app(0, "C0", "R0", Some(submitted()), None)];
        let s3 = build_call_sheet(&apps, &contacts(), day(3)); // First
        let s100 = build_call_sheet(&apps, &contacts(), day(100)); // Archive -> excluded
        assert_eq!(s3.len(), 1);
        assert!(s100.is_empty());
        assert_ne!(s3, s100);
    }
}
