// L1 unit — normalized-job.mjs toJson strict-shape serialization, incl. the
// optional-field branches the cores do not exercise on their own.
import { test } from "node:test";
import assert from "node:assert/strict";
import { toJson } from "../src/normalized-job.mjs";

test("omits all empty optionals (minimal strict shape)", () => {
  const json = toJson({
    title: "T",
    company: "C",
    location: "",
    responsibilities: [],
    requirements: { mustHave: [], niceToHave: [] },
    keywords: [],
  });
  assert.deepEqual(JSON.parse(json), {
    title: "T",
    company: "C",
    requirements: { mustHave: [], niceToHave: [] },
  });
});

test("emits populated optionals (location/responsibilities/keywords)", () => {
  const json = toJson({
    title: "T",
    company: "C",
    location: "Remote (AU)",
    responsibilities: ["ship", "operate"],
    requirements: { mustHave: ["Rust"], niceToHave: ["Typst"] },
    keywords: ["platform", "fleet"],
  });
  assert.deepEqual(JSON.parse(json), {
    title: "T",
    company: "C",
    location: "Remote (AU)",
    responsibilities: ["ship", "operate"],
    requirements: { mustHave: ["Rust"], niceToHave: ["Typst"] },
    keywords: ["platform", "fleet"],
  });
});

test("toJson works without optional keys present at all", () => {
  const json = toJson({
    title: "T",
    company: "C",
    requirements: { mustHave: [], niceToHave: [] },
  });
  assert.equal("location" in JSON.parse(json), false);
});
