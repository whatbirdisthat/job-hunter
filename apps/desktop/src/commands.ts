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
  exportApplication(): Promise<{ cvPdfLen: number; coverLetterPdfLen: number }>;
}
