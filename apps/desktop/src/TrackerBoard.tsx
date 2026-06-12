// TrackerBoard — the item #5 application tracker / CRM surface (R-TRK / R-CSH / R-CRM).
//
// Three panels over the new tracker command surface:
//   - tracker board: columns per AppState; each card shows company/role/state + an advance
//     control that calls advanceApplication(appId, toState, today). An illegal transition
//     surfaces the typed error with role="alert" (NO silent failure).
//   - call-sheet view: dailyCallSheet(today) rows — company, role, application date, follow-up
//     window, contact, suggested channel, next action, draft message, priority score. The
//     "export call sheet" affordance copies the deterministic rows (no LLM).
//   - contact panel: add a contact + link it to an application.
//
// Tests are LOCAL ONLY (the `ui` CI job is continue-on-error, issue #2). The end-to-end
// journey is proven by the Rust command-level STORY (L5) in the blocking rust-workspace job.

import { useState } from "react";
import type {
  AppState,
  CallSheetRow,
  Channel,
  TrackedApplication,
  TrackerCommands,
  TrackerDate,
} from "./commands";

const COLUMNS: AppState[] = [
  "Discovered",
  "Tailored",
  "Applied",
  "FollowUpDue",
  "Interview",
  "Closed",
];

// The deterministic next legal state per current state (the UI's single "advance" button maps
// to the FIRST legal forward edge; mirrors aa_tracker::lifecycle legal_transitions). Closed is
// terminal → no advance.
const NEXT: Partial<Record<AppState, AppState>> = {
  Discovered: "Tailored",
  Tailored: "Applied",
  Applied: "FollowUpDue",
  FollowUpDue: "Interview",
  Interview: "Closed",
};

export interface TrackerBoardProps {
  commands: Pick<
    TrackerCommands,
    "advanceApplication" | "dailyCallSheet" | "addContact" | "linkContact" | "listApplications"
  >;
  applications: TrackedApplication[];
  today: TrackerDate;
}

function fmtDate(d: TrackerDate | null): string {
  return d ? `${d.year}-${String(d.month).padStart(2, "0")}-${String(d.day).padStart(2, "0")}` : "—";
}

export function TrackerBoard({ commands, applications, today }: TrackerBoardProps) {
  const [apps, setApps] = useState<TrackedApplication[]>(applications);
  const [sheet, setSheet] = useState<CallSheetRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => setApps(await commands.listApplications());

  const advance = async (app: TrackedApplication) => {
    const to = NEXT[app.state];
    if (!to) return; // Closed is terminal
    setError(null);
    try {
      await commands.advanceApplication(app.id, to, today);
      await refresh();
    } catch (e) {
      // R-TRK board: an illegal transition surfaces explicitly, never a silent failure.
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const buildSheet = async () => {
    setError(null);
    try {
      setSheet(await commands.dailyCallSheet(today));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="tracker">
      {error && (
        <p role="alert" className="tracker-error">
          {error}
        </p>
      )}

      <section aria-label="Tracker board" className="tracker-board">
        {COLUMNS.map((col) => (
          <div key={col} className="tracker-column" data-state={col}>
            <h3>{col}</h3>
            {apps
              .filter((a) => a.state === col)
              .map((a) => (
                <div key={a.id} className="tracker-card" data-testid={`card-${a.id}`}>
                  <strong>{a.job.company}</strong>
                  <span>{a.job.title}</span>
                  <span className="tracker-card-state">{a.state}</span>
                  {NEXT[a.state] && (
                    <button
                      aria-label={`Advance ${a.id} to ${NEXT[a.state]}`}
                      onClick={() => advance(a)}
                    >
                      → {NEXT[a.state]}
                    </button>
                  )}
                </div>
              ))}
          </div>
        ))}
      </section>

      <section aria-label="Daily call sheet" className="call-sheet">
        <button aria-label="Build call sheet" onClick={buildSheet}>
          Build call sheet
        </button>
        {sheet && (
          <ul>
            {sheet.map((r) => (
              <li key={r.applicationId} data-testid={`row-${r.applicationId}`}>
                <strong>{r.company}</strong> — {r.role} (applied {fmtDate(r.applicationDate)})
                <div>
                  window: day {r.followUpWindow.opensDay}–{r.followUpWindow.closesDay} ·
                  next: {r.nextAction} · via {r.suggestedChannel} · priority {r.priorityScore}
                </div>
                {r.contact && <div>contact: {r.contact.name} ({r.contact.channel})</div>}
                <p className="draft">{r.draftMessage}</p>
              </li>
            ))}
          </ul>
        )}
      </section>

      <ContactPanel
        commands={commands}
        appIds={apps.map((a) => a.id)}
        onError={setError}
      />
    </div>
  );
}

interface ContactPanelProps {
  commands: Pick<TrackerCommands, "addContact" | "linkContact">;
  appIds: string[];
  onError: (msg: string) => void;
}

function ContactPanel({ commands, appIds, onError }: ContactPanelProps) {
  const [name, setName] = useState("");
  const [org, setOrg] = useState("");
  const [role, setRole] = useState("");
  const [channel, setChannel] = useState<Channel>("Email");
  const [contactId, setContactId] = useState<string | null>(null);

  const add = async () => {
    try {
      setContactId(await commands.addContact(name, org, role, channel));
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    }
  };

  const link = async (appId: string) => {
    if (!contactId) return;
    try {
      await commands.linkContact(appId, contactId);
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <section aria-label="Contact panel" className="contact-panel">
      <input aria-label="Contact name" value={name} onChange={(e) => setName(e.target.value)} />
      <input aria-label="Contact org" value={org} onChange={(e) => setOrg(e.target.value)} />
      <input aria-label="Contact role" value={role} onChange={(e) => setRole(e.target.value)} />
      <select
        aria-label="Contact channel"
        value={channel}
        onChange={(e) => setChannel(e.target.value as Channel)}
      >
        {(["Email", "Phone", "LinkedIn", "Other"] as Channel[]).map((c) => (
          <option key={c} value={c}>
            {c}
          </option>
        ))}
      </select>
      <button aria-label="Add contact" onClick={add}>
        Add contact
      </button>
      {contactId && (
        <div data-testid="new-contact">
          <span>added {contactId}</span>
          {appIds.map((id) => (
            <button key={id} aria-label={`Link ${contactId} to ${id}`} onClick={() => link(id)}>
              link to {id}
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
