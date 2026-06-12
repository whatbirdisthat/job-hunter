// ─────────────────────────────────────────────────────────────────────────────
// dom-extract.mjs — the dom-extract CORE (pure; HTML string in, NormalizedJob out).
//
// htmlToText → parseJd. No live DOM, no browser globals, no network. The content
// script is the ONLY place a live DOM is touched; it passes outerHTML here.
// Never throws: empty / hostile / no-cue input yields a job with empty buckets.
// ─────────────────────────────────────────────────────────────────────────────

import { htmlToText } from "./html-text.mjs";
import { parseJd } from "./jd-core.mjs";

/**
 * Extract a single NormalizedJob from an HTML string.
 * @param {string} html
 * @returns {import("./normalized-job.mjs").NormalizedJob}
 */
export function extractFromHtml(html) {
  const text = htmlToText(html);
  return parseJd(text);
}
