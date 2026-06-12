// L1 unit — jd-core.mjs. Mirrors EVERY Rust test in crates/jobparse/src/lib.rs
// (the §F oracle), plus extra coordinates for branches the Rust port introduces.
//
// ─── ORACLE RELATIONSHIP (read before changing these tests) ──────────────────
// These cases are MIRRORED from crates/jobparse/src/lib.rs::parse. That Rust
// crate is the authoritative oracle for §F; jd-core.mjs is a semantic port of
// it. The port reproduces the oracle EXACTLY, with one documented, intentional
// divergence-CLASS (not a single case):
//
//   • The Rust oracle finds cue offsets in `raw.to_lowercase()` and slices the
//     ORIGINAL `raw` with them. For any input containing a character whose
//     Unicode lowercase mapping changes UTF-8 byte length, those offsets no
//     longer align to `raw`, and the oracle misbehaves — a whole FAMILY:
//       – SHRINK (lowercase shorter, e.g. U+1E9E 'ẞ' → 'ß'): Rust PANICS
//         (out-of-bounds / non-char-boundary slice).
//       – GROW   (lowercase longer, e.g. U+0130 'İ' → 'i̇'): Rust does NOT panic
//         but returns a CORRUPTED / shifted value.
//     The JS port lowers over a length-preserving byte view, so it does NEITHER:
//     it never panics (shrink) and never corrupts (grow). See the tests
//     "U+1E9E (SHRINK) does NOT panic" and "U+0130 (GROW) does NOT corrupt".
//     This is deliberate, better behaviour (never-panic / no-corruption
//     contract), uniform across the whole family — not an accident.
//
// A LIVE cross-language differential harness (running the Rust oracle at JS-test
// time) would need a Rust toolchain available during `node --test`, which is out
// of scope for the zero-npm / zero-toolchain test gate. The standing guard
// against silent divergence is therefore this discipline: every oracle test is
// mirrored here by hand, and the intentional divergence FAMILY (shrink→Rust-
// panics; grow→Rust-corrupts; JS does neither) is documented and pinned by
// mirrored cases. A future reader changing jd-core.mjs MUST keep these mirrored
// cases in lockstep with lib.rs and must not introduce a NEW, undocumented
// divergence beyond that documented family.
// ─────────────────────────────────────────────────────────────────────────────
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  MUST_CUES,
  NICE_CUES,
  splitItems,
  parseTitle,
  parseCompany,
  parseJd,
} from "../src/jd-core.mjs";
import { toJson } from "../src/normalized-job.mjs";

test("classifies_required_and_nice_cues", () => {
  const j = parseJd("Required: A; B. Nice to have: C; D.");
  assert.deepEqual(j.requirements.mustHave, ["A", "B"]);
  assert.deepEqual(j.requirements.niceToHave, ["C", "D"]);
});

test("each_must_cue_phrase", () => {
  for (const cue of MUST_CUES) {
    const j = parseJd(`${cue}: Thing.`);
    assert.deepEqual(j.requirements.mustHave, ["Thing"], `cue ${cue}`);
  }
});

test("each_nice_cue_phrase", () => {
  for (const cue of NICE_CUES) {
    const j = parseJd(`${cue}: Thing.`);
    assert.deepEqual(j.requirements.niceToHave, ["Thing"], `cue ${cue}`);
  }
});

test("parses_title_and_company", () => {
  const j = parseJd(
    "We are hiring a Senior Backend Engineer at Acme Group. Required: X.",
  );
  assert.equal(j.title, "Senior Backend Engineer");
  assert.equal(j.company, "Acme Group");
});

test("empty_input_yields_empty_buckets", () => {
  const j = parseJd("");
  assert.equal(j.requirements.mustHave.length, 0);
  assert.equal(j.requirements.niceToHave.length, 0);
});

test("no_cue_garbage_no_panic_empty_requirements", () => {
  const j = parseJd(
    "lorem ipsum dolor sit amet without any structure at all",
  );
  assert.equal(j.requirements.mustHave.length, 0);
  assert.equal(j.requirements.niceToHave.length, 0);
});

test("unicode_does_not_panic", () => {
  const j = parseJd("Required: café ☕ skills; 日本語.");
  assert.equal(j.requirements.mustHave.length, 2);
});

test("multi_line_headings", () => {
  const j = parseJd(
    "Required:\n  TypeScript;\n  Python.\nNice to have:\n  GraphQL.",
  );
  assert.ok(j.requirements.mustHave.includes("TypeScript"));
  assert.ok(j.requirements.niceToHave.includes("GraphQL"));
});

test("title_marker_hiring_an_and_no_at", () => {
  const j = parseJd(
    "We are hiring an Engineer without a company clause. Required: X.",
  );
  assert.equal(j.title, "");
  assert.equal(j.company, "");
  assert.deepEqual(j.requirements.mustHave, ["X"]);
});

test("title_marker_hiring_an_with_at", () => {
  const j = parseJd("We are hiring an Architect at Globex. Required: Y.");
  assert.equal(j.title, "Architect");
  assert.equal(j.company, "Globex");
});

test("to_json_round_trips", () => {
  const j = parseJd("Required: A. Nice to have: B.");
  const s = toJson(j);
  const j2 = JSON.parse(s);
  assert.deepEqual(j2.requirements.mustHave, ["A"]);
  assert.deepEqual(j2.requirements.niceToHave, ["B"]);
  // strict shape: empty optionals omitted
  assert.equal("location" in j2, false);
  assert.equal("responsibilities" in j2, false);
  assert.equal("keywords" in j2, false);
});

// ── extra coordinates for the JS port's branches ────────────────────────────

test("splitItems strips trailing period and drops empties", () => {
  assert.deepEqual(splitItems("A; B; C."), ["A", "B", "C"]);
  assert.deepEqual(splitItems(""), []);
  assert.deepEqual(splitItems("  ;  ; X ; "), ["X"]);
  assert.deepEqual(splitItems("Only one."), ["Only one"]);
});

test("splitItems strips ALL trailing dots (mirrors trim_end_matches('.'))", () => {
  // Rust `s.trim().trim_end_matches('.').trim()` removes EVERY trailing '.',
  // not just one. Reviewer evidence (HIGH): the old port stripped only one.
  assert.deepEqual(splitItems("A.."), ["A"]);
  assert.deepEqual(splitItems("X....."), ["X"]);
  // " A . " → trim → "A ." → trim_end_matches('.') → "A " → trim → "A".
  assert.deepEqual(splitItems(" A . "), ["A"]);
});

test("parse strips all trailing dots in a clause (HIGH evidence, parse-level)", () => {
  // "...independently..." — the body trims to text before the LAST '.', leaving
  // "ability to work independently.." which splitItems must reduce to a single
  // dot-free item. Matches Rust parse() exactly.
  const j = parseJd("Required: ability to work independently...");
  assert.deepEqual(j.requirements.mustHave, ["ability to work independently"]);
});

test("parseTitle with no marker returns empty", () => {
  assert.equal(parseTitle("nothing here"), "");
});

test("parseTitle 'hiring a' marker present but no ' at ' → empty", () => {
  // exercises the 'hiring a ' (not 'an') marker-found-but-no-' at ' branch.
  assert.equal(parseTitle("We are hiring a Cook in the kitchen"), "");
});

test("parseCompany with no ' at ' returns empty", () => {
  assert.equal(parseCompany("no company clause"), "");
});

test("parseCompany with ' at ' but no trailing period runs to end", () => {
  assert.equal(parseCompany("We are hiring a Dev at Northwind Robotics"), "Northwind Robotics");
});

test("leading colon and whitespace after cue is stripped", () => {
  const j = parseJd("Essential:   Rust.");
  assert.deepEqual(j.requirements.mustHave, ["Rust"]);
});

test("body empty after last-period trim falls back to whole slice", () => {
  // Oracle: cend after "required"; bodyStart skips ':' + ws → points at '.'.
  // slice = ". Rust"; rsplit_once('.') head = "" → body empty → fallback keeps
  // the whole slice ". Rust"; splitItems trims but the leading '.' is NOT a
  // trailing period, so the item is ". Rust" verbatim (matches Rust exactly).
  const j = parseJd("Required: . Rust");
  assert.deepEqual(j.requirements.mustHave, [". Rust"]);
});

test("no '.' in slice keeps whole slice (lastIndexOf -1 branch)", () => {
  // No period anywhere → lastDot === -1 path; body = whole slice.
  const j = parseJd("Required: Go");
  assert.deepEqual(j.requirements.mustHave, ["Go"]);
});

test("two must cues accumulate", () => {
  const j = parseJd("Required: A. Must have: B.");
  assert.deepEqual(j.requirements.mustHave, ["A", "B"]);
});

test("never throws on lone cue with no body", () => {
  const j = parseJd("required");
  assert.equal(j.requirements.mustHave.length, 0);
  assert.equal(j.requirements.niceToHave.length, 0);
});

// ── FIX 2 (HIGH): cue followed ONLY by ':'/ws → bodyStart = cend (keep colons) ──

test("cue then colon-only retains the colon (Rust unwrap_or(cend))", () => {
  // raw[cend..] = ":" — char_indices().find(non-:/ws) is None, so Rust
  // unwrap_or(cend) keeps bodyStart at the cue end; the ':' stays in the slice.
  // The old port advanced past the ':' to end-of-string and DROPPED it.
  const j = parseJd("you will need:");
  assert.deepEqual(j.requirements.mustHave, [":"]);
});

test("cue then trailing double-colon reproduces Rust '::' (HIGH evidence)", () => {
  // Differential-fuzz input. "required" matches inside "Requiredmandatory::";
  // its body to end-of-string is "::" (all colons → bodyStart stays at cend),
  // so mustHave = ["::"]. "ideally"/"advantageous" yield the nice bucket item.
  const j = parseJd(
    "ideally Senior Engineer must have advantageous; Requiredmandatory::",
  );
  assert.deepEqual(j.requirements.mustHave, ["::"]);
  assert.deepEqual(j.requirements.niceToHave, ["Senior Engineer"]);
});

test("cue then colon-plus-trailing-whitespace retains the colon", () => {
  // raw[cend..] = ":   " — no non-:/ws char to end → bodyStart = cend (Rust
  // unwrap_or(cend)). slice ":   "; rsplit_once('.') None → body ":   ";
  // body.trim()=":" not empty → body stays; splitItems → [":"]. Mirrors Rust.
  const j = parseJd("Required:   ");
  assert.deepEqual(j.requirements.mustHave, [":"]);
  assert.equal(j.requirements.niceToHave.length, 0);
});

test("cue then pure-whitespace-only body yields no item", () => {
  // "Minimum " followed only by whitespace to end: raw[cend..]=" " (one space),
  // no non-:/ws → bodyStart=cend; slice=" "; body=" "; body.trim() empty →
  // fallback keeps " "; splitItems(" ") trims to "" → dropped. No spurious item.
  const j = parseJd("Minimum ");
  assert.equal(j.requirements.mustHave.length, 0);
  assert.equal(j.requirements.niceToHave.length, 0);
});

// ── FIX 3: the intentional deviation-CLASS from the oracle (byte-length folds) ─
// The oracle finds cue offsets in to_lowercase(raw) and slices `raw` with them.
// Any char whose lowercase mapping changes UTF-8 byte length breaks that
// alignment. This is a FAMILY, not a single case: SHRINK → Rust PANICS;
// GROW → Rust returns a CORRUPTED value. The JS port (length-preserving byte
// view) does NEITHER. Both ends of the family are pinned below.

test("U+1E9E (SHRINK) does NOT panic (never-panic contract)", () => {
  // SHRINK case. Rust's parse() PANICS here: to_lowercase() folds 'ẞ' (U+1E9E,
  // 3 bytes) to 'ß' (2 bytes), so offsets found in `lower` mis-slice `raw` at a
  // non-char boundary. This port lowers over a length-preserving byte view → no
  // panic, and returns the parse over the original bytes. One documented end of
  // the divergence FAMILY (see the file header in jd-core.mjs).
  let job;
  assert.doesNotThrow(() => {
    job = parseJd("ẞ Required: Go.");
  });
  assert.ok(job.requirements.mustHave.includes("Go"));
});

test("U+0130 (GROW) does NOT corrupt (no-corruption contract)", () => {
  // GROW case — the OTHER end of the divergence family, pinned so it can never
  // silently change. to_lowercase() folds 'İ' (U+0130 Turkish dotted capital I,
  // 2 bytes) to 'i̇' (U+0069 U+0307, 3 bytes). The Rust oracle does NOT panic
  // (offsets stay in-bounds) but its byte offsets are shifted, so it returns a
  // CORRUPTED value — for this input the Rust oracle yields mustHave = ["o"].
  // This port, lowering over a length-preserving byte view, returns the CLEAN
  // value parsed over the original bytes: mustHave includes "Go". A deliberate,
  // documented divergence in favour of the no-corruption contract.
  let job;
  assert.doesNotThrow(() => {
    job = parseJd("İİİ Required: Go.");
  });
  assert.ok(job.requirements.mustHave.includes("Go"));
  // (Rust oracle would instead return the corrupted ["o"]; this port does not.)
});
