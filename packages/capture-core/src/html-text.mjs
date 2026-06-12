// ─────────────────────────────────────────────────────────────────────────────
// html-text.mjs — zero-dep tolerant HTML → normalized text block.
//
// Reduces markup to a §F-ready text block: remove <script>/<style> blocks
// entirely, strip every tag to a single space, decode the common named and
// numeric HTML entities, collapse runs of whitespace. Must survive malformed /
// unclosed markup without throwing (the hostile.html fixture). Node 24 only.
// ─────────────────────────────────────────────────────────────────────────────

/** @type {Record<string, string>} */
const NAMED_ENTITIES = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
};

/**
 * Decode a single entity body (the text between '&' and ';').
 * Handles &name; , &#NN; (decimal) and &#xNN; / &#XNN; (hex). Unknown entities
 * are returned re-wrapped (`&body;`) so nothing is silently dropped.
 * @param {string} body
 * @returns {string}
 */
function decodeEntityBody(body) {
  if (body[0] === "#") {
    const hex = body[1] === "x" || body[1] === "X";
    const digits = hex ? body.slice(2) : body.slice(1);
    // The caller's regex already guaranteed ≥1 hex char after the optional
    // 'x'. For the DECIMAL branch those chars may still include a–f (e.g.
    // "&#FF;"), which is not a valid decimal entity → emit it verbatim.
    if (!hex && !/^[0-9]+$/.test(digits)) return `&${body};`;
    const code = Number.parseInt(digits, hex ? 16 : 10);
    // A code point above the Unicode max is out of range → emit verbatim.
    if (code > 0x10ffff) return `&${body};`;
    return String.fromCodePoint(code);
  }
  const named = NAMED_ENTITIES[body];
  return named !== undefined ? named : `&${body};`;
}

/**
 * Decode the supported HTML entities in a text fragment.
 * @param {string} text
 * @returns {string}
 */
function decodeEntities(text) {
  return text.replace(/&(#[xX]?[0-9a-fA-F]+|[a-zA-Z]+);/g, (_m, body) =>
    decodeEntityBody(body),
  );
}

/**
 * Tolerant HTML → normalized text. Never throws.
 * @param {string} html
 * @returns {string}
 */
export function htmlToText(html) {
  if (typeof html !== "string" || html.length === 0) return "";

  // 1. Remove <script>…</script> and <style>…</style> blocks entirely.
  //    Tolerant to case and to an unclosed final block (consume to end).
  let s = html.replace(
    /<script\b[^>]*>[\s\S]*?(?:<\/script\s*>|$)/gi,
    " ",
  );
  s = s.replace(/<style\b[^>]*>[\s\S]*?(?:<\/style\s*>|$)/gi, " ");

  // 2. Strip comments.
  s = s.replace(/<!--[\s\S]*?(?:-->|$)/g, " ");

  // 3. Strip every remaining tag (including malformed/unclosed at EOF) to a space.
  s = s.replace(/<[^>]*>/g, " ");
  // A trailing unclosed '<…' with no '>' — drop from the '<' to end.
  s = s.replace(/<[^>]*$/g, " ");

  // 4. Decode entities.
  s = decodeEntities(s);

  // 5. Collapse all whitespace (incl. decoded &nbsp;) to single spaces, trim.
  s = s.replace(/\s+/g, " ").trim();

  return s;
}
