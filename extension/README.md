# Job Hunter — "Clip this job" extension (MV3)

A minimal, **user-driven, compliant** Manifest V3 extension. It clips the single
job posting you are already viewing — on your click — and hands a Normalized Job
`.json` to the desktop app's existing import path.

## Compliance posture (non-negotiable)

User-driven capture **only**:

- **No automated login.** The extension never authenticates anywhere.
- **No navigation/automation.** It never drives the browser to other pages.
- **No background scraping.** Nothing runs on a timer or in the background.
- **No anti-bot evasion.** No fingerprint spoofing, no rate dodging.
- **No reading of tabs you did not explicitly clip.** Capture runs ONLY on the
  active tab, ONLY when you click "Clip this job".

These prohibitions are stated verbatim in `manifest.json`'s `description`.

## How it works

1. You open a job page yourself and click the extension's **Clip this job** button.
2. The popup programmatically injects `src/content.mjs` into **that tab only**
   (`chrome.scripting.executeScript`, gated by your click — never auto-injected).
3. The content script reads `document.documentElement.outerHTML` and passes the
   **string** to the pure `dom-extract` core in `packages/capture-core/`. No
   deterministic logic lives in the extension wiring.
4. The core returns a Normalized Job that validates against
   `doc/schemas/normalized-job.schema.json`.
5. The background service worker triggers a `.json` download
   (`clipped-job-<ts>.json`) via `chrome.downloads`. You import it through the
   desktop's existing import path (byte-compatible with `CoreJob::from_json`).

## Permissions (minimal)

`activeTab`, `scripting`, `downloads` — and nothing else. There are **no**
`host_permissions` and **no** auto-injected `content_scripts`. Injection is
always programmatic and gesture-driven.

## Handoff

The shipped handoff is the downloadable `.json` (DISCUSS-HANDOFF). The optional
localhost `POST` handoff is documented-as-deferred and is **not** built in this
item (it needs a security model before it can ship).
