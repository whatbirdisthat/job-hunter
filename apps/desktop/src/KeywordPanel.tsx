// KeywordPanel — item #6 capability C UI. Shows, over the TAILORED view, which job
// keywords are FOUND (with the surfaced evidence locations) vs MISSING, split by
// must-have / nice-to-have class. VISIBILITY-ONLY: it reports what the pure Rust
// `keyword_coverage` returns; it never stuffs, reorders, or fabricates keywords.

import { useEffect, useState } from "react";
import type { KeywordCoverage, KeywordHit, Commands } from "./commands";

export interface KeywordPanelProps {
  commands: Pick<Commands, "keywordCoverage">;
}

function classLabel(h: KeywordHit): string {
  return h.class === "MustHave" ? "must-have" : "nice-to-have";
}

export function KeywordPanel({ commands }: KeywordPanelProps) {
  const [cov, setCov] = useState<KeywordCoverage | null>(null);

  useEffect(() => {
    let live = true;
    void commands.keywordCoverage().then((c) => {
      if (live) setCov(c);
    });
    return () => {
      live = false;
    };
  }, [commands]);

  return (
    <section aria-label="Keyword coverage">
      <h2>Keyword coverage</h2>
      <h3>Found ({cov?.found.length ?? 0})</h3>
      <ul>
        {(cov?.found ?? []).map((h) => (
          <li key={`f-${h.keyword}`} data-testid={`kw-found-${h.keyword}`} data-class={h.class}>
            <strong>{h.keyword}</strong> ({classLabel(h)}) —{" "}
            <span>in {h.evidenceIds.join(", ")}</span>
          </li>
        ))}
      </ul>
      <h3>Missing ({cov?.missing.length ?? 0})</h3>
      <ul>
        {(cov?.missing ?? []).map((h) => (
          <li key={`m-${h.keyword}`} data-testid={`kw-missing-${h.keyword}`} data-class={h.class}>
            <strong>{h.keyword}</strong> ({classLabel(h)}) — not surfaced
          </li>
        ))}
      </ul>
    </section>
  );
}
