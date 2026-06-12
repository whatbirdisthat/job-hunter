// L5 STORY — the two human journeys, perf-instrumented and gated.
//
//   1. clip journey : html fixture → extractFromHtml → toJson → validateJob green
//   2. email journey: eml fixture  → extractFromEml  → toJson → validateJob green
//
// Each measures parse wall-clock and asserts against TWO independent obligations
// (mirrors doc/perf/README.md):
//   • Absolute budget  : elapsed < BUDGET_MS
//   • Regression delta : elapsed <= baseline * DELTA_FACTOR
// Both arms are capable of FAILING (non-vacuous): the measured time is compared
// to the committed baseline file, not to a self-ratcheted value.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { extractFromHtml } from "../src/dom-extract.mjs";
import { extractFromEml } from "../src/email-extract.mjs";
import { toJson } from "../src/normalized-job.mjs";
import { validateJob } from "../src/validate-job.mjs";

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const fx = (name) => readFileSync(here(`../fixtures/${name}`), "utf8");
const baselineSec = (name) =>
  Number.parseFloat(readFileSync(here(`../../../doc/perf/${name}`), "utf8").trim());

// Absolute budget: a single deterministic parse must complete well under 1 s on
// any CI runner. DELTA_FACTOR mirrors the Rust gate (3×) — a >3× regression over
// the committed baseline fails even while under the absolute budget.
const BUDGET_MS = 1000;
const DELTA_FACTOR = 3.0;

/**
 * Run `fn` once, return the best-of-N elapsed ms (reduces scheduler flake while
 * keeping the gate honest — a genuine slowdown raises even the minimum).
 * @param {() => void} fn
 * @returns {number}
 */
function bestMs(fn, samples = 25) {
  // warm
  for (let i = 0; i < 5; i++) fn();
  let best = Infinity;
  for (let i = 0; i < samples; i++) {
    const t0 = performance.now();
    fn();
    const dt = performance.now() - t0;
    if (dt < best) best = dt;
  }
  return best;
}

/**
 * Assert both perf obligations. Capable of failing on either arm.
 * @param {string} label
 * @param {number} elapsedMs
 * @param {number} baselineMs
 */
function assertPerf(label, elapsedMs, baselineMs) {
  const deltaCeilMs = baselineMs * DELTA_FACTOR;
  // eslint-disable-next-line no-console
  console.log(
    `[L5 STORY perf] ${label}: elapsed=${elapsedMs.toFixed(4)}ms ` +
      `budget=${BUDGET_MS}ms delta_ceiling=${deltaCeilMs.toFixed(2)}ms ` +
      `(baseline=${baselineMs.toFixed(2)}ms × ${DELTA_FACTOR})`,
  );
  assert.ok(
    elapsedMs < BUDGET_MS,
    `${label}: ${elapsedMs.toFixed(4)}ms exceeds absolute budget ${BUDGET_MS}ms`,
  );
  assert.ok(
    elapsedMs <= deltaCeilMs,
    `${label}: ${elapsedMs.toFixed(4)}ms exceeds delta ceiling ${deltaCeilMs.toFixed(2)}ms ` +
      `(baseline ${baselineMs.toFixed(2)}ms × ${DELTA_FACTOR})`,
  );
}

test("clip journey: html → extractFromHtml → toJson → validateJob green + perf gate", () => {
  const html = fx("linkedin-job.html");

  // behaviour
  const job = extractFromHtml(html);
  const json = toJson(job);
  const round = JSON.parse(json);
  assert.deepEqual(validateJob(round), []);
  assert.equal(round.title, "Staff Platform Engineer");
  assert.equal(round.company, "Northwind Robotics");

  // perf
  const elapsed = bestMs(() => extractFromHtml(html));
  const baselineMs = baselineSec("capture-clip-story-baseline.txt") * 1000;
  assertPerf("clip", elapsed, baselineMs);
});

test("email journey: eml → extractFromEml → each toJson → validateJob green + perf gate", () => {
  const eml = fx("multi-posting.eml");

  // behaviour
  const jobs = extractFromEml(eml);
  assert.equal(jobs.length, 3);
  for (const job of jobs) {
    const round = JSON.parse(toJson(job));
    assert.deepEqual(validateJob(round), []);
  }

  // perf
  const elapsed = bestMs(() => extractFromEml(eml));
  const baselineMs = baselineSec("capture-email-story-baseline.txt") * 1000;
  assertPerf("email", elapsed, baselineMs);
});

test("perf gate is NON-VACUOUS: a simulated regression trips the delta arm", () => {
  // Prove the assertion can fail: feed an elapsed far above baseline*factor and
  // assert that assertPerf throws. This guards against a vacuous (always-pass)
  // gate — the exact failure mode doc/perf/README.md warns about.
  const baselineMs = baselineSec("capture-clip-story-baseline.txt") * 1000;
  const regressed = baselineMs * DELTA_FACTOR + 1; // 1ms over the ceiling
  assert.throws(
    () => assertPerf("simulated", regressed, baselineMs),
    /exceeds delta ceiling/,
  );
});
