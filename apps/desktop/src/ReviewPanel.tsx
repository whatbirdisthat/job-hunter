// ReviewPanel — the coverage + approve/reject review UI (slice step 4).
//
// Shows each tailored bullet with its evidence id (the ledger is a VISIBLE
// feature, not just a guard) and lets the user approve/reject. A rejected bullet is
// dropped from the export (honesty over polish — never fabricated back). The
// approve/reject interaction is the component covered by the R-D3 RTL test.

import { useState } from "react";
import type { Bullet, Commands } from "./commands";

export interface ReviewPanelProps {
  bullets: Bullet[];
  commands: Pick<Commands, "setDecision">;
  onExport?: () => void;
}

type Decision = "approved" | "rejected";

export function ReviewPanel({ bullets, commands, onExport }: ReviewPanelProps) {
  const [decisions, setDecisions] = useState<Record<string, Decision>>(
    Object.fromEntries(bullets.map((b) => [b.id, "approved" as Decision])),
  );

  const decide = async (id: string, decision: Decision) => {
    setDecisions((prev) => ({ ...prev, [id]: decision }));
    await commands.setDecision(id, decision === "approved");
  };

  const approvedCount = Object.values(decisions).filter((d) => d === "approved").length;

  return (
    <section aria-label="Review tailored bullets">
      <h2>Review ({approvedCount}/{bullets.length} approved)</h2>
      <ul>
        {bullets.map((b) => (
          <li key={b.id} data-testid={`bullet-${b.id}`} data-decision={decisions[b.id]}>
            <span>{b.description}</span>
            <small> [evidence: {b.id}]</small>
            <button
              aria-label={`Approve ${b.id}`}
              disabled={decisions[b.id] === "approved"}
              onClick={() => decide(b.id, "approved")}
            >
              Approve
            </button>
            <button
              aria-label={`Reject ${b.id}`}
              disabled={decisions[b.id] === "rejected"}
              onClick={() => decide(b.id, "rejected")}
            >
              Reject
            </button>
          </li>
        ))}
      </ul>
      <button
        aria-label="Export two PDFs"
        disabled={approvedCount === 0}
        onClick={() => onExport?.()}
      >
        Export CV + cover letter
      </button>
    </section>
  );
}
