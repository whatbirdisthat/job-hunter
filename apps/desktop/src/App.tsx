// App — the slice-1 UI shell (5 steps): onboarding/import → JD paste →
// coverage+review (ReviewPanel) → preview → export. Slice 1 wires the review step;
// the other steps are thin stubs over the command surface (the journey is proven by
// the Rust command-level STORY per R-D3).

import { useState } from "react";
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

  const onImported = async (json: string) => {
    await commands.importMasterCv(json);
    setStep("paste");
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
        <button aria-label="Import master CV" onClick={() => onImported("{}")}>
          Import master CV
        </button>
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
