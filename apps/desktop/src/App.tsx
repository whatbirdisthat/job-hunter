// App — the slice-1 UI shell (5 steps): onboarding/import → JD paste →
// coverage+review (ReviewPanel) → preview → export. Slice 1 wires the review step;
// the other steps are thin stubs over the command surface (the journey is proven by
// the Rust command-level STORY per R-D3).

import { useRef, useState } from "react";
import { ReviewPanel } from "./ReviewPanel";
import type { Bullet, CoverageReport, Commands } from "./commands";

export interface AppProps {
  commands: Commands;
}

type Step = "import" | "paste" | "review" | "done";

export function App({ commands }: AppProps) {
  const [step, setStep] = useState<Step>("import");
  const [coverage, setCoverage] = useState<CoverageReport | null>(null);
  const [bullets, setBullets] = useState<Bullet[]>([]);
  const [importError, setImportError] = useState<string | null>(null);
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

  const onExport = async () => {
    await commands.exportApplication();
    setStep("done");
  };

  return (
    <main>
      <h1>Applicant Advocate</h1>
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
          <ReviewPanel bullets={bullets} commands={commands} onExport={onExport} />
        </>
      )}
      {step === "done" && <p>Exported cv.pdf + cover-letter.pdf.</p>}
    </main>
  );
}
