//! C2 — Normalize + alias map (§A). The matching primitive's foundation.
//!
//! Normalization = lowercase + a small, extensible alias map seeded in core.
//! Aliases declared on `skill.aliases[]` participate too (wired in `match` via
//! an extended alias set). Tokenization splits on whitespace and punctuation so a
//! requirement phrase yields comparable tokens.

use std::collections::HashMap;

/// The seed alias map (§A). Keys and values are already lowercased. Multi-word
/// expansions (e.g. `ci/cd -> continuous integration`) are matched as a whole
/// normalized token AND contribute their expansion tokens.
pub fn seed_aliases() -> HashMap<String, String> {
    [
        ("js", "javascript"),
        ("ts", "typescript"),
        ("k8s", "kubernetes"),
        ("ci/cd", "continuous integration"),
        ("golang", "go"),
        ("postgres", "postgresql"),
        ("gcp", "google cloud"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// Lowercase a single token. Control characters are stripped; surrounding
/// whitespace trimmed. Hostile input (10k chars, control chars) must not panic.
pub fn normalize_token(tok: &str) -> String {
    tok.trim()
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .to_lowercase()
}

/// Split a free-text phrase into normalized tokens. Splits on whitespace and
/// common punctuation, but preserves `/` so `ci/cd` survives as one token (it is
/// a known alias key). Empty tokens are dropped.
pub fn tokenize(phrase: &str) -> Vec<String> {
    phrase
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '.' | '(' | ')' | ':'))
        .map(normalize_token)
        .filter(|t| !t.is_empty())
        .collect()
}

/// Expand a normalized token through the alias map: returns the token plus, if it
/// is an alias key, the alias target's tokens. e.g. `js -> {js, javascript}`,
/// `ci/cd -> {ci/cd, continuous, integration}`.
pub fn expand(token: &str, aliases: &HashMap<String, String>) -> Vec<String> {
    let mut out = vec![token.to_string()];
    if let Some(target) = aliases.get(token) {
        for t in tokenize(target) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
    out
}

/// The full normalized-token set for a phrase, alias-expanded — the comparable
/// form used by the matching primitive.
pub fn normalized_set(phrase: &str, aliases: &HashMap<String, String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tok in tokenize(phrase) {
        for e in expand(&tok, aliases) {
            if !out.contains(&e) {
                out.push(e);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_token_normalizes_to_empty() {
        assert_eq!(normalize_token(""), "");
        assert_eq!(normalize_token("   "), "");
    }

    #[test]
    fn case_folds() {
        assert_eq!(normalize_token("JS"), "js");
        assert_eq!(normalize_token("Js"), "js");
        assert_eq!(normalize_token("javaScript"), "javascript");
    }

    #[test]
    fn js_aliases_to_javascript() {
        let a = seed_aliases();
        let set = normalized_set("JS", &a);
        assert!(set.contains(&"js".to_string()));
        assert!(set.contains(&"javascript".to_string()));
    }

    #[test]
    fn cicd_expands_to_continuous_integration() {
        let a = seed_aliases();
        let set = normalized_set("CI/CD", &a);
        assert!(set.contains(&"ci/cd".to_string()));
        assert!(set.contains(&"continuous".to_string()));
        assert!(set.contains(&"integration".to_string()));
    }

    #[test]
    fn unicode_preserved_and_lowercased() {
        assert_eq!(normalize_token("Café"), "café");
        let a = seed_aliases();
        let set = normalized_set("golang", &a);
        assert!(set.contains(&"go".to_string()));
    }

    #[test]
    fn hostile_long_token_and_control_chars_no_panic() {
        let long = "x".repeat(10_000);
        assert_eq!(normalize_token(&long).len(), 10_000);
        let ctrl = "a\u{0007}b\u{0000}c".to_string();
        assert_eq!(normalize_token(&ctrl), "abc");
    }

    #[test]
    fn tokenize_splits_on_punctuation_keeps_slash() {
        let toks = tokenize("Strong TypeScript, Python; CI/CD.");
        assert!(toks.contains(&"strong".to_string()));
        assert!(toks.contains(&"typescript".to_string()));
        assert!(toks.contains(&"python".to_string()));
        assert!(toks.contains(&"ci/cd".to_string()));
    }

    #[test]
    fn expand_unknown_token_returns_itself() {
        let a = seed_aliases();
        assert_eq!(expand("rust", &a), vec!["rust".to_string()]);
    }

    #[test]
    fn normalized_set_dedups() {
        let a = seed_aliases();
        let set = normalized_set("go go golang", &a);
        assert_eq!(set.iter().filter(|t| *t == "go").count(), 1);
    }
}
