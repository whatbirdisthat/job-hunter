//! Live HTTP adapters — `--features live-http` ONLY (never compiled by the `--workspace`
//! CI gate, which builds `aa-advocate` with DEFAULT features). Two providers:
//!
//!   * [`OllamaProvider`] — POSTs `http://localhost:11434/api/generate` (a local model).
//!   * [`HttpKeyProvider`] — POSTs a user-supplied endpoint with an `Authorization` header
//!     (a bring-your-own-key cloud model).
//!
//! Both send ONLY [`build_prompt`](crate::build_prompt) output (template + evidence +
//! requirement — no PII) and cite back the request's evidence id. They are a
//! feature-gated surface; CI proves the contract with the deterministic stub instead of
//! a live model. An unreachable endpoint yields [`AdvocateError::Unreachable`] — never a
//! silent fallback (R-ADV-9).
//!
//! # CITATION / FAITHFULNESS LIMITATION (read before wiring these to a command)
//!
//! These adapters are currently **unwired dead code behind `live-http`** — no command
//! calls them; CI exercises the deterministic [`StubProvider`](crate::StubProvider)
//! instead. Before activating them, understand exactly what the downstream ledger guard
//! does and does NOT protect against:
//!
//! Both providers hardcode `cited_evidence_id = req.evidence_id.clone()`. The export-time
//! ledger guard is therefore an **EVIDENCE-ID guard, NOT a text-faithfulness / hallucination
//! guard.** For the stub adversarial path the guard is genuinely protective — a dangling /
//! fabricated id is NAMED and BLOCKED (proven by the L4 adversarial + non-vacuous twin). But
//! for a REAL model the requested id ALWAYS resolves (it is stamped back verbatim), so the
//! guard CANNOT catch a model that hallucinates a *rewrite of the text* under an otherwise
//! valid id. A future slice MUST add, before these adapters are wired to a command, one or
//! both of:
//!
//!   * (a) parsing the model's OWN claimed citation out of its response into the
//!     adopt/guard branch (so a model that cites a different id is blocked like the stub), and/or
//!   * (b) a text-faithfulness check (e.g. the rewrite stays bounded to the evidence_text).
//!
//! See doc/spec/item-3-advocate-llm.md "Residual risks (live adapters)" — R-ADV-RES-1 /
//! R-ADV-RES-2.

use crate::{build_prompt, AdvocateError, AdvocateProvider, RewriteRequest, RewriteResponse};

/// Local Ollama adapter. Targets `http://localhost:11434/api/generate` by default.
///
/// The endpoint is hardcoded to loopback (`localhost`) so cleartext `http://` is acceptable:
/// nothing leaves the machine. Unlike [`HttpKeyProvider`] there is no bearer key, so there is
/// no secret to protect on the wire.
///
/// NOTE (faithfulness): cites back `req.evidence_id` verbatim — see the module-level
/// CITATION / FAITHFULNESS LIMITATION. The id-guard does not catch a hallucinated rewrite.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    model: String,
}

impl OllamaProvider {
    pub fn new(model: impl Into<String>) -> Self {
        OllamaProvider {
            model: model.into(),
        }
    }

    /// The hardcoded loopback endpoint. Kept private + fixed so this provider can NEVER be
    /// pointed at a remote host (which would need TLS + the BYO-key path instead).
    fn endpoint() -> &'static str {
        "http://localhost:11434/api/generate"
    }
}

impl AdvocateProvider for OllamaProvider {
    fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError> {
        let prompt = build_prompt(req);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
        });
        let mut resp = ureq::post(Self::endpoint())
            .send_json(&body)
            .map_err(|e| AdvocateError::Unreachable(e.to_string()))?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| AdvocateError::Malformed(e.to_string()))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AdvocateError::Malformed(e.to_string()))?;
        let rewritten_text = parsed
            .get("response")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdvocateError::Malformed("missing `response` field".into()))?
            .trim()
            .to_string();
        // EVIDENCE-ID guard, NOT a faithfulness guard: provenance stays bound to the
        // requested evidence id (stamped back verbatim). The ledger guard backstops the id;
        // it does NOT verify the rewritten TEXT is faithful to the evidence. See the
        // module-level CITATION / FAITHFULNESS LIMITATION (R-ADV-RES-1 / R-ADV-RES-2).
        Ok(RewriteResponse {
            rewritten_text,
            cited_evidence_id: req.evidence_id.clone(),
        })
    }

    fn name(&self) -> &'static str {
        "ollama"
    }
}

/// Bring-your-own-key cloud adapter: POST a generic endpoint with an `Authorization`
/// header. The endpoint + key + model are user-supplied; the payload is the same
/// PII-free prompt.
///
/// SECURITY: the endpoint MUST be `https://` — rejected by construction otherwise
/// ([`AdvocateError::InsecureEndpoint`]), so the bearer key + evidence can never be sent in
/// cleartext (parse-don't-validate; TLS via ureq's `rustls` backend under `live-http`). The
/// `api_key` is redacted in the manual [`Debug`] impl so it cannot leak through `{:?}` /
/// error context.
///
/// NOTE (faithfulness): cites back `req.evidence_id` verbatim — see the module-level
/// CITATION / FAITHFULNESS LIMITATION. The id-guard does not catch a hallucinated rewrite.
#[derive(Clone)]
pub struct HttpKeyProvider {
    endpoint: String,
    api_key: String,
    model: String,
}

// Manual Debug: NEVER print `api_key` (a secret). A derived Debug would leak the key through
// any `{:?}` / panic message / error context. Endpoint + model stay visible for diagnostics.
impl std::fmt::Debug for HttpKeyProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpKeyProvider")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .finish()
    }
}

impl HttpKeyProvider {
    /// Construct a BYO-key provider. Returns [`AdvocateError::InsecureEndpoint`] when the
    /// endpoint is not `https://` (parse-don't-validate): the bearer key + evidence must never
    /// be transmitted in cleartext. The scheme guard fires HERE, before any HTTP call.
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, AdvocateError> {
        let endpoint = endpoint.into();
        if !is_https(&endpoint) {
            return Err(AdvocateError::InsecureEndpoint(endpoint));
        }
        Ok(HttpKeyProvider {
            endpoint,
            api_key: api_key.into(),
            model: model.into(),
        })
    }
}

/// True iff `endpoint` is an `https://` URL (ASCII-case-insensitive scheme). A scheme other
/// than https — including plain `http://` — is rejected so secrets never go out in cleartext.
fn is_https(endpoint: &str) -> bool {
    endpoint
        .split_once("://")
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
}

impl AdvocateProvider for HttpKeyProvider {
    fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteResponse, AdvocateError> {
        // Defence in depth: even though `new` enforces https, re-check before sending so the
        // key can never leave over a downgraded scheme (the guard fires before any HTTP call).
        if !is_https(&self.endpoint) {
            return Err(AdvocateError::InsecureEndpoint(self.endpoint.clone()));
        }
        let prompt = build_prompt(req);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
        });
        let mut resp = ureq::post(&self.endpoint)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(&body)
            .map_err(|e| AdvocateError::Unreachable(e.to_string()))?;
        let text = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| AdvocateError::Malformed(e.to_string()))?;
        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| AdvocateError::Malformed(e.to_string()))?;
        // accept either {"response": ...} (Ollama-compatible) or OpenAI-ish choices[0].text
        let rewritten_text = parsed
            .get("response")
            .and_then(|v| v.as_str())
            .or_else(|| {
                parsed
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("text"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| AdvocateError::Malformed("no response/choices text".into()))?
            .trim()
            .to_string();
        // EVIDENCE-ID guard, NOT a faithfulness guard: see the module-level CITATION /
        // FAITHFULNESS LIMITATION. We stamp the requested id back rather than parsing the
        // model's own citation (R-ADV-RES-2); a hallucinated rewrite under a valid id is NOT
        // caught here (R-ADV-RES-1).
        Ok(RewriteResponse {
            rewritten_text,
            cited_evidence_id: req.evidence_id.clone(),
        })
    }

    fn name(&self) -> &'static str {
        "http-key"
    }
}

#[cfg(test)]
mod tests {
    //! live-http unit tests. These exercise the parse-don't-validate scheme guard and the
    //! secret-redacting Debug impl WITHOUT any live network — the guard fires before any HTTP
    //! call, so no socket is opened. (P-COV-3 feature-gated class — see doc/COVERAGE.md.)
    use super::*;

    /// FIX 1 — a non-`https://` remote endpoint is rejected by construction.
    #[test]
    fn http_key_provider_rejects_insecure_endpoint() {
        let err = HttpKeyProvider::new("http://api.example.com/v1", "sk-secret", "gpt")
            .expect_err("an http:// endpoint must be rejected");
        assert_eq!(
            err,
            AdvocateError::InsecureEndpoint("http://api.example.com/v1".to_string())
        );
    }

    /// FIX 1 — an `https://` endpoint is accepted (scheme is case-insensitive).
    #[test]
    fn http_key_provider_accepts_https_endpoint() {
        let p = HttpKeyProvider::new("HTTPS://api.example.com/v1", "sk-secret", "gpt")
            .expect("an https:// endpoint must be accepted");
        assert_eq!(p.name(), "http-key");
    }

    /// FIX 1 — a scheme-less / garbage endpoint is rejected (no `://` → not https).
    #[test]
    fn http_key_provider_rejects_schemeless_endpoint() {
        let err = HttpKeyProvider::new("api.example.com", "sk-secret", "gpt")
            .expect_err("a scheme-less endpoint must be rejected");
        assert_eq!(
            err,
            AdvocateError::InsecureEndpoint("api.example.com".to_string())
        );
    }

    /// FIX 2 — the manual Debug impl redacts the api_key (and keeps endpoint + model).
    #[test]
    fn http_key_provider_debug_redacts_api_key() {
        let p = HttpKeyProvider::new("https://api.example.com", "sk-super-secret", "gpt")
            .expect("https accepted");
        let dbg = format!("{p:?}");
        assert!(
            !dbg.contains("sk-super-secret"),
            "api_key must NOT appear: {dbg}"
        );
        assert!(
            dbg.contains("<redacted>"),
            "api_key must show as <redacted>: {dbg}"
        );
        assert!(
            dbg.contains("api.example.com"),
            "endpoint stays visible: {dbg}"
        );
        assert!(dbg.contains("gpt"), "model stays visible: {dbg}");
    }

    /// The Ollama provider names itself + targets loopback only.
    #[test]
    fn ollama_provider_names_itself_and_is_loopback() {
        let p = OllamaProvider::new("llama3");
        assert_eq!(p.name(), "ollama");
        assert!(OllamaProvider::endpoint().starts_with("http://localhost"));
        // Debug does not panic and exposes the model for diagnostics.
        assert!(format!("{p:?}").contains("llama3"));
    }
}
