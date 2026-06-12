// L1 unit — html-text.mjs. Tolerant HTML→text reduction; entity decode; hostile.
import { test } from "node:test";
import assert from "node:assert/strict";
import { htmlToText } from "../src/html-text.mjs";

test("empty / non-string input returns empty string", () => {
  assert.equal(htmlToText(""), "");
  // @ts-expect-error intentional wrong type
  assert.equal(htmlToText(undefined), "");
});

test("strips tags to spaces and collapses whitespace", () => {
  assert.equal(htmlToText("<p>Hello</p>\n\n<p>World</p>"), "Hello World");
});

test("removes <script> blocks entirely (cue text inside must NOT survive)", () => {
  const html = "<div>Required: A.</div><script>Required: HIDDEN.</script>";
  const out = htmlToText(html);
  assert.ok(out.includes("Required: A."));
  assert.ok(!out.includes("HIDDEN"));
});

test("removes <style> blocks entirely", () => {
  const out = htmlToText("<style>.x{content:'Required: Z'}</style><p>Visible</p>");
  assert.equal(out, "Visible");
  assert.ok(!out.includes("Required: Z"));
});

test("decodes named entities", () => {
  assert.equal(
    htmlToText("<p>Tinker &amp; Bell &lt;Studios&gt; &quot;ok&quot; &apos;y&apos;</p>"),
    `Tinker & Bell <Studios> "ok" 'y'`,
  );
});

test("decodes &nbsp; to a space and collapses", () => {
  assert.equal(htmlToText("a&nbsp;&nbsp;b"), "a b");
});

test("decodes decimal and hex numeric entities", () => {
  assert.equal(htmlToText("&#39;&#x41;&#X42;"), "'AB");
});

test("unknown / malformed entities are preserved verbatim", () => {
  assert.equal(htmlToText("&notreal; &#; &#x;"), "&notreal; &#; &#x;");
});

test("out-of-range numeric entity is preserved", () => {
  assert.equal(htmlToText("&#1114112;"), "&#1114112;");
});

test("a decimal entity whose digits contain a–f is preserved verbatim", () => {
  // "&#FF;" matches the entity regex (F is a hex char) but is NOT valid decimal.
  assert.equal(htmlToText("&#FF;"), "&#FF;");
});

test("an out-of-range HEX entity is preserved verbatim", () => {
  // 0x110000 = 1114112 > 0x10FFFF.
  assert.equal(htmlToText("&#x110000;"), "&#x110000;");
});

test("strips HTML comments", () => {
  assert.equal(htmlToText("<!-- secret -->kept"), "kept");
});

test("survives unclosed tags without throwing (hostile)", () => {
  const out = htmlToText("<div><span>open <b>bold and a dangling <");
  assert.equal(out, "open bold and a dangling");
});

test("survives an unclosed <script> at EOF (consumes to end)", () => {
  const out = htmlToText("visible<script>never ends");
  assert.equal(out, "visible");
});

test("survives an unclosed comment at EOF", () => {
  assert.equal(htmlToText("ok<!-- runs off the end"), "ok");
});

test("handles a 1 MB input within bound (no throw)", () => {
  const big = "<p>Required: X.</p>".repeat(60000); // ~1 MB
  const out = htmlToText(big);
  assert.ok(out.includes("Required: X."));
});
