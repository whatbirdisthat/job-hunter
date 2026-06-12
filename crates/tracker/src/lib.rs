//! aa-tracker — the application tracker / CRM workflow layer (item #5).
//!
//! FOUR pure, clock-injected cores — each a tested coordinate:
//!   - [`lifecycle`] — the application state machine (R-TRK-*)
//!   - [`scheduler`] — the follow-up aging rules (R-SCH-*)
//!   - [`callsheet`] — the deterministic daily call-sheet builder (R-CSH-*)
//!   - [`crm`]       — the recruiter/contact + notes model (R-CRM-*)
//!
//! plus a small [`date::Date`] value type (Ord, serde; calendar-day arithmetic, no timezones).
//!
//! Every core is PURE: value in, value out. `today` is ALWAYS a parameter — NO function here
//! reads the wall clock and NONE performs IO. Persistence (`TrackerStore`/`JsonFileStore`) and
//! the Tauri commands live in the command layer (`apps/desktop/src-tauri`), NOT here, so this
//! crate stays IO-free (R-STO-1). Crate graph (one-way): depends on `aa-core` ONLY.

pub mod callsheet;
pub mod crm;
pub mod date;
pub mod lifecycle;
pub mod scheduler;

pub use callsheet::{build_call_sheet, CallSheetRow, ContactRef, NextAction};
pub use crm::{
    add_note, application_id, contact_for, contact_id, Application, Channel, Contact, Note, Outcome,
};
pub use date::Date;
pub use lifecycle::{
    legal_transitions, transition, AppState, ParseEnumError, TransitionError, ALL_STATES,
};
pub use scheduler::{aging_action, days_since, follow_up_window, AgingAction, FollowUpWindow};

/// The persisted tracker document shape (R-STO-1/3). The cores own this value type so the
/// command-layer store (`apps/desktop/src-tauri`) serializes/deserializes it without
/// re-declaring the shape; the store ITSELF (the IO) lives in the command layer, not here.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackerDoc {
    pub applications: Vec<Application>,
    pub contacts: Vec<Contact>,
}
