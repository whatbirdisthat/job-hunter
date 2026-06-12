// ─────────────────────────────────────────────────────────────────────────────
// jd-core.mjs — the ONE ported §F implementation (no DOM, no I/O, zero deps).
//
// Ports §F from crates/jobparse/src/lib.rs::parse. The Rust crate is the ORACLE;
// every Rust test is mirrored in test/jd-core.test.mjs. The port is FAITHFUL to
// the oracle's observable semantics EXCEPT for one deliberate divergence-CLASS:
//
//  ⚠ THE INTENTIONAL DEVIATION CLASS (never-panic / no-corruption contract):
//    The Rust oracle computes `lower = raw.to_lowercase()`, finds cue offsets in
//    `lower`, and then slices the ORIGINAL `raw` using those offsets. When
//    to_lowercase() changes the UTF-8 BYTE LENGTH of any character, the offsets
//    found in `lower` no longer align to `raw`, and the oracle misbehaves. This
//    is NOT a single case — it is a whole FAMILY of inputs, namely every input
//    containing a character whose Unicode lowercase mapping changes byte length:
//
//      • SHRINK (lowercase is SHORTER, e.g. U+1E9E 'ẞ' → 'ß', 3 bytes → 2):
//        the Rust offsets run past `raw`'s char boundaries and Rust PANICS
//        (out-of-bounds / non-char-boundary slice at lib.rs:71/110).
//      • GROW (lowercase is LONGER, e.g. U+0130 'İ' Turkish dotted capital I
//        → 'i̇', 2 bytes → 3): the Rust offsets are shifted but in-bounds, so
//        Rust does NOT panic — it returns a CORRUPTED / shifted value.
//
//    This port lowercases over a length-preserving UTF-8 BYTE view (ASCII-only
//    fold, non-ASCII bytes passed through), so `lower` is ALWAYS the same length
//    as the source bytes and every offset stays valid. It therefore neither
//    panics (shrink case) nor corrupts (grow case); it returns the parse computed
//    over the original bytes. This is a DELIBERATE, BETTER, and UNIFORM divergence
//    across that whole family — a documented divergence-CLASS, NOT a single case —
//    in favour of the never-panic / no-corruption contract. It is pinned by the
//    tests "U+1E9E (SHRINK) does NOT panic" and "U+0130 (GROW) does NOT corrupt".
//
// Porting fidelity notes (faithful to the oracle):
//  • Rust indexes by BYTE offset into UTF-8. JS strings are UTF-16. To reproduce
//    `str::find` / `char_indices` / byte slicing, this module works over the
//    UTF-8 byte view of the input and slices by byte offset, decoding back to a
//    string only at the boundaries. So `café ☕; 日本語` behaves like Rust.
//  • earliest cue wins (lowest byte index across MUST_CUES ∪ NICE_CUES).
//  • a cue body starts at the first byte at/after the cue end that is neither
//    ':' nor whitespace; if NONE exists to end-of-string, bodyStart = cue end
//    (the ':'/ws are RETAINED — Rust's `unwrap_or(cend)`).
//  • body terminates at the next cue start (or end of input).
//  • the body is trimmed to the text before its LAST '.'; if that trim leaves the
//    body empty (whitespace only), the whole slice is kept (the Rust fallback).
//  • splitItems: split on ';', trim, strip ALL trailing '.', trim, drop empties
//    (Rust `s.trim().trim_end_matches('.').trim()`).
//  • title = between "hiring a "/"hiring an " and " at "; company = after " at "
//    up to the next '.'.
// ─────────────────────────────────────────────────────────────────────────────

/** @type {readonly string[]} */
export const MUST_CUES = Object.freeze([
  "required",
  "must have",
  "essential",
  "you will need",
  "minimum",
  "mandatory",
]);

/** @type {readonly string[]} */
export const NICE_CUES = Object.freeze([
  "preferred",
  "desirable",
  "bonus",
  "nice to have",
  "advantageous",
  "ideally",
]);

const ENC = new TextEncoder();
const DEC = new TextDecoder("utf-8");

/**
 * ASCII-only lowercase over the UTF-8 byte view. Cues/markers are all ASCII, so
 * only ASCII bytes ever need folding; non-ASCII bytes are passed through
 * unchanged. CRUCIALLY this preserves byte length (1 byte in → 1 byte out),
 * unlike Rust's `str::to_lowercase` which can change byte length and thereby
 * either panic (shrink) or corrupt the slice (grow) when its offsets are used to
 * slice the original `raw` (see the file header's "INTENTIONAL DEVIATION CLASS").
 * Keeping length stable is what gives this port its never-panic / no-corruption
 * contract across that whole family of inputs.
 * @param {Uint8Array} bytes
 * @returns {Uint8Array}
 */
function lowerBytes(bytes) {
  const out = new Uint8Array(bytes.length);
  for (let i = 0; i < bytes.length; i++) {
    const b = bytes[i];
    out[i] = b >= 0x41 && b <= 0x5a ? b + 0x20 : b;
  }
  return out;
}

/**
 * Find the first byte index of `needle` in `hay` at or after `from`.
 * @param {Uint8Array} hay
 * @param {Uint8Array} needle
 * @param {number} from
 * @returns {number} byte index, or -1 if not found
 */
function indexOfBytes(hay, needle, from) {
  // All needles here are non-empty literals (cues, markers, " at ", " ").
  const last = hay.length - needle.length;
  for (let i = from; i <= last; i++) {
    let j = 0;
    while (j < needle.length && hay[i + j] === needle[j]) j++;
    if (j === needle.length) return i;
  }
  return -1;
}

const MUST_CUE_BYTES = MUST_CUES.map((c) => ENC.encode(c));
const NICE_CUE_BYTES = NICE_CUES.map((c) => ENC.encode(c));

/**
 * Find the earliest cue (lowest byte index) at or after `from`.
 * Returns { start, end, isMust } or null. Mirrors Rust `next_cue`.
 * @param {Uint8Array} hayLower
 * @param {number} from
 * @returns {{start:number, end:number, isMust:boolean} | null}
 */
function nextCue(hayLower, from) {
  /** @type {{start:number, end:number, isMust:boolean} | null} */
  let best = null;
  /** @type {[Uint8Array[], boolean][]} */
  const groups = [
    [MUST_CUE_BYTES, true],
    [NICE_CUE_BYTES, false],
  ];
  for (const [cues, isMust] of groups) {
    for (const cue of cues) {
      const rel = indexOfBytes(hayLower, cue, from);
      if (rel !== -1) {
        const start = rel;
        const end = start + cue.length;
        if (best === null || start < best.start) {
          best = { start, end, isMust };
        }
      }
    }
  }
  return best;
}

const SPACE = 0x20;
const TAB = 0x09;
const LF = 0x0a;
const CR = 0x0d;
const FF = 0x0c;
const VT = 0x0b;
const COLON = 0x3a;
const DOT = 0x2e;

/** @param {number} b @returns {boolean} */
function isAsciiWs(b) {
  return b === SPACE || b === TAB || b === LF || b === CR || b === FF || b === VT;
}

/**
 * Trim leading/trailing ASCII whitespace from a string (Rust `str::trim`
 * trims Unicode whitespace, but our slices are split on ';'/decoded text;
 * JS String.prototype.trim covers the unicode-whitespace cases identically
 * enough for our inputs). We use the JS trim for decoded strings.
 * @param {string} s @returns {string}
 */
function jsTrim(s) {
  return s.trim();
}

/**
 * Split a requirement clause body into items on ';', trim, strip ALL trailing
 * '.', trim again, drop empties. Mirrors Rust `split_items` byte-for-byte:
 * Rust does `s.trim().trim_end_matches('.').trim()`. `trim_end_matches('.')`
 * strips EVERY trailing '.', not just one — and is applied between two trims.
 * @param {string} body
 * @returns {string[]}
 */
export function splitItems(body) {
  /** @type {string[]} */
  const out = [];
  for (const seg of body.split(";")) {
    // trim() → trim_end_matches('.') (ALL trailing dots) → trim()
    let s = jsTrim(seg);
    s = s.replace(/\.+$/, "");
    s = jsTrim(s);
    if (s.length > 0) out.push(s);
  }
  return out;
}

/**
 * Title heuristic: text between "hiring a "/"hiring an " and " at ".
 * Mirrors Rust `parse_title` (byte-indexed).
 * @param {string} raw
 * @returns {string}
 */
export function parseTitle(raw) {
  const bytes = ENC.encode(raw);
  const lower = lowerBytes(bytes);
  for (const marker of ["hiring an ", "hiring a "]) {
    const m = ENC.encode(marker);
    const i = indexOfBytes(lower, m, 0);
    if (i !== -1) {
      const start = i + m.length;
      const rel = indexOfBytes(lower, ENC.encode(" at "), start);
      if (rel !== -1) {
        return jsTrim(DEC.decode(bytes.subarray(start, rel)));
      }
    }
  }
  return "";
}

/**
 * Company heuristic: text after " at " up to the next '.'.
 * Mirrors Rust `parse_company` (byte-indexed).
 * @param {string} raw
 * @returns {string}
 */
export function parseCompany(raw) {
  const bytes = ENC.encode(raw);
  const lower = lowerBytes(bytes);
  const i = indexOfBytes(lower, ENC.encode(" at "), 0);
  if (i !== -1) {
    const start = i + 4;
    let end = -1;
    for (let k = start; k < bytes.length; k++) {
      if (bytes[k] === DOT) {
        end = k;
        break;
      }
    }
    if (end === -1) end = bytes.length;
    return jsTrim(DEC.decode(bytes.subarray(start, end)));
  }
  return "";
}

/**
 * Parse a raw JD text block into a NormalizedJob (§F). Never throws; garbage /
 * empty / unicode input yields empty requirement buckets.
 * Mirrors Rust `parse` exactly.
 * @param {string} raw
 * @returns {import("./normalized-job.mjs").NormalizedJob}
 */
export function parseJd(raw) {
  const title = parseTitle(raw);
  const company = parseCompany(raw);

  const bytes = ENC.encode(raw);
  const lower = lowerBytes(bytes);

  /** @type {string[]} */
  const must = [];
  /** @type {string[]} */
  const nice = [];

  let cursor = 0;
  for (;;) {
    const cue = nextCue(lower, cursor);
    if (cue === null) break;
    const { end: cend, isMust } = cue;

    // body_start = raw[cend..].char_indices()
    //   .find(|(_, c)| *c != ':' && !c.is_whitespace())
    //   .map(|(i, _)| cend + i)
    //   .unwrap_or(cend)
    // Scan from cend to END OF STRING for the first byte that is neither ':'
    // nor whitespace. If found, that is bodyStart. If NONE exists (everything
    // from cend onward is ':'/ws), Rust's unwrap_or(cend) keeps bodyStart=cend
    // — the colons/ws are RETAINED in the slice, NOT dropped.
    let bodyStart = cend;
    let found = false;
    for (let i = cend; i < bytes.length; i++) {
      const b = bytes[i];
      if (b !== COLON && !isAsciiWs(b)) {
        bodyStart = i;
        found = true;
        break;
      }
    }
    if (!found) bodyStart = cend;

    const after = nextCue(lower, cend);
    const bodyEnd = after === null ? bytes.length : after.start;

    const sliceBytes = bytes.subarray(bodyStart, bodyEnd);
    const slice = DEC.decode(sliceBytes);

    // trim to the text before the LAST '.'; fallback to whole slice if empty.
    const lastDot = slice.lastIndexOf(".");
    let body = lastDot === -1 ? slice : slice.slice(0, lastDot);
    if (jsTrim(body).length === 0) body = slice;

    const items = splitItems(body);
    if (isMust) {
      for (const it of items) must.push(it);
    } else {
      for (const it of items) nice.push(it);
    }
    cursor = bodyEnd;
  }

  return {
    title,
    company,
    location: "",
    responsibilities: [],
    requirements: {
      mustHave: must,
      niceToHave: nice,
    },
    keywords: [],
  };
}
