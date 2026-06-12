// L3 boundary — both cores' serialized output validates against
// normalized-job.schema.json for EVERY fixture (validateJob → 0 errors), plus
// negative self-tests proving the validator is NON-VACUOUS.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { extractFromHtml } from "../src/dom-extract.mjs";
import { extractFromEml } from "../src/email-extract.mjs";
import { toJson } from "../src/normalized-job.mjs";
import { validateJob } from "../src/validate-job.mjs";

const fx = (name) =>
  readFileSync(
    fileURLToPath(new URL(`../fixtures/${name}`, import.meta.url)),
    "utf8",
  );

const HTML_FIXTURES = [
  "linkedin-job.html",
  "seek-job.html",
  "hostile.html",
  "empty.html",
];
const EML_FIXTURES = ["linkedin-alert.eml", "seek-alert.eml", "multi-posting.eml"];

test("every HTML fixture's dom-extract output validates with 0 errors", () => {
  for (const name of HTML_FIXTURES) {
    const job = extractFromHtml(fx(name));
    const round = JSON.parse(toJson(job));
    assert.deepEqual(validateJob(round), [], `dom-extract ${name}`);
  }
});

test("every EML fixture's email-extract output validates with 0 errors", () => {
  for (const name of EML_FIXTURES) {
    const jobs = extractFromEml(fx(name));
    assert.ok(jobs.length >= 1, `${name} should yield ≥1 job`);
    for (const job of jobs) {
      const round = JSON.parse(toJson(job));
      assert.deepEqual(validateJob(round), [], `email-extract ${name}`);
    }
  }
});

// ── negative self-tests: the validator must REJECT broken shapes ─────────────

test("rejects a non-object value", () => {
  assert.deepEqual(validateJob(42), [{ path: "", message: "must be an object" }]);
  assert.equal(validateJob(null).length, 1);
  assert.equal(validateJob([]).length, 1);
});

test("rejects an unexpected top-level key (additionalProperties:false)", () => {
  const errs = validateJob({
    title: "T",
    company: "C",
    requirements: { mustHave: [], niceToHave: [] },
    source: "https://linkedin.example/job/1",
  });
  assert.ok(errs.some((e) => e.path === "source"));
});

test("rejects a missing requirements object", () => {
  const errs = validateJob({ title: "T", company: "C" });
  assert.ok(errs.some((e) => e.path === "requirements"));
});

test("rejects missing title and company", () => {
  const errs = validateJob({ requirements: { mustHave: [], niceToHave: [] } });
  assert.ok(errs.some((e) => e.path === "title"));
  assert.ok(errs.some((e) => e.path === "company"));
});

test("rejects wrong types on title/company", () => {
  const errs = validateJob({
    title: 1,
    company: false,
    requirements: { mustHave: [], niceToHave: [] },
  });
  assert.ok(errs.some((e) => e.path === "title" && e.message === "must be a string"));
  assert.ok(errs.some((e) => e.path === "company" && e.message === "must be a string"));
});

test("rejects requirements that is not an object", () => {
  const errs = validateJob({ title: "T", company: "C", requirements: [] });
  assert.ok(errs.some((e) => e.path === "requirements" && e.message === "must be an object"));
});

test("rejects missing mustHave / niceToHave inside requirements", () => {
  const errs = validateJob({ title: "T", company: "C", requirements: {} });
  assert.ok(errs.some((e) => e.path === "requirements.mustHave"));
  assert.ok(errs.some((e) => e.path === "requirements.niceToHave"));
});

test("rejects non-string-array mustHave / niceToHave", () => {
  const errs = validateJob({
    title: "T",
    company: "C",
    requirements: { mustHave: [1], niceToHave: "x" },
  });
  assert.ok(errs.some((e) => e.path === "requirements.mustHave"));
  assert.ok(errs.some((e) => e.path === "requirements.niceToHave"));
});

test("rejects an unexpected key inside requirements", () => {
  const errs = validateJob({
    title: "T",
    company: "C",
    requirements: { mustHave: [], niceToHave: [], extra: [] },
  });
  assert.ok(errs.some((e) => e.path === "requirements.extra"));
});

test("rejects wrong types on optional location/responsibilities/keywords", () => {
  const errs = validateJob({
    title: "T",
    company: "C",
    requirements: { mustHave: [], niceToHave: [] },
    location: 5,
    responsibilities: "no",
    keywords: [1, 2],
  });
  assert.ok(errs.some((e) => e.path === "location"));
  assert.ok(errs.some((e) => e.path === "responsibilities"));
  assert.ok(errs.some((e) => e.path === "keywords"));
});

test("accepts a fully-populated valid object including optionals", () => {
  const errs = validateJob({
    title: "T",
    company: "C",
    location: "Remote",
    responsibilities: ["a", "b"],
    requirements: { mustHave: ["x"], niceToHave: ["y"] },
    keywords: ["k"],
  });
  assert.deepEqual(errs, []);
});
