// L1 unit — eml.mjs. MIME walk, quoted-printable, base64, body selection.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  parseEml,
  selectBody,
  decodeQuotedPrintable,
  decodeBase64,
} from "../src/eml.mjs";

const CRLF = "\r\n";

test("empty / non-string input yields no parts", () => {
  assert.deepEqual(parseEml(""), []);
  // @ts-expect-error wrong type on purpose
  assert.deepEqual(parseEml(null), []);
});

test("selectBody on empty array returns empty string", () => {
  assert.equal(selectBody([]), "");
  // @ts-expect-error wrong type on purpose
  assert.equal(selectBody(null), "");
});

test("decodeQuotedPrintable decodes =3D and soft line breaks", () => {
  assert.equal(decodeQuotedPrintable("a=3Db"), "a=b");
  assert.equal(decodeQuotedPrintable("line1=\r\nline2"), "line1line2");
  assert.equal(decodeQuotedPrintable("line1=\nline2"), "line1line2");
});

test("decodeQuotedPrintable reassembles multi-byte UTF-8", () => {
  // café — é is C3 A9 in UTF-8.
  assert.equal(decodeQuotedPrintable("caf=C3=A9"), "café");
});

test("decodeQuotedPrintable keeps a bare '=' that is not valid hex", () => {
  assert.equal(decodeQuotedPrintable("2=2 and =zz"), "2=2 and =zz");
});

test("decodeQuotedPrintable keeps a trailing '=' at end of input", () => {
  assert.equal(decodeQuotedPrintable("ends with="), "ends with=");
});

test("decodeBase64 decodes ignoring embedded whitespace", () => {
  const b64 = Buffer.from("Required: Rust.", "utf8").toString("base64");
  // inject whitespace
  const spaced = b64.slice(0, 4) + "\r\n" + b64.slice(4);
  assert.equal(decodeBase64(spaced), "Required: Rust.");
});

function buildMultipart(boundary, htmlBody, plainBody, cte) {
  const enc = cte === "base64"
    ? (s) => Buffer.from(s, "utf8").toString("base64")
    : (s) => s;
  return [
    "From: alerts@example.com",
    "To: user@example.org",
    `Content-Type: multipart/alternative; boundary="${boundary}"`,
    "",
    "preamble ignored",
    `--${boundary}`,
    "Content-Type: text/plain; charset=UTF-8",
    cte ? `Content-Transfer-Encoding: ${cte}` : "Content-Transfer-Encoding: 7bit",
    "",
    enc(plainBody),
    `--${boundary}`,
    "Content-Type: text/html; charset=UTF-8",
    cte ? `Content-Transfer-Encoding: ${cte}` : "Content-Transfer-Encoding: 7bit",
    "",
    enc(htmlBody),
    `--${boundary}--`,
    "",
  ].join(CRLF);
}

test("walks multipart/alternative and decodes parts", () => {
  const raw = buildMultipart("BND123", "<p>HTML body</p>", "plain body");
  const parts = parseEml(raw);
  const types = parts.map((p) => p.contentType).sort();
  assert.deepEqual(types, ["text/html", "text/plain"]);
});

test("selectBody prefers text/html over text/plain", () => {
  const raw = buildMultipart("BND123", "<p>HTML body</p>", "plain body");
  const body = selectBody(parseEml(raw));
  assert.equal(body, "<p>HTML body</p>");
});

test("selectBody falls back to text/plain when no html part", () => {
  const raw = [
    `Content-Type: multipart/alternative; boundary="B"`,
    "",
    "--B",
    "Content-Type: text/plain",
    "",
    "only plain here",
    "--B--",
    "",
  ].join(CRLF);
  assert.equal(selectBody(parseEml(raw)), "only plain here");
});

test("selectBody falls back to first part for exotic-only types", () => {
  const raw = [
    `Content-Type: multipart/alternative; boundary="B"`,
    "",
    "--B",
    "Content-Type: application/octet-stream",
    "",
    "blob",
    "--B--",
    "",
  ].join(CRLF);
  assert.equal(selectBody(parseEml(raw)), "blob");
});

test("decodes a base64-encoded multipart body", () => {
  const raw = buildMultipart("BND9", "<p>café ☕</p>", "ignored", "base64");
  assert.equal(selectBody(parseEml(raw)), "<p>café ☕</p>");
});

test("decodes a quoted-printable single-part message", () => {
  const raw = [
    "Content-Type: text/plain; charset=UTF-8",
    "Content-Transfer-Encoding: quoted-printable",
    "",
    "Tinker =26 Bell wants caf=C3=A9 skills",
  ].join(CRLF);
  assert.equal(selectBody(parseEml(raw)), "Tinker & Bell wants café skills");
});

test("single-part message with no blank line is tolerated", () => {
  const raw = "Content-Type: text/plain";
  const parts = parseEml(raw);
  assert.equal(parts.length, 1);
  assert.equal(parts[0].text, "");
});

test("header continuation lines are unfolded", () => {
  const raw = [
    "Content-Type: multipart/alternative;",
    '\tboundary="WRAP"',
    "",
    "--WRAP",
    "Content-Type: text/html",
    "",
    "<p>ok</p>",
    "--WRAP--",
    "",
  ].join(CRLF);
  assert.equal(selectBody(parseEml(raw)), "<p>ok</p>");
});

test("boundary may be unquoted (bare token branch)", () => {
  const raw = [
    "Content-Type: multipart/alternative; boundary=BARE99",
    "",
    "--BARE99",
    "Content-Type: text/html",
    "",
    "<p>bare boundary</p>",
    "--BARE99--",
    "",
  ].join(CRLF);
  assert.equal(selectBody(parseEml(raw)), "<p>bare boundary</p>");
});

test("multipart with no boundary param is treated as a leaf part", () => {
  // multipart/* type but boundaryOf returns null → falls through to leaf.
  const raw = ["Content-Type: multipart/alternative", "", "orphan body"].join(CRLF);
  const parts = parseEml(raw);
  assert.equal(parts.length, 1);
  assert.equal(parts[0].text, "orphan body");
});

test("empty Content-Type value falls back to text/plain", () => {
  const raw = ["Content-Type:", "", "x"].join(CRLF);
  assert.equal(parseEml(raw)[0].contentType, "text/plain");
});

test("base64 single-part decodes (decodeBody base64 arm)", () => {
  const b64 = Buffer.from("hello b64", "utf8").toString("base64");
  const raw = [
    "Content-Type: text/plain",
    "Content-Transfer-Encoding: base64",
    "",
    b64,
  ].join(CRLF);
  assert.equal(parseEml(raw)[0].text, "hello b64");
});

test("a closing-boundary-only segment is skipped", () => {
  // Boundary appears with nothing but the closing marker after the single part.
  const raw = [
    'Content-Type: multipart/alternative; boundary="C"',
    "",
    "--C",
    "Content-Type: text/plain",
    "",
    "only part",
    "--C--",
    "trailing epilogue ignored",
  ].join(CRLF);
  const parts = parseEml(raw);
  assert.equal(parts.length, 1);
  assert.equal(parts[0].text, "only part");
});

test("missing content-type defaults to text/plain", () => {
  const raw = ["X-Foo: bar", "", "bare body"].join(CRLF);
  const parts = parseEml(raw);
  assert.equal(parts[0].contentType, "text/plain");
  assert.equal(parts[0].text, "bare body");
});

test("a content-type with an empty type token falls back to text/plain", () => {
  // ';charset=utf-8' survives the header trim (non-empty, so passes `!ct`) but
  // split(';')[0].trim() === '' → the '|| text/plain' fallback arm in mimeType.
  const raw = ["Content-Type: ;charset=utf-8", "", "body"].join(CRLF);
  assert.equal(parseEml(raw)[0].contentType, "text/plain");
});

test("pathologically deep multipart nesting is bounded (no overflow)", () => {
  // Build 20 nested multipart/alternative wrappers; the depth guard (>16) stops
  // the recursion without throwing. The innermost leaf is therefore not reached.
  let inner = ["Content-Type: text/html", "", "<p>deep</p>"].join(CRLF);
  for (let i = 0; i < 20; i++) {
    const b = `LVL${i}`;
    inner = [
      `Content-Type: multipart/alternative; boundary="${b}"`,
      "",
      `--${b}`,
      inner,
      `--${b}--`,
      "",
    ].join(CRLF);
  }
  // Must not throw and must return an array (possibly empty due to the bound).
  const parts = parseEml(inner);
  assert.ok(Array.isArray(parts));
});
