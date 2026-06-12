// L4 system — the MV3 manifest is valid and minimal.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const manifestPath = fileURLToPath(new URL("../manifest.json", import.meta.url));
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

const ALLOWED_PERMISSIONS = new Set(["activeTab", "scripting", "downloads"]);

test("manifest parses as JSON (already parsed at import — assert shape)", () => {
  assert.equal(typeof manifest, "object");
  assert.notEqual(manifest, null);
});

test("manifest_version is exactly 3", () => {
  assert.equal(manifest.manifest_version, 3);
});

test("permissions are EXACTLY the minimal allow-list", () => {
  assert.ok(Array.isArray(manifest.permissions));
  // every requested permission is in the allow-list…
  for (const p of manifest.permissions) {
    assert.ok(ALLOWED_PERMISSIONS.has(p), `unexpected permission "${p}"`);
  }
  // …and exactly the three are present (no fewer, no duplicates).
  assert.deepEqual([...manifest.permissions].sort(), [
    "activeTab",
    "downloads",
    "scripting",
  ]);
});

test("NO host_permissions key (no broad host access)", () => {
  assert.equal("host_permissions" in manifest, false);
});

test("NO content_scripts key (no auto-injection)", () => {
  assert.equal("content_scripts" in manifest, false);
});

test("description states the compliance posture verbatim", () => {
  const d = manifest.description.toLowerCase();
  assert.ok(d.includes("user-driven only"));
  assert.ok(d.includes("no automated login"));
  assert.ok(d.includes("no background scraping"));
  assert.ok(d.includes("no anti-bot evasion"));
});

test("action.default_popup and background.service_worker are wired", () => {
  assert.equal(manifest.action.default_popup, "src/popup.html");
  assert.equal(manifest.background.service_worker, "src/background.mjs");
});
