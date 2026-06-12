// App — the slice-1 UI shell (5 steps): onboarding/import → JD paste →
// coverage+review (ReviewPanel) → preview → export. Slice 1 wires the review step;
// the other steps are thin stubs over the command surface (the journey is proven by
// the Rust command-level STORY per R-D3).

import { useRef, useState } from "react";
import { ReviewPanel } from "./ReviewPanel";
import { TrackerBoard } from "./TrackerBoard";
import { AtsPanel } from "./AtsPanel";
import { KeywordPanel } from "./KeywordPanel";
import type {
  Bullet,
  CoverageReport,
  Commands,
  CvTemplate,
  TrackedApplication,
  TrackerCommands,
  TrackerDate,
} from "./commands";

export interface AppProps {
  commands: Commands;
  // Item #5 (optional): the application tracker / CRM surface. When supplied, an "Open
  // tracker" affordance reveals the board / call-sheet / contact panel. Optional so the
  // slice-1..3 flow (and its tests) need not provide the tracker commands.
  tracker?: {
    commands: TrackerCommands;
    applications: TrackedApplication[];
    today: TrackerDate;
  };
}

type Step = "import" | "paste" | "review" | "done";

export function App({ commands, tracker }: AppProps) {
  const [step, setStep] = useState<Step>("import");
  const [showTracker, setShowTracker] = useState(false);
  const [coverage, setCoverage] = useState<CoverageReport | null>(null);
  const [bullets, setBullets] = useState<Bullet[]>([]);
  const [importError, setImportError] = useState<string | null>(null);
  // Item #3 (R-ADV-13): the Applicant Advocate opt-in. OFF by default; surfaced as a
  // toggle in the review step. After export, `aiUsed` drives the "AI was used" badge.
  const [advocateEnabled, setAdvocateEnabled] = useState(false);
  const [aiUsed, setAiUsed] = useState(false);
  // Item #6 (R-TPL-6): the CV template selected in the export flow. Default Classic
  // (backward-compatible). Threaded to export + the ATS/keyword panels.
  const [template, setTemplate] = useState<CvTemplate>("classic");
  const fileRef = useRef<HTMLInputElement>(null);

  const onImported = async (json: string) => {
    await commands.importMasterCv(json);
    setStep("paste");
  };

  // Item #2 (R-CVI-10): import an existing PDF/DOCX résumé. The bytes are parsed
  // into a NEW master-CV document for review; we then route the returned review
  // JSON through the EXISTING importMasterCv install path before advancing.
  const onResumeFile = async (file: File) => {
    setImportError(null);
    const kind = file.name.toLowerCase().endsWith(".pdf") ? "pdf" : "docx";
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    try {
      const reviewJson = await commands.importResume(bytes, kind);
      await onImported(reviewJson);
    } catch (e) {
      setImportError(String(e));
    }
  };

  const onPasted = async (jd: string) => {
    await commands.parseJob(jd);
    setCoverage(await commands.computeCoverage());
    setBullets(await commands.tailoredBullets());
    setStep("review");
  };

  // R-ADV-13: toggle the advocate opt-in; the command layer records it (default OFF).
  const onToggleAdvocate = async (enabled: boolean) => {
    setAdvocateEnabled(enabled);
    await commands.setAdvocateEnabled(enabled);
  };

  const onExport = async () => {
    const result = await commands.exportApplication(template);
    setAiUsed(result.aiUsed);
    setStep("done");
  };

  return (
    <main>
      <h1>Applicant Advocate</h1>
      {tracker && (
        <button aria-label="Open tracker" onClick={() => setShowTracker((v) => !v)}>
          {showTracker ? "Close tracker" : "Open tracker"}
        </button>
      )}
      {tracker && showTracker && (
        <TrackerBoard
          commands={tracker.commands}
          applications={tracker.applications}
          today={tracker.today}
        />
      )}
      {step === "import" && (
        <>
          <button aria-label="Import master CV" onClick={() => onImported("{}")}>
            Import master CV
          </button>
          <button
            aria-label="Import résumé (PDF/DOCX)"
            onClick={() => fileRef.current?.click()}
          >
            Import résumé (PDF/DOCX)
          </button>
          <input
            ref={fileRef}
            type="file"
            accept=".pdf,.docx"
            aria-label="résumé file"
            data-testid="resume-file-input"
            style={{ display: "none" }}
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void onResumeFile(file);
            }}
          />
          {importError && <p role="alert">Import failed: {importError}</p>}
        </>
      )}
      {step === "paste" && (
        <button aria-label="Paste JD" onClick={() => onPasted("Required: X.")}>
          Paste job description
        </button>
      )}
      {step === "review" && (
        <>
          {coverage && (
            <p>
              Fit score: {(coverage.fitScore * 100).toFixed(0)}% — must-have{" "}
              {(coverage.mustHaveCoverage * 100).toFixed(0)}%
            </p>
          )}
          <label>
            <input
              type="checkbox"
              role="switch"
              aria-label="Use Applicant Advocate (AI) to rewrite bullets"
              checked={advocateEnabled}
              onChange={(e) => void onToggleAdvocate(e.target.checked)}
            />
            Use Applicant Advocate (AI) — evidence-bounded, off by default
          </label>
          <label>
            CV template:{" "}
            <select
              aria-label="CV template"
              value={template}
              onChange={(e) => setTemplate(e.target.value as CvTemplate)}
            >
              <option value="classic">Classic (two-column)</option>
              <option value="compact">Compact (single-column, ATS-friendly)</option>
            </select>
          </label>
          <AtsPanel template={template} commands={commands} />
          <KeywordPanel commands={commands} />
          <ReviewPanel bullets={bullets} commands={commands} onExport={onExport} />
        </>
      )}
      {step === "done" && (
        <>
          <p>Exported cv.pdf + cover-letter.pdf.</p>
          {aiUsed && (
            <p role="status" aria-label="AI was used">
              AI was used to rewrite bullets (evidence-bounded).
            </p>
          )}
        </>
      )}
    </main>
  );
}
