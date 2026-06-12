//! Deterministic cue-token segmentation (R-CVI-3..5). Pure functions that split an
//! [`ExtractedText`](crate::extract::ExtractedText) into labelled [`Segment`]s. No
//! LLM, no heuristics beyond a fixed, tested cue vocabulary (bounded by R3a — the
//! synthetic-persona acceptance bar). Newlines are NOT assumed to be structure
//! (spike R3b); section *labels* and structural cues drive the split.

use crate::extract::ExtractedText;

/// Which master-CV skill list a labelled skills segment feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillBucket {
    ProgrammingLanguages,
    Skills,
    ToolsTechnologies,
    AsAServices,
}

/// One recognised experience block: a job line + the lines under it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExperienceBlock {
    pub job_title: String,
    pub business_name: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub location: Option<String>,
    pub bullets: Vec<String>,
}

/// The structured result of segmentation. Empty when nothing recognisable was found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Segments {
    pub name: Option<String>,
    pub title: Option<String>,
    pub skills: Vec<(SkillBucket, Vec<String>)>,
    pub experience: Vec<ExperienceBlock>,
}

impl Segments {
    /// True when no résumé structure was recovered at all → drives `ImportError::Empty`.
    pub(crate) fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.title.is_none()
            && self.skills.is_empty()
            && self.experience.is_empty()
    }
}

const BULLET_PREFIXES: &[char] = &['\u{25B9}', '\u{2023}', '\u{2022}', '\u{00B7}', '-', '*'];

/// Map a section-label line (case-insensitive) to a skills bucket, if it is one.
fn skill_bucket_for(label: &str) -> Option<SkillBucket> {
    match label.trim().to_lowercase().as_str() {
        "languages" | "programming languages" => Some(SkillBucket::ProgrammingLanguages),
        "skills" | "core skills" => Some(SkillBucket::Skills),
        "tools & technologies" | "tools and technologies" | "technologies" | "tools" => {
            Some(SkillBucket::ToolsTechnologies)
        }
        "platforms & services" | "platforms and services" | "platforms" | "services" => {
            Some(SkillBucket::AsAServices)
        }
        _ => None,
    }
}

/// Is this line the "Experience" section header?
fn is_experience_header(label: &str) -> bool {
    matches!(
        label.trim().to_lowercase().as_str(),
        "experience"
            | "work experience"
            | "employment"
            | "employment history"
            | "experience history"
    )
}

/// Split a skills line into individual skill tokens. DOCX gives a clean separator
/// list (`Python, Go, Rust`); PDF joins them with no separator (`PythonGoTypeScript`)
/// — in that case we cannot split deterministically, so we keep the whole run as a
/// single token (R3b: PDF skills are containment-only, asserted as such in L5).
fn split_skills(line: &str) -> Vec<String> {
    let separated: Vec<String> = line
        .split([',', '·', '•', '|', '\u{2022}'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    separated
}

/// A "job line" looks like `<title> … <Mon YYYY> – <Mon YYYY|Present>`. Returns
/// (title, start_date, end_date) when a trailing date range is present. The title is
/// everything before the date tail; an empty title is rejected.
fn parse_job_line(line: &str) -> Option<(String, String, Option<String>)> {
    let (head, range) = split_on_date_range(line)?;
    let title = head.trim().to_string();
    if title.is_empty() {
        return None;
    }
    // P-COV-cvimport-3: `range` begins at a month token, so `parse_date_range`'s
    // empty-start `None` arm is unreachable from here (its `?` cannot fail on this
    // input). `parse_date_range`'s `None` arm IS covered by a direct unit test
    // (`parse_date_range_rejects_empty_start`). Kept as `?` for a total, honest contract.
    let (start, end) = parse_date_range(&range)?;
    Some((title, start, end))
}

/// Find a `<Mon YYYY> <sep> <Mon YYYY|Present>` tail; return (everything before, the
/// tail beginning at the month token).
///
/// The search is case-insensitive but every byte offset is taken in the ORIGINAL
/// `line`, never in a lowercased copy: a title char whose lowercase has a different
/// byte length (e.g. `ẞ` U+1E9E → "ss", `İ` U+0130 → "i̇") would otherwise shift the
/// offset, slicing mid-`char` (panic) or off by a glyph (corruption). We walk the
/// original's `char_indices()` and, at each char boundary, compare a lowercased window
/// against each month token — so the returned indices are always valid boundaries.
fn split_on_date_range(line: &str) -> Option<(String, String)> {
    // The earliest original-string byte offset at which a month token begins, scanning
    // only at valid `char` boundaries of `line`.
    let month_start = line.char_indices().find_map(|(byte_pos, _)| {
        let rest = &line[byte_pos..];
        MONTHS
            .iter()
            .any(|m| starts_with_ignore_ascii_case(rest, m))
            .then_some(byte_pos)
    })?;
    let tail = &line[month_start..];
    // ensure a 4-digit year follows somewhere after the month in the tail
    if tail.chars().filter(|c| c.is_ascii_digit()).count() >= 4 {
        return Some((line[..month_start].to_string(), tail.to_string()));
    }
    None
}

/// True when `haystack` begins with `needle`, comparing ASCII case-insensitively. The
/// month tokens (`"jan "` …) are pure ASCII, so an ASCII-only fold is sufficient and
/// — unlike `to_lowercase()` — never changes byte length, keeping every offset a valid
/// `char` boundary of the original line.
fn starts_with_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack.len() >= needle.len()
        && haystack.as_bytes()[..needle.len()].eq_ignore_ascii_case(needle.as_bytes())
}

fn parse_date_range(range: &str) -> Option<(String, Option<String>)> {
    // normalise the two dash variants the template can emit
    let norm = range.replace(['\u{2013}', '\u{2014}'], "-");
    let parts: Vec<&str> = norm.splitn(2, '-').collect();
    let start = parts
        .first()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if start.is_empty() {
        return None;
    }
    let end = parts
        .get(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let end = match end {
        Some(e) if e.eq_ignore_ascii_case("present") => None,
        other => other,
    };
    Some((start, end))
}

const MONTHS: &[&str] = &[
    "jan ", "feb ", "mar ", "apr ", "may ", "jun ", "jul ", "aug ", "sep ", "oct ", "nov ", "dec ",
];

/// A `<businessName> · <location>` line under a job line.
fn parse_business_line(line: &str) -> (String, Option<String>) {
    if let Some((biz, loc)) = line.split_once('\u{00B7}') {
        (
            biz.trim().to_string(),
            Some(loc.trim().to_string()).filter(|s| !s.is_empty()),
        )
    } else {
        (line.trim().to_string(), None)
    }
}

/// Strip a leading bullet marker (and surrounding whitespace) from an achievement line.
fn strip_bullet(line: &str) -> &str {
    line.trim_start_matches(|c: char| BULLET_PREFIXES.contains(&c) || c.is_whitespace())
}

/// Split a line that may glue multiple `▹`-prefixed bullets together (PDF path)
/// into individual bullet texts. The first segment (before any marker) is included
/// only if non-empty.
fn split_glued_bullets(line: &str) -> Vec<String> {
    line.split('\u{25B9}')
        .map(|s| strip_bullet(s).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The deterministic segmenter. Walks the lines once, in order.
pub(crate) fn segment(text: &ExtractedText) -> Segments {
    let mut seg = Segments::default();
    let lines = &text.lines;

    // ── header: first non-empty line = name; next non-empty = title ───────────
    let mut idx = 0;
    while idx < lines.len() && lines[idx].trim().is_empty() {
        idx += 1;
    }
    if idx < lines.len() {
        seg.name = Some(lines[idx].trim().to_string());
        idx += 1;
        while idx < lines.len() && lines[idx].trim().is_empty() {
            idx += 1;
        }
        if idx < lines.len()
            && skill_bucket_for(&lines[idx]).is_none()
            && !is_experience_header(&lines[idx])
        {
            seg.title = Some(lines[idx].trim().to_string());
            idx += 1;
        }
    }

    // ── body: scan for skill-section labels and the experience section ────────
    let mut in_experience = false;
    let mut current: Option<ExperienceBlock> = None;

    let mut i = idx;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        if let Some(bucket) = skill_bucket_for(line) {
            // the next non-empty line is the skill list
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let skills = split_skills(lines[j].trim());
                if !skills.is_empty() {
                    seg.skills.push((bucket, skills));
                }
                i = j + 1;
                continue;
            }
            i += 1;
            continue;
        }

        if is_experience_header(line) {
            in_experience = true;
            i += 1;
            continue;
        }

        if in_experience {
            if let Some((title, start, end)) = parse_job_line(line) {
                // flush the previous block
                if let Some(block) = current.take() {
                    seg.experience.push(block);
                }
                let mut block = ExperienceBlock {
                    job_title: title,
                    start_date: start,
                    end_date: end,
                    ..Default::default()
                };
                // the next non-empty line should be the business · location line —
                // UNLESS it is itself a job line (two roles back-to-back, no business line
                // between). In that case leave business empty and let the loop pick the
                // next line up as the following block (Finding 5: never consume a job line
                // as a business_name).
                let mut j = i + 1;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }
                if j < lines.len() && parse_job_line(lines[j].trim()).is_none() {
                    let (biz, loc) = parse_business_line(lines[j].trim());
                    block.business_name = biz;
                    block.location = loc;
                    i = j + 1;
                } else {
                    i += 1;
                }
                current = Some(block);
                continue;
            }
            // otherwise: a bullet (possibly glued PDF bullets) under the current block
            if let Some(block) = current.as_mut() {
                for b in split_glued_bullets(line) {
                    block.bullets.push(b);
                }
            }
        }
        i += 1;
    }
    if let Some(block) = current.take() {
        seg.experience.push(block);
    }

    seg
}

#[cfg(test)]
mod tests {
    //! L1 — segmentation unit tests on small in-memory ExtractedText literals.
    use super::*;
    use crate::extract::ExtractedText;
    use std::time::Instant;

    fn et(lines: &[&str]) -> ExtractedText {
        ExtractedText {
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn header_maps_name_and_title() {
        // R-CVI-3
        let t0 = Instant::now();
        let seg = segment(&et(&["Devin Voss", "Senior Backend Engineer", ""]));
        eprintln!("[L1 perf] segment header: {:?}", t0.elapsed());
        assert_eq!(seg.name.as_deref(), Some("Devin Voss"));
        assert_eq!(seg.title.as_deref(), Some("Senior Backend Engineer"));
    }

    #[test]
    fn header_skips_leading_blanks() {
        let seg = segment(&et(&["", "", "Devin Voss", "Engineer"]));
        assert_eq!(seg.name.as_deref(), Some("Devin Voss"));
        assert_eq!(seg.title.as_deref(), Some("Engineer"));
    }

    #[test]
    fn title_not_taken_when_next_line_is_a_section_label() {
        // a résumé whose second line is already "Skills" has no title
        let seg = segment(&et(&["Devin Voss", "Skills", "Python, Go"]));
        assert_eq!(seg.name.as_deref(), Some("Devin Voss"));
        assert_eq!(seg.title, None);
        assert_eq!(seg.skills.len(), 1);
    }

    #[test]
    fn skills_segments_route_to_correct_buckets() {
        // R-CVI-4
        let seg = segment(&et(&[
            "Devin Voss",
            "Engineer",
            "Languages",
            "Python, Go, Rust",
            "Skills",
            "System Design, Mentoring",
            "Tools & Technologies",
            "Docker, Kubernetes",
            "Platforms & Services",
            "AWS, GCP",
        ]));
        let buckets: Vec<SkillBucket> = seg.skills.iter().map(|(b, _)| *b).collect();
        assert_eq!(
            buckets,
            vec![
                SkillBucket::ProgrammingLanguages,
                SkillBucket::Skills,
                SkillBucket::ToolsTechnologies,
                SkillBucket::AsAServices,
            ]
        );
        assert_eq!(seg.skills[0].1, vec!["Python", "Go", "Rust"]);
        assert_eq!(seg.skills[3].1, vec!["AWS", "GCP"]);
    }

    #[test]
    fn skill_label_is_case_insensitive() {
        let seg = segment(&et(&["N", "T", "LANGUAGES", "Python"]));
        assert_eq!(seg.skills[0].0, SkillBucket::ProgrammingLanguages);
    }

    #[test]
    fn experience_block_maps_title_business_dates_and_bullets() {
        // R-CVI-5
        let seg = segment(&et(&[
            "Devin Voss",
            "Engineer",
            "Experience",
            "Backend Engineer Nov 2022 – Present",
            "Acme Co · Sydney, Australia",
            "Cut p99 latency by 38%",
            "Introduced contract tests",
        ]));
        assert_eq!(seg.experience.len(), 1);
        let e = &seg.experience[0];
        assert_eq!(e.job_title, "Backend Engineer");
        assert_eq!(e.business_name, "Acme Co");
        assert_eq!(e.location.as_deref(), Some("Sydney, Australia"));
        assert_eq!(e.start_date, "Nov 2022");
        assert_eq!(e.end_date, None); // "Present" → None
        assert_eq!(
            e.bullets,
            vec!["Cut p99 latency by 38%", "Introduced contract tests"]
        );
    }

    #[test]
    fn experience_end_date_recovered_when_not_present() {
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer Feb 2021 – Mar 2022",
            "Meridian · Melbourne",
            "Did a thing",
        ]));
        let e = &seg.experience[0];
        assert_eq!(e.start_date, "Feb 2021");
        assert_eq!(e.end_date.as_deref(), Some("Mar 2022"));
    }

    #[test]
    fn glued_pdf_bullets_split_on_triangle_marker() {
        // R3b: pdf-extract glues the first bullet to following ▹-prefixed ones
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer Jan 2020 – Jan 2021",
            "Biz · City",
            "First bullet glued\u{25B9}   Second bullet\u{25B9}   Third bullet",
        ]));
        assert_eq!(
            seg.experience[0].bullets,
            vec!["First bullet glued", "Second bullet", "Third bullet"]
        );
    }

    #[test]
    fn business_line_without_location_is_tolerated() {
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer Jan 2020 – Present",
            "SoloCorp",
            "A bullet",
        ]));
        assert_eq!(seg.experience[0].business_name, "SoloCorp");
        assert_eq!(seg.experience[0].location, None);
    }

    #[test]
    fn multiple_experience_blocks_are_separated() {
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer A Jan 2022 – Present",
            "Acme · Sydney",
            "bullet a",
            "Engineer B Jan 2019 – Dec 2021",
            "Beta · Perth",
            "bullet b",
        ]));
        assert_eq!(seg.experience.len(), 2);
        assert_eq!(seg.experience[0].job_title, "Engineer A");
        assert_eq!(seg.experience[1].job_title, "Engineer B");
        assert_eq!(seg.experience[1].bullets, vec!["bullet b"]);
    }

    #[test]
    fn consecutive_job_lines_do_not_drop_a_block() {
        // Finding 5: two job lines back-to-back (no `business · location` between them),
        // then a business line. The second job line must NOT be eaten as the first's
        // business_name; BOTH experiences are recovered and neither business holds a date.
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer A Jan 2022 – Present",
            "Engineer B Jan 2019 – Dec 2021",
            "Beta · Perth",
            "bullet b",
        ]));
        assert_eq!(seg.experience.len(), 2, "both experiences recovered");
        assert_eq!(seg.experience[0].job_title, "Engineer A");
        assert_eq!(seg.experience[1].job_title, "Engineer B");
        // the first block had no business line of its own → empty, NOT the second job line
        assert_eq!(seg.experience[0].business_name, "");
        assert_eq!(seg.experience[1].business_name, "Beta");
        // neither business_name accidentally captured a date tail
        for e in &seg.experience {
            assert!(
                !e.business_name.to_lowercase().contains("jan") && !e.business_name.contains("20"),
                "business_name must not contain a date: {:?}",
                e.business_name
            );
        }
    }

    #[test]
    fn structureless_text_segments_to_empty() {
        // a line with no header/skills/experience cue → only a name is taken; but a
        // truly blank doc yields an empty Segments (drives ImportError::Empty upstream)
        let seg = segment(&et(&["", "   ", ""]));
        assert!(seg.is_empty());
    }

    #[test]
    fn split_skills_handles_separatorless_pdf_run() {
        // R3b: PDF skills come with no separator — kept as a single token (containment)
        let one = split_skills("PythonGoTypeScriptSQLRust");
        assert_eq!(one, vec!["PythonGoTypeScriptSQLRust"]);
    }

    #[test]
    fn parse_job_line_rejects_a_line_with_no_date_tail() {
        assert!(parse_job_line("Just a heading with no dates").is_none());
    }

    #[test]
    fn parse_job_line_rejects_empty_title() {
        // a bare date range with no title head
        assert!(parse_job_line("Jan 2020 – Jan 2021").is_none());
    }

    // ── edge branches (coverage of the reachable defensive paths) ─────────────

    #[test]
    fn parse_date_range_rejects_empty_start() {
        // a range string that begins with the dash → empty start → None (line 128)
        assert!(parse_date_range("- Mar 2022").is_none());
    }

    #[test]
    fn parse_date_range_none_on_bare_dash() {
        // a range that is just a dash → empty start → None (the reachable None arm)
        assert!(parse_date_range("-").is_none());
    }

    #[test]
    fn header_with_blank_between_name_and_title() {
        // a blank line between the name and the title exercises the inner skip loop (185)
        let seg = segment(&et(&["Devin Voss", "", "Engineer"]));
        assert_eq!(seg.name.as_deref(), Some("Devin Voss"));
        assert_eq!(seg.title.as_deref(), Some("Engineer"));
    }

    #[test]
    fn skills_label_with_blank_then_list() {
        // a blank line between a skills label and its list exercises the skip loop (212)
        let seg = segment(&et(&["N", "T", "Languages", "", "Python, Go"]));
        assert_eq!(seg.skills.len(), 1);
        assert_eq!(seg.skills[0].1, vec!["Python", "Go"]);
    }

    #[test]
    fn skills_label_followed_by_separators_only_pushes_no_bucket() {
        // Finding 4: a skills label whose next line is separators-only (",,,") splits to an
        // EMPTY token list → the `if !skills.is_empty()` FALSE arm fires and NO (empty) skill
        // bucket is pushed. Import still proceeds (name recovered), just with no skills.
        let seg = segment(&et(&["Devin Voss", "Engineer", "Skills", ",,,"]));
        assert!(
            seg.skills.is_empty(),
            "a separators-only line must not create an empty skill bucket"
        );
        assert_eq!(seg.name.as_deref(), Some("Devin Voss"));
    }

    #[test]
    fn skills_label_at_end_of_document_yields_no_list() {
        // a skills label as the LAST line → no following list line (j >= len, lines 222-224)
        let seg = segment(&et(&["N", "T", "Languages"]));
        assert!(seg.skills.is_empty());
    }

    #[test]
    fn skills_label_followed_only_by_blank_then_eof() {
        // label, then only blanks to EOF → inner loop walks off the end (the j>=len arm)
        let seg = segment(&et(&["N", "T", "Skills", "   ", ""]));
        assert!(seg.skills.is_empty());
    }

    #[test]
    fn job_line_at_end_of_document_has_no_business_line() {
        // an experience job line as the LAST line → no business line (else arm, line 255)
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "Engineer Jan 2020 – Present",
        ]));
        assert_eq!(seg.experience.len(), 1);
        assert_eq!(seg.experience[0].job_title, "Engineer");
        assert_eq!(seg.experience[0].business_name, "");
    }

    #[test]
    fn bullets_before_any_job_line_are_ignored() {
        // in the experience section, a bullet with no current block is dropped (no panic):
        // exercises the `if let Some(block) = current.as_mut()` false arm.
        let seg = segment(&et(&[
            "N",
            "T",
            "Experience",
            "a stray bullet with no job line",
        ]));
        assert!(seg.experience.is_empty());
    }

    #[test]
    fn split_on_date_range_handles_expanding_lowercase_title_without_panic() {
        // Finding 1 (CRITICAL): a title char whose lowercase has a DIFFERENT byte length
        // (`ẞ` U+1E9E → "ss") must not shift the slice offset. `"ẞéJan …"` previously sliced
        // the original at a lowercased-copy offset that landed INSIDE `é` → panic.
        // After the fix the function returns a clean split at a valid char boundary.
        let r = split_on_date_range("ẞé Jan 2020 – Present");
        assert_eq!(
            r,
            Some(("ẞé ".to_string(), "Jan 2020 – Present".to_string()))
        );
        // the no-separator variant that used to panic mid-`é`:
        let r2 = split_on_date_range("ẞéJan 2020 – Present");
        assert_eq!(
            r2,
            Some(("ẞé".to_string(), "Jan 2020 – Present".to_string()))
        );
    }

    #[test]
    fn parse_job_line_recovers_title_and_date_for_expanding_lowercase_initial() {
        // `İ` (U+0130) lower-cases to "i̇" (2 bytes → 3): the old code grabbed the month's
        // first letter into the title ("İ J") and dropped it from the date. The fix recovers
        // the correct title and an intact start date.
        let parsed = parse_job_line("İ Engineer Jan 2020 – Present");
        assert_eq!(
            parsed,
            Some(("İ Engineer".to_string(), "Jan 2020".to_string(), None))
        );
    }

    #[test]
    fn parse_job_line_ascii_title_unchanged_regression() {
        // Regression: ordinary ASCII titles still parse title/date exactly as before.
        let parsed = parse_job_line("Backend Engineer Nov 2022 – Present");
        assert_eq!(
            parsed,
            Some(("Backend Engineer".to_string(), "Nov 2022".to_string(), None))
        );
    }

    #[test]
    fn split_on_date_range_none_when_month_present_but_too_few_digits() {
        // a month token with fewer than 4 digits in the tail → the inner `if` is false
        // and the loop completes to `None` (line 113).
        assert!(split_on_date_range("Engineer May day 12").is_none());
    }

    #[test]
    fn skills_label_immediately_followed_by_eof_hits_no_list_arm() {
        // a skills label whose next index is past the end → the `i += 1; continue` arm
        // (line 219). Here "Skills" is the final meaningful line.
        let seg = segment(&et(&["Name", "Title", "Skills"]));
        assert!(seg.skills.is_empty());
        assert_eq!(seg.name.as_deref(), Some("Name"));
    }
}
