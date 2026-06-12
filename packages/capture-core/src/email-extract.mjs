// ─────────────────────────────────────────────────────────────────────────────
// email-extract.mjs — the email-extract CORE (pure; .eml string in, jobs out).
//
// parseEml → selectBody → htmlToText → split postings on the §F
// "We are hiring a/an {role} at {company}." sentence boundary → parseJd each.
// Returns an ARRAY of NormalizedJob; an alert with no postings yields []. The
// shared jd-core guarantees dom-extract and email-extract cannot diverge.
// Never throws. Node 24 built-ins only.
// ─────────────────────────────────────────────────────────────────────────────

import { parseEml, selectBody } from "./eml.mjs";
import { htmlToText } from "./html-text.mjs";
import { parseJd } from "./jd-core.mjs";

// Deterministic posting boundary (DISCUSS-MULTI-POSTING): each posting starts at
// a "We are hiring a/an " sentence. We split the normalized text so each segment
// owns exactly one posting (its hiring sentence + the following requirement text,
// up to the next hiring sentence).
const HIRING_RE = /We are hiring an? /gi;

/**
 * Split a normalized text block into per-posting segments on the §F hiring
 * sentence boundary. Text before the first hiring sentence is discarded (chrome).
 * @param {string} text
 * @returns {string[]}
 */
export function splitPostings(text) {
  /** @type {number[]} */
  const starts = [];
  HIRING_RE.lastIndex = 0;
  let m;
  // HIRING_RE always matches a non-empty token (≥ "We are hiring a "), so the
  // global-regex cursor always advances — no zero-width guard is needed.
  while ((m = HIRING_RE.exec(text)) !== null) {
    starts.push(m.index);
  }
  if (starts.length === 0) return [];
  /** @type {string[]} */
  const segments = [];
  for (let i = 0; i < starts.length; i++) {
    const from = starts[i];
    const to = i + 1 < starts.length ? starts[i + 1] : text.length;
    const seg = text.slice(from, to).trim();
    if (seg.length > 0) segments.push(seg);
  }
  return segments;
}

/**
 * Extract an array of NormalizedJob from a raw .eml string.
 * @param {string} raw
 * @returns {import("./normalized-job.mjs").NormalizedJob[]}
 */
export function extractFromEml(raw) {
  const parts = parseEml(raw);
  const body = selectBody(parts);
  const text = htmlToText(body);
  const segments = splitPostings(text);
  return segments.map((seg) => parseJd(seg));
}
