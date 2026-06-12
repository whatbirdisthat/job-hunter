// L4 system — content/popup/background wiring smoke. Proves the wiring calls the
// bundled core correctly WITHOUT a browser: the modules import cleanly (browser
// globals guarded), the content core yields a valid job over a fixture, and the
// popup/background helpers behave with an injected fake `chrome`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import {
  clipFromOuterHtml,
  captureActiveDocument,
  runCapture,
} from "../src/content.mjs";
import { wirePopup } from "../src/popup.mjs";
import {
  downloadFilename,
  jobDataUrl,
  handoffDownload,
  registerHandoff,
} from "../src/background.mjs";
import { validateJob } from "../../packages/capture-core/src/validate-job.mjs";
import { toJson } from "../../packages/capture-core/src/normalized-job.mjs";

const fx = (name) =>
  readFileSync(
    fileURLToPath(
      new URL(`../../packages/capture-core/fixtures/${name}`, import.meta.url),
    ),
    "utf8",
  );

test("content core over linkedin-job.html yields a valid job (no DOM)", () => {
  const job = clipFromOuterHtml(fx("linkedin-job.html"));
  assert.equal(job.title, "Staff Platform Engineer");
  assert.equal(job.company, "Northwind Robotics");
  assert.deepEqual(validateJob(JSON.parse(toJson(job))), []);
});

test("captureActiveDocument reads document.documentElement.outerHTML", () => {
  // Inject a minimal fake document; proves the wrapper delegates to the core.
  const realDoc = globalThis.document;
  globalThis.document = {
    documentElement: { outerHTML: fx("seek-job.html") },
  };
  try {
    const json = captureActiveDocument();
    const round = JSON.parse(json);
    assert.equal(round.company, "Tinker & Bell Studios");
    assert.deepEqual(validateJob(round), []);
  } finally {
    if (realDoc === undefined) delete globalThis.document;
    else globalThis.document = realDoc;
  }
});

test("runCapture posts a clipped-job message with the job JSON", () => {
  const sent = [];
  const chromeApi = { runtime: { sendMessage: (m) => sent.push(m) } };
  runCapture(chromeApi, () => '{"title":"T","company":"C"}');
  assert.equal(sent.length, 1);
  assert.equal(sent[0].type, "clipped-job");
  assert.equal(JSON.parse(sent[0].json).company, "C");
});

test("runCapture posts a clip-error message when capture throws", () => {
  const sent = [];
  const chromeApi = { runtime: { sendMessage: (m) => sent.push(m) } };
  runCapture(chromeApi, () => {
    throw new Error("boom");
  });
  assert.equal(sent.length, 1);
  assert.equal(sent[0].type, "clip-error");
  assert.ok(sent[0].message.includes("boom"));
});

test("runCapture default capture delegates to captureActiveDocument", () => {
  const realDoc = globalThis.document;
  globalThis.document = {
    documentElement: { outerHTML: fx("linkedin-job.html") },
  };
  const sent = [];
  try {
    runCapture({ runtime: { sendMessage: (m) => sent.push(m) } });
    assert.equal(sent[0].type, "clipped-job");
    assert.equal(JSON.parse(sent[0].json).company, "Northwind Robotics");
  } finally {
    if (realDoc === undefined) delete globalThis.document;
    else globalThis.document = realDoc;
  }
});

test("popup button injects content.mjs into the ACTIVE tab only, on click", async () => {
  const calls = { query: [], executeScript: [] };
  const fakeChrome = {
    tabs: {
      query: async (q) => {
        calls.query.push(q);
        return [{ id: 99 }];
      },
    },
    scripting: {
      executeScript: async (opts) => {
        calls.executeScript.push(opts);
      },
    },
  };
  // minimal fake DOM with a clickable button and a status element
  let clickHandler;
  const button = {
    addEventListener: (_ev, fn) => {
      clickHandler = fn;
    },
  };
  const status = { textContent: "" };
  const doc = {
    getElementById: (id) => (id === "clip" ? button : status),
  };

  wirePopup(fakeChrome, doc);
  await clickHandler(); // simulate the user gesture

  // queried the active tab in the current window…
  assert.deepEqual(calls.query, [{ active: true, currentWindow: true }]);
  // …and injected ONLY into that tab, with the content file (programmatic).
  assert.equal(calls.executeScript.length, 1);
  assert.deepEqual(calls.executeScript[0].target, { tabId: 99 });
  assert.deepEqual(calls.executeScript[0].files, ["src/content.mjs"]);
});

test("popup surfaces an error when injection fails", async () => {
  const fakeChrome = {
    tabs: { query: async () => [{ id: 1 }] },
    scripting: {
      executeScript: async () => {
        throw new Error("denied");
      },
    },
  };
  let clickHandler;
  const button = { addEventListener: (_e, fn) => (clickHandler = fn) };
  const status = { textContent: "" };
  const doc = { getElementById: (id) => (id === "clip" ? button : status) };
  wirePopup(fakeChrome, doc);
  await clickHandler();
  assert.ok(status.textContent.includes("Could not clip"));
});

test("wirePopup is a no-op when the button is absent", () => {
  // getElementById returns null for #clip → early return, no throw.
  const doc = { getElementById: () => null };
  assert.doesNotThrow(() => wirePopup({}, doc));
});

test("background filename + data URL helpers are deterministic", () => {
  assert.equal(downloadFilename(1700000000000), "clipped-job-1700000000000.json");
  const url = jobDataUrl('{"a":1}');
  assert.ok(url.startsWith("data:application/json;charset=utf-8,"));
  assert.equal(decodeURIComponent(url.split(",")[1]), '{"a":1}');
});

test("handoffDownload calls chrome.downloads.download with the JSON + filename", () => {
  const calls = [];
  const fakeChrome = {
    downloads: { download: (opts) => calls.push(opts) },
  };
  const json = toJson(clipFromOuterHtml(fx("linkedin-job.html")));
  handoffDownload(fakeChrome, json, 1234);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].filename, "clipped-job-1234.json");
  assert.equal(calls[0].saveAs, true);
  assert.equal(decodeURIComponent(calls[0].url.split(",")[1]), json);
});

test("registerHandoff uses the default Date.now clock when none is injected", () => {
  const downloads = [];
  let listener;
  const fakeChrome = {
    runtime: { onMessage: { addListener: (fn) => (listener = fn) } },
    downloads: { download: (o) => downloads.push(o) },
  };
  const before = Date.now();
  registerHandoff(fakeChrome); // default now = () => Date.now()
  listener({ type: "clipped-job", json: '{"title":"T"}' });
  assert.equal(downloads.length, 1);
  const ts = Number(downloads[0].filename.match(/clipped-job-(\d+)\.json/)[1]);
  assert.ok(ts >= before, "filename timestamp came from Date.now()");
});

test("registerHandoff downloads on a clipped-job message and ignores others", () => {
  const downloads = [];
  let listener;
  const fakeChrome = {
    runtime: { onMessage: { addListener: (fn) => (listener = fn) } },
    downloads: { download: (o) => downloads.push(o) },
  };
  registerHandoff(fakeChrome, () => 555);
  listener({ type: "clipped-job", json: '{"title":"T"}' });
  listener({ type: "clip-error", message: "x" }); // ignored
  listener(null); // ignored, no throw
  listener({ type: "clipped-job" }); // no json → ignored
  assert.equal(downloads.length, 1);
  assert.equal(downloads[0].filename, "clipped-job-555.json");
});
