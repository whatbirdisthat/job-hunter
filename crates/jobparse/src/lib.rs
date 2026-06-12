//! aa-jobparse — JD text → Normalized Job JSON (§F).
//!
//! Deterministic, rule-based. Depends on `serde` only; NEVER on `aa-core` (the
//! binding dependency rule). Its sole output is a Normalized Job that validates
//! against `doc/schemas/normalized-job.schema.json` (R-D1) — the data-not-code
//! seam. Core owns a mirror type validated against the same schema.
//!
//! §F classification cues:
//!   must = required | must have | essential | you will need | minimum | mandatory
//!   nice = preferred | desirable | bonus | nice to have | advantageous | ideally
//!   An UNMARKED requirement defaults to nice-to-have.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedJob {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub company: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub location: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub responsibilities: Vec<String>,
    pub requirements: Requirements,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Requirements {
    #[serde(rename = "mustHave", default)]
    pub must_have: Vec<String>,
    #[serde(rename = "niceToHave", default)]
    pub nice_to_have: Vec<String>,
}

const MUST_CUES: &[&str] = &[
    "required",
    "must have",
    "essential",
    "you will need",
    "minimum",
    "mandatory",
];
const NICE_CUES: &[&str] = &[
    "preferred",
    "desirable",
    "bonus",
    "nice to have",
    "advantageous",
    "ideally",
];

/// Split a requirement clause body into individual requirement strings on `;`,
/// trimming whitespace and a trailing period; drops empties.
fn split_items(body: &str) -> Vec<String> {
    body.split(';')
        .map(|s| s.trim().trim_end_matches('.').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Find the byte index just after the first matching cue (case-insensitive) and the
/// cue's classification, searching from `from`. Returns the earliest cue occurrence.
fn next_cue(hay_lower: &str, from: usize) -> Option<(usize, usize, bool)> {
    // returns (cue_start, cue_end, is_must)
    let mut best: Option<(usize, usize, bool)> = None;
    for (cues, is_must) in [(MUST_CUES, true), (NICE_CUES, false)] {
        for cue in cues {
            if let Some(rel) = hay_lower[from..].find(cue) {
                let start = from + rel;
                let end = start + cue.len();
                if best.is_none_or(|(bs, _, _)| start < bs) {
                    best = Some((start, end, is_must));
                }
            }
        }
    }
    best
}

/// Parse a raw JD into a Normalized Job (§F). Never panics; garbage / no-cue input
/// yields empty requirement buckets.
pub fn parse(raw: &str) -> NormalizedJob {
    let title = parse_title(raw);
    let company = parse_company(raw);

    let lower = raw.to_lowercase();
    let mut must = Vec::new();
    let mut nice = Vec::new();

    // Walk cue by cue; each cue owns the text up to the next cue (or end). A cue
    // body is terminated by the next sentence boundary that introduces another cue,
    // so we slice [cue_end .. next_cue_start].
    let mut cursor = 0usize;
    while let Some((_cstart, cend, is_must)) = next_cue(&lower, cursor) {
        // strip an immediate ":" after the cue
        let body_start = raw[cend..]
            .char_indices()
            .find(|(_, c)| *c != ':' && !c.is_whitespace())
            .map(|(i, _)| cend + i)
            .unwrap_or(cend);
        let body_end = match next_cue(&lower, cend) {
            Some((ns, _, _)) => ns,
            None => raw.len(),
        };
        // a body ends at the sentence period preceding the next cue; trim to the
        // last '.' inside the slice if the next cue starts a new sentence.
        let slice = &raw[body_start..body_end];
        let body = slice
            .rsplit_once('.')
            .map(|(head, _)| head)
            .unwrap_or(slice);
        let body = if body.trim().is_empty() { slice } else { body };
        let items = split_items(body);
        if is_must {
            must.extend(items);
        } else {
            nice.extend(items);
        }
        cursor = body_end;
    }

    NormalizedJob {
        title,
        company,
        location: String::new(),
        responsibilities: Vec::new(),
        requirements: Requirements {
            must_have: must,
            nice_to_have: nice,
        },
        keywords: Vec::new(),
    }
}

/// Title heuristic: text between "hiring a " / "hiring an " and " at ".
fn parse_title(raw: &str) -> String {
    let lower = raw.to_lowercase();
    for marker in ["hiring an ", "hiring a "] {
        if let Some(i) = lower.find(marker) {
            let start = i + marker.len();
            if let Some(rel) = lower[start..].find(" at ") {
                return raw[start..start + rel].trim().to_string();
            }
        }
    }
    String::new()
}

/// Company heuristic: text after " at " up to the next sentence period.
fn parse_company(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if let Some(i) = lower.find(" at ") {
        let start = i + 4;
        let end = raw[start..]
            .find('.')
            .map(|r| start + r)
            .unwrap_or(raw.len());
        return raw[start..end].trim().to_string();
    }
    String::new()
}

/// Serialize to the Normalized Job JSON shape (R-D1 seam).
pub fn to_json(job: &NormalizedJob) -> Result<String, serde_json::Error> {
    serde_json::to_string(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_required_and_nice_cues() {
        let j = parse("Required: A; B. Nice to have: C; D.");
        assert_eq!(j.requirements.must_have, vec!["A", "B"]);
        assert_eq!(j.requirements.nice_to_have, vec!["C", "D"]);
    }

    #[test]
    fn each_must_cue_phrase() {
        for cue in MUST_CUES {
            let j = parse(&format!("{cue}: Thing."));
            assert_eq!(j.requirements.must_have, vec!["Thing"], "cue {cue}");
        }
    }

    #[test]
    fn each_nice_cue_phrase() {
        for cue in NICE_CUES {
            let j = parse(&format!("{cue}: Thing."));
            assert_eq!(j.requirements.nice_to_have, vec!["Thing"], "cue {cue}");
        }
    }

    #[test]
    fn parses_title_and_company() {
        let j = parse("We are hiring a Senior Backend Engineer at Acme Group. Required: X.");
        assert_eq!(j.title, "Senior Backend Engineer");
        assert_eq!(j.company, "Acme Group");
    }

    #[test]
    fn empty_input_yields_empty_buckets() {
        let j = parse("");
        assert!(j.requirements.must_have.is_empty());
        assert!(j.requirements.nice_to_have.is_empty());
    }

    #[test]
    fn no_cue_garbage_no_panic_empty_requirements() {
        let j = parse("lorem ipsum dolor sit amet without any structure at all");
        assert!(j.requirements.must_have.is_empty());
        assert!(j.requirements.nice_to_have.is_empty());
    }

    #[test]
    fn unicode_does_not_panic() {
        let j = parse("Required: café ☕ skills; 日本語.");
        assert_eq!(j.requirements.must_have.len(), 2);
    }

    #[test]
    fn multi_line_headings() {
        let j = parse("Required:\n  TypeScript;\n  Python.\nNice to have:\n  GraphQL.");
        assert!(j.requirements.must_have.contains(&"TypeScript".to_string()));
        assert!(j.requirements.nice_to_have.contains(&"GraphQL".to_string()));
    }

    #[test]
    fn title_marker_hiring_an_and_no_at() {
        // "hiring an" marker branch + marker present but no " at " (falls through)
        let j = parse("We are hiring an Engineer without a company clause. Required: X.");
        // no " at " after the marker → title stays empty, company empty, parse still ok
        assert_eq!(j.title, "");
        assert_eq!(j.company, "");
        assert_eq!(j.requirements.must_have, vec!["X"]);
    }

    #[test]
    fn title_marker_hiring_an_with_at() {
        let j = parse("We are hiring an Architect at Globex. Required: Y.");
        assert_eq!(j.title, "Architect");
        assert_eq!(j.company, "Globex");
    }

    #[test]
    fn to_json_round_trips() {
        let j = parse("Required: A. Nice to have: B.");
        let s = to_json(&j).unwrap();
        let j2: NormalizedJob = serde_json::from_str(&s).unwrap();
        assert_eq!(j, j2);
    }
}
