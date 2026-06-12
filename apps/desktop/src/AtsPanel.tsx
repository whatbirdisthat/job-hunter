// AtsPanel — item #6 capability B UI. Shows the ATS-readability report as a list of
// Pass/Warn check rows for the currently-selected CV template. Read-only: it surfaces
// what the pure Rust `ats_report` returns; it never edits the CV.

import { useEffect, useState } from "react";
import type { AtsReport, CvTemplate, Commands } from "./commands";

export interface AtsPanelProps {
  template: CvTemplate;
  commands: Pick<Commands, "atsReport">;
}

export function AtsPanel({ template, commands }: AtsPanelProps) {
  const [report, setReport] = useState<AtsReport | null>(null);

  useEffect(() => {
    let live = true;
    void commands.atsReport(template).then((r) => {
      if (live) setReport(r);
    });
    return () => {
      live = false;
    };
  }, [template, commands]);

  const warnCount = report?.checks.filter((c) => c.status === "Warn").length ?? 0;

  return (
    <section aria-label="ATS readability report">
      <h2>
        ATS readability ({warnCount} warning{warnCount === 1 ? "" : "s"})
      </h2>
      <ul>
        {(report?.checks ?? []).map((c) => (
          <li key={c.id} data-testid={`ats-${c.id}`} data-status={c.status}>
            <strong>{c.status === "Warn" ? "⚠ " : "✓ "}</strong>
            <span>{c.message}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
