# PR Review — item #3 Applicant Advocate LLM layer (`item-3-advocate-llm`)

**Range:** `main..item-3-advocate-llm` · **Gate:** always-on adversarial review (governance `pr-approval`)

## VERDICT: PASS (after one NEEDS_REVISION round)

Synthesised from six adversarial reviewer roles + SENTINEL's three-lens security-gate, each prompted
to refute the change. Round 1 returned NEEDS_REVISION (2×HIGH, 2×MEDIUM, 2×LOW); all were remediated;
round 2 re-review of the gating roles returned PASS.

## Roles run

| Lens | Round 1 | Round 2 |
|---|---|---|
| CORRECTNESS | NEEDS_REVISION | **PASS** |
| SECURITY + PROMPT-INJECTION | NEEDS_REVISION | **PASS** |
| ARCHITECTURE + REGRESSION | PASS | — |
| SENTINEL pii-audit | PASS | — |
| SENTINEL secret-scan | PASS (1 MEDIUM advisory → fixed) | — |
| SENTINEL dependency-audit | REVIEW (1 HIGH no-TLS) | **resolved** |

## Findings and disposition

| # | Sev | Finding | Disposition |
|---|---|---|---|
| 1 | HIGH | `HttpKeyProvider` plaintext-HTTP-only (ureq dropped TLS); cleartext bearer key | FIXED — `ureq/rustls` enabled under `live-http`; `https://`-only scheme guard (proven against 20 attack vectors, fails closed); Ollama loopback-only |
| 2 | HIGH | ledger guard checks cited *id*, not rewritten *text*; live providers stamp the requested id → not a hallucination guard for live models | DOCUMENTED & DEFERRED — spec/ARCHITECTURE/live.rs now scope it as an EVIDENCE-ID guard (stub/CI path fully guarded, proven non-vacuously); R-ADV-RES-1/RES-2 must close before adapters are wired |
| 3 | MEDIUM | `#[derive(Debug)]` on `HttpKeyProvider` leaks `api_key` | FIXED — manual redacting `Debug` (`api_key` → `"<redacted>"`); test asserts redaction |
| 4 | MEDIUM | free-text PII inside `evidence_text` not scrubbed | DOCUMENTED — structural firewall blocks the `Person` block by type; free-text is user content (R-ADV-RES-3) |
| 5 | LOW | cover-letter strength rewritten twice | FIXED — `build_cover_letter` runs before the bullet-rewrite loop; rewritten exactly once (new test) |

## What IS guaranteed this slice (verified non-vacuously)

- The LLM layer is OPTIONAL and OFF by default; the deterministic product ships fully without it
  (`flag_off_is_byte_identical_to_deterministic`).
- The EVIDENCE-ID is guarded by construction: a rewrite citing a dangling/absent id is BLOCKED at
  export against the IMMUTABLE master CV and NAMED — for CV bullets AND cover-letter strengths
  (`adversarial_stub_fabricates_dangling_id_blocks_export` + honest twin;
  `fabricated_cover_letter_strength_id_blocks_export`).
- Redaction is STRUCTURAL by type: the outbound `RewriteRequest` has no `Person` field
  (`outbound_payload_has_no_master_cv_fields`, exact-key assertion).
- No live model in CI by construction: `ureq` is absent from the default/desktop dependency tree;
  the live adapters are uncompiled, unwired dead code behind `live-http`.
- Honesty over fallback: flag-ON with an unreachable provider surfaces an explicit error, never a
  silent deterministic fallback.

## What was NOT reviewed

- Live-model behaviour against a real Ollama/BYO-key endpoint (no live model in CI by design; the
  adapters are unwired this slice).
- Tauri runtime IPC wiring (no `#[command]`/`invoke_handler` added this slice; the command surface is
  exercised via the Rust `Session` and the React mock, per R-D3).
- The `ui` CI job (non-blocking infra, issue #2 — runners can't reach any npm registry); UI tests
  pass locally 14/14.

## KAIZEN tripwire (recorded by two reviewers)

This verdict **flips to BLOCK** if a future diff wires a live adapter to a command (removes the
`live-http` gate, or constructs `OllamaProvider`/`HttpKeyProvider` from `Session`) WITHOUT first
landing the text-faithfulness check (R-ADV-RES-1) and parsing the model's own citation into the
adopt/guard branch (R-ADV-RES-2). The disclosure is the gate; activation-without-closure is the
regression to catch.
