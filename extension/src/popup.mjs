// ─────────────────────────────────────────────────────────────────────────────
// popup.mjs — THIN popup wiring. The "Clip this job" button programmatically
// injects the content script into the ACTIVE tab only, gated by the user's
// click (R-EXT-1). No auto-injection, no broad host registration. Contains NO
// extraction logic. Browser-only; guarded so it imports cleanly under node.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Wire the popup button to a gesture-driven capture on the active tab.
 * Exported and parameterized over `chrome` + `document` so the wiring is
 * unit-testable without a real browser.
 * @param {*} chromeApi
 * @param {Document} doc
 */
export function wirePopup(chromeApi, doc) {
  const button = doc.getElementById("clip");
  const status = doc.getElementById("status");
  if (!button) return;

  const setStatus = (msg) => {
    if (status) status.textContent = msg;
  };

  button.addEventListener("click", async () => {
    setStatus("Clipping…");
    try {
      const [tab] = await chromeApi.tabs.query({
        active: true,
        currentWindow: true,
      });
      // Programmatic, gesture-driven injection into THIS tab only.
      await chromeApi.scripting.executeScript({
        target: { tabId: tab.id },
        files: ["src/content.mjs"],
      });
      setStatus("Clipped — saving JSON…");
    } catch (err) {
      setStatus(`Could not clip: ${String(err)}`);
    }
  });
}

// Auto-wire in the browser only.
if (
  typeof globalThis !== "undefined" &&
  typeof globalThis.chrome !== "undefined" &&
  typeof globalThis.document !== "undefined"
) {
  wirePopup(globalThis.chrome, globalThis.document);
}
