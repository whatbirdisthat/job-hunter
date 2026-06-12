// ─────────────────────────────────────────────────────────────────────────────
// content.mjs — THIN content script. Contains NO deterministic extraction logic.
//
// It reads the live page's outerHTML, hands the STRING to the pure dom-extract
// core, and posts the resulting NormalizedJob back to the extension. The ONLY
// DOM touch in the whole package lives here. Browser globals are guarded so the
// file imports cleanly under `node --test` (the smoke test exercises the CORE
// over a fixture, never the live DOM).
// ─────────────────────────────────────────────────────────────────────────────

import { extractFromHtml } from "../../packages/capture-core/src/dom-extract.mjs";
import { toJson } from "../../packages/capture-core/src/normalized-job.mjs";

/**
 * Read the current document's markup and reduce it to a NormalizedJob via the
 * pure core. Pure wrapper around the core — given an HTML string, returns the
 * job. Exported so the smoke test can call it without a DOM.
 * @param {string} outerHtml
 * @returns {import("../../packages/capture-core/src/normalized-job.mjs").NormalizedJob}
 */
export function clipFromOuterHtml(outerHtml) {
  return extractFromHtml(outerHtml);
}

/**
 * Capture the active document and return the serialized Normalized Job JSON.
 * Touches the live DOM; only callable in a browser. Kept tiny and logic-free.
 * @returns {string} toJson(job)
 */
export function captureActiveDocument() {
  // `document` exists only in the browser; never referenced at import time.
  const outerHtml = document.documentElement.outerHTML;
  const job = clipFromOuterHtml(outerHtml);
  return toJson(job);
}

/**
 * When injected into a page (programmatically, on the user's click), capture the
 * document and hand the JSON back to the extension via the runtime message bus.
 * Posts a `clip-error` message if capture throws. Parameterized over `chromeApi`
 * and a `capture` function so it is fully unit-testable without a browser.
 * @param {*} chromeApi
 * @param {() => string} [capture]
 */
export function runCapture(chromeApi, capture = captureActiveDocument) {
  try {
    const json = capture();
    chromeApi.runtime.sendMessage({ type: "clipped-job", json });
  } catch (err) {
    chromeApi.runtime.sendMessage({ type: "clip-error", message: String(err) });
  }
}

// Auto-run only inside a browser content-script context (guarded so importing
// this module under `node --test` never touches browser globals).
/* c8 ignore start — browser-only auto-run guard */
if (
  typeof globalThis !== "undefined" &&
  typeof globalThis.chrome !== "undefined" &&
  globalThis.chrome?.runtime?.sendMessage &&
  typeof document !== "undefined"
) {
  runCapture(globalThis.chrome);
}
/* c8 ignore stop */
