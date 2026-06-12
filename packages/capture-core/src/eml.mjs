// ─────────────────────────────────────────────────────────────────────────────
// eml.mjs — zero-dep .eml parsing (scoped per DISCUSS-EML-SCOPE).
//
// Supports: multipart/alternative walk, quoted-printable (=XX + soft line breaks
// =\n / =\r\n) and base64 transfer encodings, UTF-8 only. OUT of scope: nested
// multipart/related, non-UTF-8 charsets, S/MIME, inline images.
//
// parseEml(raw)   → EmlBody[]   decoded body parts ({ contentType, text }).
// selectBody(parts) → string    prefers text/html else text/plain.
// Node 24 built-ins only (Buffer for base64). Never throws on malformed input.
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @typedef {Object} EmlBody
 * @property {string} contentType  lowercased MIME type, e.g. "text/html"
 * @property {string} text         decoded UTF-8 body text
 */

/**
 * Split a raw message (or part) into [headerBlock, body] on the first blank line.
 * Tolerant to both CRLF and LF. If there is no blank line, the whole input is
 * treated as headers with an empty body.
 * @param {string} raw
 * @returns {[string, string]}
 */
function splitHeadersBody(raw) {
  const m = raw.match(/\r?\n\r?\n/);
  if (!m || m.index === undefined) return [raw, ""];
  const headers = raw.slice(0, m.index);
  const body = raw.slice(m.index + m[0].length);
  return [headers, body];
}

/**
 * Unfold and parse headers into a case-insensitive map (last value wins is fine
 * for our needs; we only read Content-Type and Content-Transfer-Encoding).
 * @param {string} headerBlock
 * @returns {Record<string, string>}
 */
function parseHeaders(headerBlock) {
  /** @type {Record<string, string>} */
  const out = {};
  // Unfold continuation lines (leading whitespace continues the prior header).
  const unfolded = headerBlock.replace(/\r?\n[ \t]+/g, " ");
  for (const line of unfolded.split(/\r?\n/)) {
    const idx = line.indexOf(":");
    if (idx === -1) continue;
    const name = line.slice(0, idx).trim().toLowerCase();
    const value = line.slice(idx + 1).trim();
    if (name.length > 0) out[name] = value;
  }
  return out;
}

/**
 * Extract the bare MIME type (lowercased) from a Content-Type value.
 * @param {string | undefined} ct
 * @returns {string}
 */
function mimeType(ct) {
  if (!ct) return "text/plain";
  return ct.split(";")[0].trim().toLowerCase() || "text/plain";
}

/**
 * Extract a boundary parameter from a Content-Type value (quoted or bare).
 * @param {string | undefined} ct
 * @returns {string | null}
 */
function boundaryOf(ct) {
  if (!ct) return null;
  const m = ct.match(/boundary\s*=\s*("([^"]*)"|([^;\s]+))/i);
  if (!m) return null;
  return m[2] !== undefined ? m[2] : m[3];
}

/**
 * Decode quoted-printable text to UTF-8. Handles =XX hex octets and soft line
 * breaks (a trailing '=' at end of line). Decoded octet sequences are reassembled
 * as bytes then UTF-8 decoded so multi-byte characters survive.
 * @param {string} input
 * @returns {string}
 */
export function decodeQuotedPrintable(input) {
  // Remove soft line breaks first (=<CRLF> or =<LF>).
  const noSoft = input.replace(/=\r?\n/g, "");
  /** @type {number[]} */
  const bytes = [];
  for (let i = 0; i < noSoft.length; i++) {
    const c = noSoft[i];
    if (c === "=" && i + 2 < noSoft.length + 1) {
      const hex = noSoft.slice(i + 1, i + 3);
      if (/^[0-9a-fA-F]{2}$/.test(hex)) {
        bytes.push(Number.parseInt(hex, 16));
        i += 2;
        continue;
      }
      // A bare '=' not followed by valid hex: keep literally.
      bytes.push(c.charCodeAt(0));
      continue;
    }
    // Push the UTF-8 bytes of this character (covers any already-literal unicode).
    const enc = Buffer.from(c, "utf8");
    for (const b of enc) bytes.push(b);
  }
  return Buffer.from(bytes).toString("utf8");
}

/**
 * Decode a base64 body to UTF-8 text (tolerant of embedded whitespace).
 * @param {string} input
 * @returns {string}
 */
export function decodeBase64(input) {
  const clean = input.replace(/\s+/g, "");
  return Buffer.from(clean, "base64").toString("utf8");
}

/**
 * Decode a body according to its transfer encoding.
 * @param {string} body
 * @param {string | undefined} cte  content-transfer-encoding (lowercased)
 * @returns {string}
 */
function decodeBody(body, cte) {
  switch ((cte || "").toLowerCase()) {
    case "quoted-printable":
      return decodeQuotedPrintable(body);
    case "base64":
      return decodeBase64(body);
    default:
      // 7bit / 8bit / binary / none → as-is.
      return body;
  }
}

/**
 * Walk one MIME entity, collecting leaf body parts. Recurses into
 * multipart/alternative (and any multipart/*) by its boundary.
 * @param {string} raw  the entity (headers + body)
 * @param {EmlBody[]} acc
 * @param {number} depth  recursion guard
 */
function walkEntity(raw, acc, depth) {
  if (depth > 16) return; // defensive bound; never throws
  const [headerBlock, body] = splitHeadersBody(raw);
  const headers = parseHeaders(headerBlock);
  const ct = headers["content-type"];
  const type = mimeType(ct);
  const boundary = boundaryOf(ct);

  if (type.startsWith("multipart/") && boundary) {
    // Split on --boundary ; ignore the preamble and the closing --boundary--.
    const delim = `--${boundary}`;
    const segments = body.split(delim);
    // segments[0] is the preamble (ignored). The closing marker yields a final
    // segment starting with "--"; skip empty/closing segments.
    for (let i = 1; i < segments.length; i++) {
      let seg = segments[i];
      if (seg.startsWith("--")) continue; // closing boundary
      // Trim a single leading CRLF/LF that follows the boundary line, and the
      // single trailing CRLF/LF that belongs to the NEXT boundary delimiter
      // (RFC 2046: the CRLF preceding a boundary is part of the boundary).
      seg = seg.replace(/^\r?\n/, "").replace(/\r?\n$/, "");
      walkEntity(seg, acc, depth + 1);
    }
    return;
  }

  // Leaf part.
  const decoded = decodeBody(body, headers["content-transfer-encoding"]);
  acc.push({ contentType: type, text: decoded });
}

/**
 * Parse a raw .eml string into decoded body parts. Never throws.
 * @param {string} raw
 * @returns {EmlBody[]}
 */
export function parseEml(raw) {
  if (typeof raw !== "string" || raw.length === 0) return [];
  /** @type {EmlBody[]} */
  const acc = [];
  walkEntity(raw, acc, 0);
  return acc;
}

/**
 * Select the best body for extraction: prefer text/html, else text/plain, else
 * the first part, else empty string.
 * @param {EmlBody[]} parts
 * @returns {string}
 */
export function selectBody(parts) {
  if (!Array.isArray(parts) || parts.length === 0) return "";
  const html = parts.find((p) => p.contentType === "text/html");
  if (html) return html.text;
  const plain = parts.find((p) => p.contentType === "text/plain");
  if (plain) return plain.text;
  return parts[0].text;
}
