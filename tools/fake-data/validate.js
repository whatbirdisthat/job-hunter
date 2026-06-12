#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// validate.js — zero-dependency structural validator for Master CV documents.
//
// Checks the invariants that matter for doc/schemas/master-cv.schema.json without
// pulling a JSON-Schema engine (keeps the foundation dependency-free; a full ajv
// pass can be wired in later by /ideator + FOUNDRY). Exits non-zero on any error.
//
// Usage: node tools/fake-data/validate.js fixtures/personas/*.cv.json
// ─────────────────────────────────────────────────────────────────────────────

const fs = require("fs");

const TOP_REQUIRED = ["schemaVersion", "person", "experience"];
const PERSON_KEYS = ["name", "professionalTitle", "professionalDescription", "location", "email", "phone", "linkedin", "github", "website", "image"];
const SKILL_LISTS = ["programmingLanguages", "skills", "toolsTechnologies", "asAServices"];

let errors = 0;
const fail = (file, msg) => { console.error(`✗ ${file}: ${msg}`); errors++; };

function validateSkill(file, s, where) {
  if (typeof s.name !== "string" || !s.name) fail(file, `${where}: skill.name must be a non-empty string`);
  if (!Number.isInteger(s.proficiency) || s.proficiency < 1 || s.proficiency > 5)
    fail(file, `${where}: skill.proficiency must be an integer 1–5 (got ${s.proficiency})`);
}

function validate(file) {
  let doc;
  try { doc = JSON.parse(fs.readFileSync(file, "utf8")); }
  catch (e) { fail(file, `invalid JSON: ${e.message}`); return; }

  for (const k of TOP_REQUIRED) if (!(k in doc)) fail(file, `missing required top-level key "${k}"`);
  if (doc.schemaVersion !== "1.0.0") fail(file, `schemaVersion must be "1.0.0"`);

  if (typeof doc.person !== "object" || doc.person === null) fail(file, `person must be an object`);
  else for (const k of Object.keys(doc.person))
    if (!PERSON_KEYS.includes(k)) fail(file, `person has unexpected key "${k}"`);

  for (const list of SKILL_LISTS)
    if (doc[list]) {
      if (!Array.isArray(doc[list])) fail(file, `${list} must be an array`);
      else doc[list].forEach((s, i) => validateSkill(file, s, `${list}[${i}]`));
    }

  if (!Array.isArray(doc.experience)) { fail(file, `experience must be an array`); return; }
  doc.experience.forEach((e, i) => {
    const w = `experience[${i}]`;
    for (const k of ["id", "jobTitle", "businessName", "startDate"])
      if (typeof e[k] !== "string" || !e[k]) fail(file, `${w}: missing/empty "${k}"`);
    if (e.employmentType != null && !["Employer", "Client"].includes(e.employmentType))
      fail(file, `${w}: employmentType must be Employer|Client|null`);
    if (e.achievementsTasks) e.achievementsTasks.forEach((a, j) => {
      if (typeof a.id !== "string" || !a.id) fail(file, `${w}.achievementsTasks[${j}]: missing id`);
      if (typeof a.description !== "string" || !a.description) fail(file, `${w}.achievementsTasks[${j}]: missing description`);
      if (a.emphasise != null && typeof a.emphasise !== "boolean") fail(file, `${w}.achievementsTasks[${j}]: emphasise must be boolean`);
    });
  });
}

const files = process.argv.slice(2);
if (files.length === 0) { console.error("usage: node validate.js <file.cv.json> [...]"); process.exit(2); }
files.forEach(validate);

if (errors) { console.error(`\n${errors} error(s).`); process.exit(1); }
console.log(`✓ ${files.length} document(s) valid against master-cv schema invariants.`);
