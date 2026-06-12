# SPECIFICATION ONLY — NOT EXECUTABLE

> Item #3 — Applicant Advocate LLM layer. EARS requirements (Step 1) + Gherkin acceptance scenarios
> (Step 2) for ROADMAP item #3. Optional, feature-flagged, evidence-bounded rewrite/draft over a
> local Ollama instance or a user-supplied key. Redact before any call; never invent; cite evidence
> ids; fully disablable. Tests use a deterministic **stub** adapter — **NO live model in CI**.
> Authored by `lifecycle-orchestrator` from the authoritative LEAD ENGINEER plan (FOUNDRY_PLAN.md
> Item #3) and SUBJECT_MATTER_UNDERSTANDING.md (AI guardrails) + ARCHITECTURE.md (AI row).
>
> The `.feature`-style scenarios below are **specification only** and live under `doc/spec/`
> (DoD §2) — each maps to an executable Rust/TS test (L1–L5) carrying the same `R-ADV-*` id in a
> comment. The traceability table at the foot names the proving test for every requirement.

---

## Design invariants carried from the brief (apply to every requirement)

- **PII firewall (I-PII):** the bytes that leave the process to any model carry CV *evidence* only —
  never `Person` name/email/phone/linkedin/github. Redaction is *structural*: the outbound request
  type physically cannot hold `Person` fields.
- **Evidence-ID-bound (I2, §E):** the rewrite cites an evidence *id*; the EXISTING ledger `guard`
  (against the IMMUTABLE master CV, not the view) is the backstop. A provider that cites an id
  resolving NOWHERE is NAMED and BLOCKED — honesty over polish. This is an **EVIDENCE-ID guard,
  proven by construction** for the stub/CI path (the adversarial test blocks a dangling id). It is
  NOT a text-faithfulness guard: see **Residual risks (live adapters)** below — for a real model the
  current live adapters stamp the requested id back verbatim, so the id always resolves and the guard
  cannot catch a hallucinated *rewrite of the text* under a valid id. The stub/CI path used by every
  test IS fully guarded.
- **Fully disablable (I-FLAG):** `AdvocateConfig::default().enabled == false`. With the flag off the
  export path is byte-identical to the deterministic slice-1/2 path; `default` cargo features compile
  the trait + stub ONLY (no network dependency), so the `--workspace` CI gate carries no network code.
- **Honesty over silent fallback:** flag ON but provider unreachable → an EXPLICIT error surfaces; the
  app NEVER silently falls back to the deterministic text and pretends AI ran.

---

## Residual risks (live adapters)

The stub/CI path is **fully guarded**: every test runs the deterministic `StubProvider`, the
EVIDENCE-ID guard blocks a dangling/absent id by construction (the L4 adversarial test proves it),
and the outbound `RewriteRequest` type physically cannot carry `Person` PII (proven at L3). The
following residuals apply ONLY to the `live-http` adapters, which are **currently unwired dead code
behind `--features live-http`** (no command calls them; CI never compiles them). They MUST be closed
before the live adapters are activated:

- **R-ADV-RES-1 — text-faithfulness is not yet checked for live models.** The id-guard verifies the
  cited *evidence id* resolves; it does NOT verify the rewritten *text* is faithful to the evidence.
  For a real model a hallucinated rewrite under an otherwise-valid id would pass. A future slice must
  add a text-faithfulness check (e.g. the rewrite stays bounded to `evidence_text`) before wiring a
  live adapter to a command.
- **R-ADV-RES-2 — live providers stamp the requested id rather than parsing the model's citation.**
  Both `OllamaProvider` and `HttpKeyProvider` set `cited_evidence_id = req.evidence_id` verbatim, so
  the id ALWAYS resolves for a live model and the guard cannot distinguish honest from fabricated the
  way it does for the stub. A future slice must parse the model's OWN claimed citation into the
  adopt/guard branch so a model that cites a different id is blocked like the stub.
- **R-ADV-RES-3 — free-text PII in `evidence_text` is not scrubbed.** The PII firewall is
  STRUCTURAL: the `Person` block (name/contact) is blocked by construction because `RewriteRequest`
  has no `Person` field. But free-text PII a user pastes INTO a bullet `description` (the
  `evidence_text`) is the user's own content and is carried verbatim into the prompt — it is NOT
  scrubbed in this slice. Accepted risk; a future slice may add a free-text scrub/warn pass over
  `evidence_text`.

**TLS (closed):** the `HttpKeyProvider` BYO-key endpoint is rejected unless it is `https://`
(parse-don't-validate), and `live-http` pulls `ureq`'s `rustls` backend, so the bearer key + evidence
are never transmitted in cleartext. The `api_key` is redacted in the provider's `Debug` impl.

---

## EARS requirements (Step 1) — the R-ADV family

| ID | EARS statement |
|---|---|
| **R-ADV-1** | The advocate crate (`aa-advocate`) SHALL depend on `aa-core` ONLY (one-way crate graph, mirroring `cvimport`), and SHALL be a member of the workspace so it rides the `--workspace` gate. |
| **R-ADV-2** | The advocate crate SHALL expose `default = []` features compiling a provider trait + a deterministic `StubProvider` with **no network dependency compiled**; the live adapters SHALL sit behind a `live-http = ["dep:ureq"]` feature so the CI gate (`aa-desktop` → `aa-advocate` with DEFAULT features) compiles NO network code. |
| **R-ADV-3** | WHEN `redact(achievement, requirement)` is called, it SHALL produce a `RewriteRequest` whose serialized form carries the achievement's evidence id + description text + the requirement string + a `RewriteKind`, and SHALL physically be incapable of carrying any `Person` PII (the type has no `Person` field). |
| **R-ADV-4** | WHEN `build_prompt(req)` assembles the outbound prompt, it SHALL be the ONLY function that assembles outbound bytes, and SHALL contain ONLY a fixed template + the request's `evidence_text` + `requirement` (and nothing else — no PII, no master-CV fields). |
| **R-ADV-5** | An `AdvocateProvider::rewrite(req)` SHALL return a `RewriteResponse { rewritten_text, cited_evidence_id }`; the `StubProvider` SHALL return a deterministic canned response for the same input, and `StubProvider::fabricating()` SHALL return a `cited_evidence_id` ABSENT from any master CV (the adversarial fixture). |
| **R-ADV-6** | The serialized outbound `RewriteRequest` JSON SHALL have EXACTLY the keys `{evidence_id, evidence_text, requirement, kind}` — proving structurally that no master-CV/`Person` field can cross the boundary (the L3 redaction boundary). |
| **R-ADV-7** | WHEN `Session::export_application` runs with `advocate.enabled == true`, the session SHALL, for each selected CV bullet, build a `RewriteRequest` via `redact()` (pairing the top-matching must-have requirement), call `provider.rewrite()`, and set the bullet `description = resp.rewritten_text` ONLY when `resp.cited_evidence_id == a.id`; otherwise it SHALL adopt `a.id = resp.cited_evidence_id` (the possibly-fabricated id) so the EXISTING ledger guard can NAME and BLOCK it. |
| **R-ADV-8** | The EXISTING `guard(&cv_ledger(&view), cv)` (checking against the IMMUTABLE master `cv`, NOT the view) SHALL run AFTER the advocate rewrite; a fabricated/dangling cited id SHALL produce `CoreError::LedgerBlocked` → `CommandError::ExportBlocked` whose message NAMES the fabricated id. The same re-guard re-entry SHALL apply to cover-letter strength paragraphs. |
| **R-ADV-9** | WHEN the advocate flag is ON but the provider is unreachable, `rewrite()` SHALL return `AdvocateError::Unreachable` which SHALL surface as a `CommandError` (NO silent fallback to deterministic text — the user must know AI did not run). |
| **R-ADV-10** | `ExportResult` SHALL carry `ai_used: bool` (+ the provider `name` string) and each rewritten bullet SHALL be markable `rewritten: bool` for the UI badge; this provenance is **surface-only** (no SQLCipher persistence this slice). |
| **R-ADV-11** | The advocate SHALL default to **disabled**: `AdvocateConfig::default().enabled == false`, and WHEN disabled the export path SHALL be byte-identical to the deterministic path (no rewrite, `ai_used == false`). |
| **R-ADV-12** | `aa-core` SHALL expose a helper returning the top-matching must-have requirement string for a selected achievement id (reusing the existing `matches_must` logic); the session SHALL thread that requirement into the `RewriteRequest`. WHEN no must-have requirement matches, the helper SHALL deterministically return the joined must-have list (pinned by test), so every rewrite carries a non-empty requirement. |
| **R-ADV-13** | The React UI SHALL offer a clear opt-in toggle (OFF by default, `aria-label`ed) in the review step that calls `setAdvocateEnabled(enabled)`, and SHALL show an "AI was used" badge after export WHEN `exportApplication` returns `aiUsed == true`; error surfaces SHALL keep `role="alert"`. |

### Delegated implementation calls (recorded, per plan)

- **Requirement-pairing (R-ADV-12):** the `aa-core` helper is `tailor::requirement_for(cv, job, evidence_id) -> String`. It reuses `matches_must`; the deterministic no-match fallback is the **joined must-have list** (`must_have.join(", ")`), NOT a skip — pinned by an L1 test so every rewrite always carries a non-empty requirement.
- **Live adapters (R-ADV-2, behind `live-http`):** `OllamaProvider` POSTs `localhost:11434/api/generate`; `HttpKeyProvider` POSTs a generic endpoint with an `Authorization` header. Neither compiles under default features → NO network in CI by construction. They are NOT exercised by CI tests (live-model-free); their existence is a compile-gated, feature-flagged surface only.
- **Provenance transport (R-ADV-10):** `ExportResult` gains `ai_used: bool` + `provider: Option<String>`; the bullet-level `rewritten: bool` is surfaced to the UI via the export result's coverage-adjacent fields. Surface-only; no persistence.

---

## Gherkin acceptance scenarios (Step 2) — happy / unhappy / abuse

```gherkin
Feature: Applicant Advocate — evidence-bounded, feature-flagged LLM rewrite

  # ── PII firewall: redaction is structural (the headline guard) ──────────────
  @R-ADV-3 @R-ADV-6
  Scenario: The outbound request physically cannot carry Person PII
    Given an achievement and a requirement string
    When I redact(achievement, requirement) into a RewriteRequest and serialize it
    Then the JSON keys are EXACTLY {evidence_id, evidence_text, requirement, kind}
    And the serialized bytes contain NONE of the person's name/email/phone/linkedin/github

  @R-ADV-4
  Scenario: build_prompt assembles only evidence + requirement
    Given a RewriteRequest
    When I build_prompt(req)
    Then the prompt contains the evidence_text and the requirement
    And the prompt contains no other CV field

  # ── deterministic stub (no live model in CI) ────────────────────────────────
  @R-ADV-5
  Scenario: The stub returns a deterministic, evidence-citing response
    Given a StubProvider and a RewriteRequest for evidence id "exp_1_0_b0"
    When I rewrite(req)
    Then the response cites "exp_1_0_b0"
    And the same input yields a byte-identical response

  @R-ADV-5
  Scenario: The fabricating stub cites an id absent from the master CV
    Given StubProvider::fabricating() and a RewriteRequest
    When I rewrite(req)
    Then the response cites an id that resolves nowhere in the master CV

  # ── flag default OFF; disabled path is byte-identical ───────────────────────
  @R-ADV-11
  Scenario: The advocate is disabled by default
    Given AdvocateConfig::default()
    Then enabled is false

  @R-ADV-11
  Scenario: With the flag off, export is byte-identical to the deterministic path
    Given a session with advocate disabled
    When I export_application()
    Then the two PDFs equal the deterministic export and ai_used is false

  # ── the headline integration (system L4) ────────────────────────────────────
  @R-ADV-7 @R-ADV-10
  Scenario: Flag ON + honest stub exports two valid PDFs marked ai_used
    Given a session with advocate enabled and an honest StubProvider
    When I export_application()
    Then two valid PDFs are produced and ai_used is true

  # ── THE MANDATORY ADVERSARIAL NON-VACUOUS TEST ──────────────────────────────
  @R-ADV-7 @R-ADV-8
  Scenario: A fabricating provider's dangling cited id BLOCKS the export and is NAMED
    Given a session with advocate enabled and StubProvider::fabricating()
    When I export_application()
    Then the export returns Err(ExportBlocked) and the message NAMES the fabricated id
    # non-vacuous twin: the SAME journey with an honest stub PASSES (proves the block is real)

  @R-ADV-9
  Scenario: Flag ON but provider unreachable surfaces an explicit error
    Given a session with advocate enabled and an unreachable provider
    When I export_application()
    Then a CommandError surfaces (NO silent fallback to deterministic text)

  # ── UI opt-in + provenance badge ────────────────────────────────────────────
  @R-ADV-13
  Scenario: The opt-in toggle is off by default and drives setAdvocateEnabled
    Given the review step
    Then the advocate toggle is off
    When I turn it on
    Then setAdvocateEnabled(true) is called

  @R-ADV-13
  Scenario: The "AI was used" badge shows after an ai_used export
    Given an export that returns aiUsed true
    Then the "AI was used" badge is visible
```

---

## Traceability — requirement → proving test

| Requirement | Level | Proving test |
|---|---|---|
| R-ADV-1 | build | workspace compiles `aa-advocate`; `aa-core`-only dep graph |
| R-ADV-2 | build | `cargo test --workspace` compiles default (no `ureq`); `live-http` gated |
| R-ADV-3 | L1 | `redact_strips_all_person_pii` (advocate src `#[cfg(test)]`) |
| R-ADV-4 | L1 | `build_prompt_contains_only_evidence_and_requirement` |
| R-ADV-5 | L1 | `stub_returns_deterministic_response`, `stub_fabricating_cites_absent_id`, `rewrite_response_carries_cited_evidence_id` |
| R-ADV-6 | L3 | `outbound_payload_has_no_master_cv_fields` (`tests/redaction_boundary_l3.rs`) |
| R-ADV-7 | L4 | `rewrite_enabled_clean_stub_exports_two_pdfs` (`tests/advocate_l4.rs`) |
| R-ADV-8 | L4 | `adversarial_stub_fabricates_dangling_id_blocks_export` + honest-twin (NON-VACUOUS) |
| R-ADV-9 | L1/L4 | `unreachable_provider_surfaces_error` |
| R-ADV-10 | L4 | `rewrite_enabled_clean_stub_exports_two_pdfs` asserts `ai_used == true` |
| R-ADV-11 | L2/L4 | `config_disabled_is_default`, `flag_off_is_byte_identical_to_deterministic` |
| R-ADV-12 | L1 | `requirement_for_returns_top_match`, `requirement_for_falls_back_to_joined_must` |
| R-ADV-13 | UI | `App.test.tsx` toggle-off-by-default / calls-setAdvocateEnabled / badge-shows |
| (perf) | L5 | `story_advocate_rewrite_perf_delta_gated` (`tests/story_l5.rs`, baseline `doc/perf/desktop-advocate-story-baseline.txt`) |
