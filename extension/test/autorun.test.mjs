// L4 system — cover the browser-only auto-run bootstrap guards. Each wiring
// module auto-runs a side effect ONLY when chrome/document globals exist. node's
// --test runs each test FILE in its own process, so setting the globals at the
// top of THIS file — before any import of the wiring modules — lets their
// bootstrap guards execute their truthy branch exactly once, in a single cached
// module instance (no cache-busting, no fragmentation).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const fx = (name) =>
  readFileSync(
    fileURLToPath(
      new URL(`../../packages/capture-core/fixtures/${name}`, import.meta.url),
    ),
    "utf8",
  );

// ── Arrange the browser-like globals BEFORE importing the wiring modules. ────
const sentMessages = [];
const downloadCalls = [];
let popupGotElement = false;
let bgRegistered = false;

globalThis.chrome = {
  runtime: {
    sendMessage: (m) => sentMessages.push(m),
    onMessage: { addListener: () => (bgRegistered = true) },
  },
  downloads: { download: (o) => downloadCalls.push(o) },
  tabs: { query: async () => [{ id: 1 }] },
  scripting: { executeScript: async () => {} },
};
globalThis.document = {
  documentElement: { outerHTML: fx("linkedin-job.html") },
  getElementById: () => {
    popupGotElement = true;
    return null; // #clip absent → wirePopup returns after the lookup
  },
};

// Dynamic imports AFTER globals are set: each module's bootstrap guard runs now.
await import("../src/content.mjs");
await import("../src/popup.mjs");
await import("../src/background.mjs");

test("content.mjs auto-run captured the document and posted a clipped-job", () => {
  assert.ok(
    sentMessages.some(
      (m) => m.type === "clipped-job" && JSON.parse(m.json).company === "Northwind Robotics",
    ),
  );
});

test("popup.mjs auto-wired against the document on import", () => {
  assert.equal(popupGotElement, true);
});

test("background.mjs auto-registered its message listener on import", () => {
  assert.equal(bgRegistered, true);
});
