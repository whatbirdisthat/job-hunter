// L2 module — dom-extract core: HTML fixture in → valid NormalizedJob out.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { extractFromHtml } from "../src/dom-extract.mjs";
import { validateJob } from "../src/validate-job.mjs";
import { toJson } from "../src/normalized-job.mjs";

const fx = (name) =>
  readFileSync(
    fileURLToPath(new URL(`../fixtures/${name}`, import.meta.url)),
    "utf8",
  );

test("linkedin-job.html → expected title/company/cues, valid", () => {
  const j = extractFromHtml(fx("linkedin-job.html"));
  assert.equal(j.title, "Staff Platform Engineer");
  assert.equal(j.company, "Northwind Robotics");
  assert.deepEqual(j.requirements.mustHave, [
    "Rust",
    "Kubernetes",
    "distributed systems",
  ]);
  assert.deepEqual(j.requirements.niceToHave, [
    "Typst",
    "embedded Linux",
    "fleet telemetry",
  ]);
  assert.deepEqual(validateJob(JSON.parse(toJson(j))), []);
});

test("script/style cue text never leaks into requirements", () => {
  const j = extractFromHtml(fx("linkedin-job.html"));
  const all = [...j.requirements.mustHave, ...j.requirements.niceToHave].join(" ");
  assert.ok(!all.includes("do-not-leak"));
  assert.ok(!all.includes("also-do-not-leak"));
});

test("seek-job.html → essential/preferred map to must/nice, valid", () => {
  const j = extractFromHtml(fx("seek-job.html"));
  assert.equal(j.title, "Senior Data Engineer");
  assert.equal(j.company, "Tinker & Bell Studios");
  assert.deepEqual(j.requirements.mustHave, ["SQL", "Python", "data modelling"]);
  assert.deepEqual(j.requirements.niceToHave, ["dbt", "Airflow", "cost optimisation"]);
  assert.deepEqual(validateJob(JSON.parse(toJson(j))), []);
});

test("hostile.html → tolerated, valid job, no throw, no css/script leak", () => {
  const j = extractFromHtml(fx("hostile.html"));
  assert.equal(j.title, "Resilience Engineer");
  const all = [...j.requirements.mustHave, ...j.requirements.niceToHave].join(" ");
  assert.ok(!all.includes("css-noise"));
  assert.ok(!all.includes("script-noise"));
  assert.deepEqual(validateJob(JSON.parse(toJson(j))), []);
});

test("empty.html → valid job with empty buckets, no throw", () => {
  const j = extractFromHtml(fx("empty.html"));
  assert.equal(j.requirements.mustHave.length, 0);
  assert.equal(j.requirements.niceToHave.length, 0);
  assert.equal(j.title, "");
  assert.equal(j.company, "");
  assert.deepEqual(validateJob(JSON.parse(toJson(j))), []);
});
