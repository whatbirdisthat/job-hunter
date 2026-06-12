// L2 module — email-extract core: .eml fixture in → array of valid jobs out.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { extractFromEml, splitPostings } from "../src/email-extract.mjs";
import { validateJob } from "../src/validate-job.mjs";
import { toJson } from "../src/normalized-job.mjs";

const fx = (name) =>
  readFileSync(
    fileURLToPath(new URL(`../fixtures/${name}`, import.meta.url)),
    "utf8",
  );

test("linkedin-alert.eml (quoted-printable) → 1 valid job", () => {
  const jobs = extractFromEml(fx("linkedin-alert.eml"));
  assert.equal(jobs.length, 1);
  assert.equal(jobs[0].title, "Principal SRE");
  assert.equal(jobs[0].company, "Northwind Robotics");
  assert.deepEqual(jobs[0].requirements.mustHave, [
    "Rust",
    "Kubernetes",
    "incident command",
  ]);
  assert.deepEqual(validateJob(JSON.parse(toJson(jobs[0]))), []);
});

test("seek-alert.eml (base64) → 1 valid job with decoded company", () => {
  const jobs = extractFromEml(fx("seek-alert.eml"));
  assert.equal(jobs.length, 1);
  assert.equal(jobs[0].title, "Lead Frontend Engineer");
  assert.equal(jobs[0].company, "Tinker & Bell Studios");
  assert.deepEqual(validateJob(JSON.parse(toJson(jobs[0]))), []);
});

test("multi-posting.eml → 3 valid jobs (deterministic split)", () => {
  const jobs = extractFromEml(fx("multi-posting.eml"));
  assert.equal(jobs.length, 3);
  assert.deepEqual(
    jobs.map((j) => j.title),
    ["Staff Platform Engineer", "Applied ML Engineer", "Senior Data Engineer"],
  );
  assert.deepEqual(
    jobs.map((j) => j.company),
    ["Northwind Robotics", "Greywater Systems", "Tinker & Bell Studios"],
  );
  for (const j of jobs) assert.deepEqual(validateJob(JSON.parse(toJson(j))), []);
});

test("multi-posting splits on both 'hiring a' and 'hiring an'", () => {
  const jobs = extractFromEml(fx("multi-posting.eml"));
  // the ML role uses "hiring an"; presence proves the 'an' branch is split too.
  assert.ok(jobs.some((j) => j.title === "Applied ML Engineer"));
});

test("an alert with no postings yields an empty array (never throws)", () => {
  const raw = [
    "Content-Type: text/html; charset=UTF-8",
    "",
    "<p>No jobs match your search this week.</p>",
  ].join("\r\n");
  assert.deepEqual(extractFromEml(raw), []);
});

test("garbage / empty input yields an empty array", () => {
  assert.deepEqual(extractFromEml(""), []);
  assert.deepEqual(extractFromEml("not an email at all"), []);
});

test("splitPostings returns [] when no hiring sentence present", () => {
  assert.deepEqual(splitPostings("nothing relevant here"), []);
});

test("splitPostings yields one segment per hiring sentence", () => {
  const text =
    "We are hiring a A at X. Required: r1. We are hiring an B at Y. Essential: r2.";
  const segs = splitPostings(text);
  assert.equal(segs.length, 2);
  assert.ok(segs[0].startsWith("We are hiring a A"));
  assert.ok(segs[1].startsWith("We are hiring an B"));
});
