#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// validate-job.js — zero-dependency structural validator for Normalized Job JSON.
//
// Sibling of validate.js (which validates the master-CV schema). This one lints
// files against doc/schemas/normalized-job.schema.json: camelCase;
// additionalProperties:false; required title/company/requirements{mustHave,
// niceToHave}; optional location/responsibilities/keywords. Exits non-zero on any
// violation. Zero npm deps; Node built-ins only.
//
// The validation logic is duplicated here (not imported) so the CLI shim stays a
// standalone CommonJS script symmetric with validate.js, while the test path
// imports the ESM core packages/capture-core/src/validate-job.mjs directly. Both
// enforce the SAME rules.
//
// Usage: node tools/fake-data/validate-job.js clipped-job.json [...]
// ─────────────────────────────────────────────────────────────────────────────

const fs = require("fs");

const TOP_ALLOWED = new Set([
  "title",
  "company",
  "location",
  "responsibilities",
  "requirements",
  "keywords",
]);
const REQ_ALLOWED = new Set(["mustHave", "niceToHave"]);

const isPlainObject = (v) =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const isStringArray = (v) =>
  Array.isArray(v) && v.every((x) => typeof x === "string");

// Mirror of packages/capture-core/src/validate-job.mjs::validateJob.
function validateJob(value) {
  const errors = [];
  const push = (path, message) => errors.push({ path, message });

  if (!isPlainObject(value)) {
    push("", "must be an object");
    return errors;
  }
  for (const key of Object.keys(value)) {
    if (!TOP_ALLOWED.has(key)) push(key, `unexpected property "${key}"`);
  }
  if (!("title" in value)) push("title", 'missing required property "title"');
  else if (typeof value.title !== "string") push("title", "must be a string");

  if (!("company" in value)) push("company", 'missing required property "company"');
  else if (typeof value.company !== "string") push("company", "must be a string");

  if ("location" in value && typeof value.location !== "string")
    push("location", "must be a string");
  if ("responsibilities" in value && !isStringArray(value.responsibilities))
    push("responsibilities", "must be an array of strings");
  if ("keywords" in value && !isStringArray(value.keywords))
    push("keywords", "must be an array of strings");

  if (!("requirements" in value)) {
    push("requirements", 'missing required property "requirements"');
  } else if (!isPlainObject(value.requirements)) {
    push("requirements", "must be an object");
  } else {
    const req = value.requirements;
    for (const key of Object.keys(req)) {
      if (!REQ_ALLOWED.has(key))
        push(`requirements.${key}`, `unexpected property "${key}"`);
    }
    if (!("mustHave" in req))
      push("requirements.mustHave", 'missing required property "mustHave"');
    else if (!isStringArray(req.mustHave))
      push("requirements.mustHave", "must be an array of strings");
    if (!("niceToHave" in req))
      push("requirements.niceToHave", 'missing required property "niceToHave"');
    else if (!isStringArray(req.niceToHave))
      push("requirements.niceToHave", "must be an array of strings");
  }
  return errors;
}

let errors = 0;
const fail = (file, msg) => {
  console.error(`✗ ${file}: ${msg}`);
  errors++;
};

function validateFile(file) {
  let doc;
  try {
    doc = JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (e) {
    fail(file, `invalid JSON: ${e.message}`);
    return;
  }
  const errs = validateJob(doc);
  for (const e of errs) fail(file, `${e.path || "<root>"}: ${e.message}`);
}

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("usage: node validate-job.js <normalized-job.json> [...]");
  process.exit(2);
}
files.forEach(validateFile);

if (errors) {
  console.error(`\n${errors} error(s).`);
  process.exit(1);
}
console.log(
  `✓ ${files.length} document(s) valid against normalized-job schema invariants.`,
);
