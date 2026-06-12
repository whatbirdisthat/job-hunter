// ─────────────────────────────────────────────────────────────────────────────
// validate-job.mjs — zero-dep structural validator for
// doc/schemas/normalized-job.schema.json.
//
// Enforces (camelCase, additionalProperties:false at every level):
//   top-level required: title(string), company(string), requirements(object)
//   requirements required: mustHave(string[]), niceToHave(string[])
//   optional: location(string), responsibilities(string[]), keywords(string[])
//
// Returns JobValidationError[] — an EMPTY array means valid. Never throws.
// Node 24 built-ins only.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @typedef {Object} JobValidationError
 * @property {string} path
 * @property {string} message
 */

const TOP_ALLOWED = new Set([
  "title",
  "company",
  "location",
  "responsibilities",
  "requirements",
  "keywords",
]);
const REQ_ALLOWED = new Set(["mustHave", "niceToHave"]);

/** @param {unknown} v @returns {boolean} */
function isPlainObject(v) {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/**
 * @param {unknown} v
 * @returns {boolean} true if v is an array of strings
 */
function isStringArray(v) {
  return Array.isArray(v) && v.every((x) => typeof x === "string");
}

/**
 * Validate a value against the strict Normalized Job schema.
 * @param {unknown} value
 * @returns {JobValidationError[]}
 */
export function validateJob(value) {
  /** @type {JobValidationError[]} */
  const errors = [];
  const push = (path, message) => errors.push({ path, message });

  if (!isPlainObject(value)) {
    push("", "must be an object");
    return errors;
  }
  const obj = /** @type {Record<string, unknown>} */ (value);

  // unexpected top-level keys
  for (const key of Object.keys(obj)) {
    if (!TOP_ALLOWED.has(key)) push(key, `unexpected property "${key}"`);
  }

  // required: title
  if (!("title" in obj)) push("title", 'missing required property "title"');
  else if (typeof obj.title !== "string") push("title", "must be a string");

  // required: company
  if (!("company" in obj)) push("company", 'missing required property "company"');
  else if (typeof obj.company !== "string") push("company", "must be a string");

  // optional: location
  if ("location" in obj && typeof obj.location !== "string") {
    push("location", "must be a string");
  }

  // optional: responsibilities
  if ("responsibilities" in obj && !isStringArray(obj.responsibilities)) {
    push("responsibilities", "must be an array of strings");
  }

  // optional: keywords
  if ("keywords" in obj && !isStringArray(obj.keywords)) {
    push("keywords", "must be an array of strings");
  }

  // required: requirements
  if (!("requirements" in obj)) {
    push("requirements", 'missing required property "requirements"');
  } else if (!isPlainObject(obj.requirements)) {
    push("requirements", "must be an object");
  } else {
    const req = /** @type {Record<string, unknown>} */ (obj.requirements);
    for (const key of Object.keys(req)) {
      if (!REQ_ALLOWED.has(key)) {
        push(`requirements.${key}`, `unexpected property "${key}"`);
      }
    }
    if (!("mustHave" in req)) {
      push("requirements.mustHave", 'missing required property "mustHave"');
    } else if (!isStringArray(req.mustHave)) {
      push("requirements.mustHave", "must be an array of strings");
    }
    if (!("niceToHave" in req)) {
      push("requirements.niceToHave", 'missing required property "niceToHave"');
    } else if (!isStringArray(req.niceToHave)) {
      push("requirements.niceToHave", "must be an array of strings");
    }
  }

  return errors;
}
