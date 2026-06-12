// ─────────────────────────────────────────────────────────────────────────────
// normalized-job.mjs — the strict seam shape (camelCase, mirrors aa-jobparse).
//
// The value contract on the jobparse→core seam (R-D1). Output validates against
// doc/schemas/normalized-job.schema.json: required title/company/requirements
// {mustHave,niceToHave}; optional location/responsibilities/keywords. toJson()
// emits the strict shape and OMITS empty optionals (mirrors serde
// skip_serializing_if = "is_empty" in crates/jobparse).
//
// Zero dependencies. Node 24 built-ins only.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @typedef {Object} Requirements
 * @property {string[]} mustHave
 * @property {string[]} niceToHave
 */

/**
 * @typedef {Object} NormalizedJob
 * @property {string} title
 * @property {string} company
 * @property {string} [location]
 * @property {string[]} [responsibilities]
 * @property {Requirements} requirements
 * @property {string[]} [keywords]
 */

/**
 * Serialize a NormalizedJob to the strict camelCase JSON shape.
 * Mirrors the Rust serde output: empty `location` is omitted, and empty
 * `responsibilities`/`keywords` arrays are omitted. `requirements` is always
 * present with both `mustHave` and `niceToHave` arrays (possibly empty).
 *
 * @param {NormalizedJob} job
 * @returns {string}
 */
export function toJson(job) {
  /** @type {Record<string, unknown>} */
  const out = {
    title: job.title,
    company: job.company,
  };
  if (typeof job.location === "string" && job.location.length > 0) {
    out.location = job.location;
  }
  if (Array.isArray(job.responsibilities) && job.responsibilities.length > 0) {
    out.responsibilities = job.responsibilities;
  }
  out.requirements = {
    mustHave: job.requirements.mustHave,
    niceToHave: job.requirements.niceToHave,
  };
  if (Array.isArray(job.keywords) && job.keywords.length > 0) {
    out.keywords = job.keywords;
  }
  return JSON.stringify(out);
}
