#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// validate-tracker.js — zero-dependency structural validator for the Tracker
// Document JSON (item #5). Sibling of validate.js (master-CV) and validate-job.js
// (normalized-job). Lints files against doc/schemas/tracker-doc.schema.json:
// camelCase; additionalProperties:false everywhere; required applications/contacts;
// PascalCase enum strings (AppState / Channel / Outcome). Exits non-zero on any
// violation. Zero npm deps; Node built-ins only.
//
// The validation logic is hand-written here (not a JSON-Schema engine) so the CLI
// shim stays a standalone CommonJS script symmetric with validate-job.js. It
// enforces the SAME invariants the schema declares.
//
// Usage:  node tools/fake-data/validate-tracker.js tracker-doc.json [...]
//         node tools/fake-data/validate-tracker.js --self-test
//   --self-test feeds a hand-broken document and asserts the validator produces a
//   NON-EMPTY error list (proves the validator is non-vacuous), then exits 0.
// ─────────────────────────────────────────────────────────────────────────────

const fs = require("fs");

const APP_STATES = new Set([
  "Discovered",
  "Tailored",
  "Applied",
  "FollowUpDue",
  "Interview",
  "Closed",
]);
const CHANNELS = new Set(["Email", "Phone", "LinkedIn", "Other"]);
const OUTCOMES = new Set(["Contacted", "Replied", "Voicemail", "NextStep"]);

const isPlainObject = (v) =>
  typeof v === "object" && v !== null && !Array.isArray(v);
const isStringArray = (v) =>
  Array.isArray(v) && v.every((x) => typeof x === "string");
const isInteger = (v) => typeof v === "number" && Number.isInteger(v);

function onlyKeys(obj, allowed, path, errors) {
  for (const key of Object.keys(obj)) {
    if (!allowed.has(key))
      errors.push({ path: `${path}${key}`, message: `unexpected property "${key}"` });
  }
}

function validateDate(value, path, errors) {
  if (!isPlainObject(value)) {
    errors.push({ path, message: "must be an object" });
    return;
  }
  onlyKeys(value, new Set(["year", "month", "day"]), `${path}.`, errors);
  for (const k of ["year", "month", "day"]) {
    if (!(k in value)) errors.push({ path: `${path}.${k}`, message: `missing required "${k}"` });
    else if (!isInteger(value[k])) errors.push({ path: `${path}.${k}`, message: "must be an integer" });
  }
}

function validateJob(value, path, errors) {
  if (!isPlainObject(value)) {
    errors.push({ path, message: "must be an object" });
    return;
  }
  const allowed = new Set([
    "title",
    "company",
    "location",
    "responsibilities",
    "requirements",
    "keywords",
  ]);
  onlyKeys(value, allowed, `${path}.`, errors);
  if (typeof value.title !== "string") errors.push({ path: `${path}.title`, message: "must be a string" });
  if (typeof value.company !== "string") errors.push({ path: `${path}.company`, message: "must be a string" });
  if ("location" in value && typeof value.location !== "string")
    errors.push({ path: `${path}.location`, message: "must be a string" });
  if ("responsibilities" in value && !isStringArray(value.responsibilities))
    errors.push({ path: `${path}.responsibilities`, message: "must be an array of strings" });
  if ("keywords" in value && !isStringArray(value.keywords))
    errors.push({ path: `${path}.keywords`, message: "must be an array of strings" });
  if (!isPlainObject(value.requirements)) {
    errors.push({ path: `${path}.requirements`, message: "must be an object" });
  } else {
    onlyKeys(value.requirements, new Set(["mustHave", "niceToHave"]), `${path}.requirements.`, errors);
    if (!isStringArray(value.requirements.mustHave))
      errors.push({ path: `${path}.requirements.mustHave`, message: "must be an array of strings" });
    if (!isStringArray(value.requirements.niceToHave))
      errors.push({ path: `${path}.requirements.niceToHave`, message: "must be an array of strings" });
  }
}

function validateNote(value, path, errors) {
  if (!isPlainObject(value)) {
    errors.push({ path, message: "must be an object" });
    return;
  }
  onlyKeys(value, new Set(["at", "outcome", "text"]), `${path}.`, errors);
  validateDate(value.at, `${path}.at`, errors);
  if (!OUTCOMES.has(value.outcome))
    errors.push({ path: `${path}.outcome`, message: `must be one of ${[...OUTCOMES].join(", ")}` });
  if (typeof value.text !== "string") errors.push({ path: `${path}.text`, message: "must be a string" });
}

function validateApplication(value, path, errors) {
  if (!isPlainObject(value)) {
    errors.push({ path, message: "must be an object" });
    return;
  }
  const allowed = new Set(["id", "job", "documentIds", "state", "submitted", "contactId", "notes"]);
  onlyKeys(value, allowed, `${path}.`, errors);
  for (const k of [...allowed]) {
    if (!(k in value)) errors.push({ path: `${path}.${k}`, message: `missing required "${k}"` });
  }
  if ("id" in value && typeof value.id !== "string")
    errors.push({ path: `${path}.id`, message: "must be a string" });
  if ("job" in value) validateJob(value.job, `${path}.job`, errors);
  if ("documentIds" in value && !isStringArray(value.documentIds))
    errors.push({ path: `${path}.documentIds`, message: "must be an array of strings" });
  if ("state" in value && !APP_STATES.has(value.state))
    errors.push({ path: `${path}.state`, message: `must be one of ${[...APP_STATES].join(", ")}` });
  if ("submitted" in value && value.submitted !== null)
    validateDate(value.submitted, `${path}.submitted`, errors);
  if ("contactId" in value && value.contactId !== null && typeof value.contactId !== "string")
    errors.push({ path: `${path}.contactId`, message: "must be a string or null" });
  if ("notes" in value) {
    if (!Array.isArray(value.notes))
      errors.push({ path: `${path}.notes`, message: "must be an array" });
    else value.notes.forEach((n, i) => validateNote(n, `${path}.notes[${i}]`, errors));
  }
}

function validateContact(value, path, errors) {
  if (!isPlainObject(value)) {
    errors.push({ path, message: "must be an object" });
    return;
  }
  const allowed = new Set(["id", "name", "org", "role", "channel"]);
  onlyKeys(value, allowed, `${path}.`, errors);
  for (const k of ["id", "name", "org", "role"]) {
    if (typeof value[k] !== "string") errors.push({ path: `${path}.${k}`, message: "must be a string" });
  }
  if (!CHANNELS.has(value.channel))
    errors.push({ path: `${path}.channel`, message: `must be one of ${[...CHANNELS].join(", ")}` });
}

// The validator entry point — returns a (possibly empty) list of {path, message}.
function validateTrackerDoc(value) {
  const errors = [];
  if (!isPlainObject(value)) {
    errors.push({ path: "", message: "must be an object" });
    return errors;
  }
  onlyKeys(value, new Set(["applications", "contacts"]), "", errors);
  if (!Array.isArray(value.applications)) {
    errors.push({ path: "applications", message: 'missing/invalid required "applications" array' });
  } else {
    value.applications.forEach((a, i) => validateApplication(a, `applications[${i}]`, errors));
  }
  if (!Array.isArray(value.contacts)) {
    errors.push({ path: "contacts", message: 'missing/invalid required "contacts" array' });
  } else {
    value.contacts.forEach((c, i) => validateContact(c, `contacts[${i}]`, errors));
  }
  return errors;
}

module.exports = { validateTrackerDoc };

// ── CLI ─────────────────────────────────────────────────────────────────────
if (require.main === module) {
  const args = process.argv.slice(2);

  if (args.includes("--self-test")) {
    // Non-vacuous self-test: a hand-broken doc MUST yield a non-empty error list.
    const broken = {
      applications: [
        {
          id: "ap_0",
          job: { title: "T", company: "C", requirements: { mustHave: [], niceToHave: [] } },
          documentIds: [],
          state: "Teleported", // bad enum
          submitted: null,
          contactId: null,
          notes: [],
          surprise: true, // additionalProperties violation
        },
      ],
      contacts: [{ id: "ct_0", name: "N", org: "O", role: "R", channel: "Telepathy" }], // bad enum
    };
    const errs = validateTrackerDoc(broken);
    if (errs.length === 0) {
      console.error("::self-test FAILED:: a hand-broken document produced NO errors (vacuous validator).");
      process.exit(1);
    }
    console.log(`✓ self-test: hand-broken document produced ${errs.length} error(s) (validator is non-vacuous).`);
    // And a known-good minimal doc validates clean.
    const good = { applications: [], contacts: [] };
    const goodErrs = validateTrackerDoc(good);
    if (goodErrs.length !== 0) {
      console.error("::self-test FAILED:: an empty-but-valid document produced errors.");
      process.exit(1);
    }
    console.log("✓ self-test: empty-but-valid document validates clean.");
    process.exit(0);
  }

  const files = args;
  if (files.length === 0) {
    console.error("usage: node validate-tracker.js <tracker-doc.json> [...]   (or --self-test)");
    process.exit(2);
  }
  let errors = 0;
  for (const file of files) {
    let doc;
    try {
      doc = JSON.parse(fs.readFileSync(file, "utf8"));
    } catch (e) {
      console.error(`✗ ${file}: invalid JSON: ${e.message}`);
      errors++;
      continue;
    }
    const errs = validateTrackerDoc(doc);
    for (const e of errs) {
      console.error(`✗ ${file}: ${e.path || "<root>"}: ${e.message}`);
      errors++;
    }
  }
  if (errors) {
    console.error(`\n${errors} error(s).`);
    process.exit(1);
  }
  console.log(`✓ ${files.length} document(s) valid against tracker-doc schema invariants.`);
}
