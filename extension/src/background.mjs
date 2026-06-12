// ─────────────────────────────────────────────────────────────────────────────
// background.mjs — THIN service worker. Receives the clipped Normalized Job JSON
// from the content script and triggers the download handoff (the chosen baseline
// per DISCUSS-HANDOFF). Contains NO extraction logic. Browser-only; the pure
// builder below is exported for unit testing without a browser.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Build the deterministic download filename for a clipped job.
 * @param {number} ts  epoch millis
 * @returns {string}
 */
export function downloadFilename(ts) {
  return `clipped-job-${ts}.json`;
}

/**
 * Build a data: URL carrying the Normalized Job JSON. Used instead of
 * URL.createObjectURL so the handoff works inside a service worker (no DOM) and
 * is unit-testable in plain Node.
 * @param {string} json
 * @returns {string}
 */
export function jobDataUrl(json) {
  const encoded = encodeURIComponent(json);
  return `data:application/json;charset=utf-8,${encoded}`;
}

/**
 * Trigger the download handoff for a clipped job JSON via chrome.downloads.
 * @param {*} chromeApi
 * @param {string} json
 * @param {number} ts
 * @returns {*} the value chrome.downloads.download returns (for testing)
 */
export function handoffDownload(chromeApi, json, ts) {
  return chromeApi.downloads.download({
    url: jobDataUrl(json),
    filename: downloadFilename(ts),
    saveAs: true,
  });
}

/**
 * Register the runtime message listener that performs the download handoff.
 * @param {*} chromeApi
 * @param {() => number} now  injectable clock (defaults to Date.now)
 */
export function registerHandoff(chromeApi, now = () => Date.now()) {
  chromeApi.runtime.onMessage.addListener((message) => {
    if (message && message.type === "clipped-job" && typeof message.json === "string") {
      handoffDownload(chromeApi, message.json, now());
    }
    // clip-error messages are surfaced by the popup; nothing to download.
  });
}

// Register in the browser only.
if (
  typeof globalThis !== "undefined" &&
  typeof globalThis.chrome !== "undefined" &&
  globalThis.chrome?.runtime?.onMessage
) {
  registerHandoff(globalThis.chrome);
}
