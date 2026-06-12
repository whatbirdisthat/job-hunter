// Tauri command bindings (R-D3 seam). The UI invokes these; in slice 1 they are
// thin wrappers over the Rust command layer (aa-desktop). The actual `invoke` is
// injected so the review UI is testable headlessly (the STORY journey drives the
// Rust commands directly; the UI component test mocks this surface).

export interface RequirementCoverage {
  requirement: string;
  covered: boolean;
  evidenceIds: string[];
}

export interface CoverageReport {
  mustHave: RequirementCoverage[];
  niceToHave: RequirementCoverage[];
  mustHaveCoverage: number;
  niceHaveCoverage: number;
  fitScore: number;
}

export interface Bullet {
  id: string; // sourceEvidenceId — the evidence ledger atom
  description: string;
  role: string;
}

// ── Item #6: CV templates + ATS-readability + keyword-coverage ───────────────
// The CV template the export flow renders. `Modern` is deferred (not shipped); the
// Rust `CvTemplate` enum is exactly {Classic, Compact}. Unknown strings reject with a
// typed error at the command boundary (R-TPL-7).
export type CvTemplate = "classic" | "compact";

export type AtsCheckId =
  | "ColumnReliance"
  | "OverlyLong"
  | "NonStandardHeadings"
  | "MissingExtractableText"
  | "UnusualFont";
export type AtsStatus = "Pass" | "Warn";

export interface AtsCheck {
  id: AtsCheckId;
  status: AtsStatus;
  message: string;
}
export interface AtsReport {
  checks: AtsCheck[];
}

export type KeywordClass = "MustHave" | "NiceToHave";
export interface KeywordHit {
  keyword: string;
  class: KeywordClass;
  evidenceIds: string[];
}
export interface KeywordCoverage {
  found: KeywordHit[];
  missing: KeywordHit[];
}

// ── Item #5: the application tracker / CRM surface ───────────────────────────
// Mirrors aa_tracker's value types (serde shape) + the new aa_desktop::Session
// tracker commands. Dates marshal as { year, month, day } objects; AppState /
// Channel / Outcome cross the boundary as strings parsed by a ::parse helper on
// the Rust side (typed error, never a panic).

export interface TrackerDate {
  year: number;
  month: number;
  day: number;
}

export type AppState =
  | "Discovered"
  | "Tailored"
  | "Applied"
  | "FollowUpDue"
  | "Interview"
  | "Closed";

export type Channel = "Email" | "Phone" | "LinkedIn" | "Other";
export type Outcome = "Contacted" | "Replied" | "Voicemail" | "NextStep";

export interface TrackedApplication {
  id: string;
  job: { title: string; company: string };
  documentIds: string[];
  state: AppState;
  submitted: TrackerDate | null;
  contactId: string | null;
  notes: { at: TrackerDate; outcome: Outcome; text: string }[];
}

export interface FollowUpWindow {
  opensDay: number;
  closesDay: number;
}

export interface CallSheetRow {
  applicationId: string;
  company: string;
  role: string;
  applicationDate: TrackerDate;
  followUpWindow: FollowUpWindow;
  contact: { name: string; org: string; channel: Channel } | null;
  suggestedChannel: Channel;
  nextAction: "FirstFollowUp" | "SecondFollowUp";
  draftMessage: string;
  priorityScore: number;
}

// The tracker command surface (mirrors the new aa_desktop::Session commands).
export interface TrackerCommands {
  // Create an application (state Discovered) from a NormalizedJob JSON + doc ids.
  trackApplication(jobJson: string, documentIds: string[]): Promise<string>;
  // Advance an application's lifecycle; entering Applied stamps `submitted` with `today`.
  // An illegal transition rejects with a typed tracker error.
  advanceApplication(appId: string, to: AppState, today: TrackerDate): Promise<void>;
  addContact(name: string, org: string, role: string, channel: Channel): Promise<string>;
  linkContact(appId: string, contactId: string): Promise<void>;
  addNote(appId: string, outcome: Outcome, text: string, today: TrackerDate): Promise<void>;
  dailyCallSheet(today: TrackerDate): Promise<CallSheetRow[]>;
  listApplications(): Promise<TrackedApplication[]>;
}

// The command surface (mirrors aa_desktop::Session). Implemented by the Tauri
// bridge at runtime; mocked in tests.
export interface Commands {
  importMasterCv(json: string): Promise<void>;
  // Item #2: parse a PDF/DOCX résumé's bytes into a NEW master-CV document and
  // return its JSON for review (R-CVI-10). The bytes cross the Tauri boundary as a
  // number[] (Array.from(new Uint8Array(file))) which marshals to Rust Vec<u8>;
  // `kind` is "pdf" | "docx". Installation reuses importMasterCv (slice-1 path).
  importResume(bytes: number[], kind: "pdf" | "docx"): Promise<string>;
  parseJob(rawJd: string): Promise<void>;
  computeCoverage(): Promise<CoverageReport>;
  tailoredBullets(): Promise<Bullet[]>;
  setDecision(evidenceId: string, approved: boolean): Promise<void>;
  // Item #3 (R-ADV-13): opt INTO the Applicant Advocate LLM rewrite. Default OFF; the
  // user toggles it in the review step. Mirrors aa_desktop::Session::set_advocate_enabled.
  setAdvocateEnabled(enabled: boolean): Promise<void>;
  // Item #3: export now reports `aiUsed` (R-ADV-10) so the UI can show an "AI was used"
  // badge. Surface-only provenance; no persistence this slice.
  // Item #6 (R-TPL-6): the export flow now takes an OPTIONAL CV template. Omitted /
  // undefined → Classic (backward-compatible, R-TPL-5). An unknown string rejects with
  // a typed error on the Rust side (R-TPL-7).
  exportApplication(template?: CvTemplate): Promise<{
    cvPdfLen: number;
    coverLetterPdfLen: number;
    aiUsed: boolean;
  }>;
  // Item #6 (R-ATS-1): the ATS-readability report for the current tailored view under a
  // chosen template. Pure/read-only on the Rust side.
  atsReport(template: CvTemplate): Promise<AtsReport>;
  // Item #6 (R-KWC-1): the keyword-coverage report over the current tailored view + job.
  keywordCoverage(): Promise<KeywordCoverage>;
}

// The full runtime surface composes the slice-1..3 commands with the item-#5 tracker
// commands. Kept as a separate interface (above) so the tracker board/call-sheet/contact
// components can depend on JUST the tracker subset in their (local-only) tests.
export type AllCommands = Commands & TrackerCommands;
