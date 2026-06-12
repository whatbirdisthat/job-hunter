// ─────────────────────────────────────────────────────────────────────────────
// index.mjs — public re-exports for the capture-core package.
// Zero dependencies; Node 24 built-ins only.
// ─────────────────────────────────────────────────────────────────────────────

export { toJson } from "./normalized-job.mjs";
export {
  MUST_CUES,
  NICE_CUES,
  splitItems,
  parseTitle,
  parseCompany,
  parseJd,
} from "./jd-core.mjs";
export { htmlToText } from "./html-text.mjs";
export {
  parseEml,
  selectBody,
  decodeQuotedPrintable,
  decodeBase64,
} from "./eml.mjs";
export { extractFromHtml } from "./dom-extract.mjs";
export { extractFromEml, splitPostings } from "./email-extract.mjs";
export { validateJob } from "./validate-job.mjs";
