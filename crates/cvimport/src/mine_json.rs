//! Adaptive JSON miner (item 8a) — mine an ARBITRARY CV JSON value into a NEW
//! [`aa_core::MasterCv`] (I1). Deterministic; **no LLM, no network**.
//!
//! The Master-CV schema is INTERNAL: callers pass whatever JSON shape they have and the
//! miner maps known **case-insensitive synonym keys** (never fixed JSON paths) onto the
//! master-CV fields the app needs, builds the struct DIRECTLY (never via the text
//! `Segments`/`map` path — which has no contact slots and hardcodes proficiency 3, and so
//! would drop the entire contact block and every real proficiency — R-INGEST-9), assigns
//! synthetic ids (`imp_exp_N` / `imp_exp_N_bM`), and applies honesty defaults (proficiency
//! 3 only when the source carries no usable rating; never invents text — R-INGEST-7).
//!
//! Every helper is **total** over arbitrary `serde_json::Value` — none panics (R-INGEST-14,
//! I5). Output is guaranteed to validate against `doc/schemas/master-cv.schema.json` for any
//! input whose emitted experiences carry the schema-required fields (R-INGEST-10/13).

use aa_core::{Achievement, Experience, MasterCv, Person, Skill};
use serde::Serialize;
use serde_json::Value;

use crate::error::ImportError;

/// Default proficiency for an imported skill that carries no usable rating. Mirrors
/// `map.rs`'s `IMPORTED_PROFICIENCY`: the schema requires `proficiency` ∈ 1..5, so an
/// unrated skill gets a neutral mid-scale 3 — honest ("unrated/imported, neutral"), never
/// an inflated or invented rating (I3). The user re-rates during review.
const DEFAULT_PROFICIENCY: u8 = 3;

/// Candidate person-object keys, in preference order (R-INGEST-2).
const PERSON_OBJECT_KEYS: &[&str] = &["person", "basics", "profile"];

/// Candidate role-array container keys, in disambiguation priority order (R-INGEST-4).
const ROLE_ARRAY_KEYS: &[&str] = &[
    "experience",
    "work",
    "workExperience",
    "employment",
    "positions",
    "history",
];

/// What the miner could NOT find that the app needs (item 8b's CLI renders this to the user).
/// Each `missing_*` flag is TRUE when that IMPORTANT class is empty in the produced MasterCv.
/// `ignored_role_arrays` names every role-shaped array that lost the highest-priority-key
/// contest (multi-array disambiguation) — surfaced, never silently merged in v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompletenessReport {
    pub missing_person_name: bool,
    pub missing_experience: bool,
    pub missing_achievement: bool,
    pub missing_skill: bool,
    pub ignored_role_arrays: Vec<String>,
}

impl CompletenessReport {
    /// True when every IMPORTANT class is present (all four `missing_*` are false). The CLI
    /// (item 8b) uses this for its "ready to install?" decision. `ignored_role_arrays` is
    /// informational and does NOT affect completeness.
    pub fn is_complete(&self) -> bool {
        !self.missing_person_name
            && !self.missing_experience
            && !self.missing_achievement
            && !self.missing_skill
    }
}

/// Mine an arbitrary CV JSON value into a NEW `aa_core::MasterCv` (I1). Deterministic; no
/// LLM, no network. Errors with [`ImportError::Empty`] when the value carries no
/// recognisable CV content at all (no person name AND no experience AND no skill).
pub fn import_cv_json(v: &Value) -> Result<MasterCv, ImportError> {
    let src = person_source(v);
    let person = extract_person(src, v);
    let (experience, _ignored) = extract_experience(v);
    let skills = extract_skills(v);

    // Empty gate (R-INGEST-14): no recognisable content at all.
    if person.name.is_none() && experience.is_empty() && skills.is_empty() {
        return Err(ImportError::Empty);
    }

    let headline = person.professional_title.clone();
    let mut cv = MasterCv {
        schema_version: "1.0.0".to_string(),
        person,
        headline,
        summary_variants: Vec::new(),
        programming_languages: Vec::new(),
        skills: Vec::new(),
        tools_technologies: Vec::new(),
        as_a_services: Vec::new(),
        experience,
        projects: Vec::new(),
        education: Vec::new(),
        certifications: Vec::new(),
        awards: Vec::new(),
        preferences: None,
    };
    cv.programming_languages = skills.programming_languages;
    cv.skills = skills.skills;
    cv.tools_technologies = skills.tools_technologies;
    cv.as_a_services = skills.as_a_services;
    Ok(cv)
}

/// Pure: derive a completeness report from a produced `MasterCv` plus the names of role
/// arrays the miner ignored during disambiguation (not recoverable from the collapsed CV,
/// so passed in — see [`ignored_role_arrays`]). R-INGEST-11/12.
pub fn completeness(cv: &MasterCv, ignored_role_arrays: &[String]) -> CompletenessReport {
    let missing_person_name = cv.person.name.is_none();
    let missing_experience = !cv
        .experience
        .iter()
        .any(|e| !e.job_title.is_empty() && !e.business_name.is_empty());
    let missing_achievement = !cv.experience.iter().any(|e| {
        e.achievements_tasks
            .iter()
            .any(|a| !a.description.is_empty())
    });
    let missing_skill = cv.programming_languages.is_empty()
        && cv.skills.is_empty()
        && cv.tools_technologies.is_empty()
        && cv.as_a_services.is_empty();
    CompletenessReport {
        missing_person_name,
        missing_experience,
        missing_achievement,
        missing_skill,
        ignored_role_arrays: ignored_role_arrays.to_vec(),
    }
}

/// The source key names of every role-shaped array the miner ignored during disambiguation
/// (R-INGEST-4/12). The winning array's data is in the `MasterCv`, but the ignored names are
/// not — item 8b's CLI calls this, then threads the result into [`completeness`].
pub fn ignored_role_arrays(v: &Value) -> Vec<String> {
    let (_exp, ignored) = extract_experience(v);
    ignored
}

// ── extractors ──────────────────────────────────────────────────────────────────────────

/// Return the dedicated person object when a `person`/`basics`/`profile` key is present
/// (first match in that priority order), else the top-level value. Pure lookup; never clones.
fn person_source(v: &Value) -> &Value {
    for key in PERSON_OBJECT_KEYS {
        if let Some(obj @ Value::Object(_)) = lc_get(v, key) {
            return obj;
        }
    }
    v
}

/// Pull each `Person` field by synonym priority order (R-INGEST-1), case-insensitive key
/// match. Contact fields not found in the dedicated `src` object fall back to `root` (a
/// `basics` object may hold name/summary while email sits top-level — R-INGEST-2). Empty/
/// whitespace values map to `None`.
fn extract_person(src: &Value, root: &Value) -> Person {
    let pick = |keys: &[&str]| str_field(src, keys).or_else(|| str_field(root, keys));
    Person {
        name: str_field(src, &["name", "fullName", "candidateName"])
            .or_else(|| str_field(root, &["name", "fullName", "candidateName"])),
        professional_title: pick(&["professionalTitle", "title", "headline", "role", "label"]),
        professional_description: pick(&["professionalDescription", "summary", "about", "bio"]),
        location: pick(&["location"]),
        email: pick(&["email"]),
        phone: pick(&["phone"]),
        linkedin: pick(&["linkedin"]),
        github: pick(&["github"]),
        website: pick(&["website", "url"]),
        image: None,
    }
}

/// Find the winning role array (R-INGEST-4 disambiguation), map each element to an
/// `Experience` (R-INGEST-3/13), coerce dates (R-INGEST-6), split achievements (R-INGEST-5).
/// Returns the built experiences AND the names of role arrays it ignored (R-INGEST-12).
fn extract_experience(v: &Value) -> (Vec<Experience>, Vec<String>) {
    // The winning key is the highest-priority key holding a NON-EMPTY array.
    let mut winner: Option<&str> = None;
    for key in ROLE_ARRAY_KEYS {
        if let Some(arr) = lc_get(v, key).and_then(Value::as_array) {
            if !arr.is_empty() {
                winner = Some(key);
                break;
            }
        }
    }
    let Some(win_key) = winner else {
        return (Vec::new(), Vec::new());
    };

    // Every OTHER present non-empty role array is ignored and named.
    let ignored: Vec<String> = ROLE_ARRAY_KEYS
        .iter()
        .filter(|k| **k != win_key)
        .filter(|k| {
            lc_get(v, k)
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty())
        })
        .map(|k| (*k).to_string())
        .collect();

    let arr = lc_get(v, win_key)
        .and_then(Value::as_array)
        .expect("winner holds a non-empty array");

    let mut experiences = Vec::new();
    for el in arr {
        let job_title = str_field(el, &["jobTitle", "title", "position", "role"]);
        // R-INGEST-13: emit iff a non-empty jobTitle was found.
        let Some(job_title) = job_title else { continue };
        let n = experiences.len(); // dense ids over EMITTED experiences
        let exp_id = format!("imp_exp_{n}");
        let achievements = extract_achievements(el, &exp_id);
        experiences.push(Experience {
            id: exp_id,
            job_title,
            // `name` is the LOWEST-priority businessName synonym: JSON Resume keys the
            // employer as `work[].name` (DISCUSS-8a-4). Safe here because it is only
            // consulted inside a role-array element, after the explicit business keys.
            business_name: str_field(
                el,
                &[
                    "businessName",
                    "company",
                    "employer",
                    "organisation",
                    "name",
                ],
            )
            .unwrap_or_default(),
            consultancy: None,
            location: str_field(el, &["location"]),
            employment_type: None,
            start_date: coerce_date(
                lc_get(el, "startDate")
                    .or_else(|| lc_get(el, "start"))
                    .or_else(|| lc_get(el, "from")),
            ),
            end_date: {
                let d = coerce_date(
                    lc_get(el, "endDate")
                        .or_else(|| lc_get(el, "end"))
                        .or_else(|| lc_get(el, "to")),
                );
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            domain: None,
            hide: None,
            contact: None,
            tags: Vec::new(),
            achievements_tasks: achievements,
        });
    }
    (experiences, ignored)
}

/// Map an experience element's achievement value to `Achievement`s (R-INGEST-5). The value
/// is found by synonym key and may be a single string (newline-split), an array of strings
/// (each split), or an array of objects keyed `description`/`text`/`name`.
fn extract_achievements(el: &Value, exp_id: &str) -> Vec<Achievement> {
    const ACH_KEYS: &[&str] = &[
        "achievementsTasks",
        "achievements",
        "highlights",
        "bullets",
        "responsibilities",
        "tasks",
    ];
    let mut descriptions: Vec<String> = Vec::new();
    for key in ACH_KEYS {
        match lc_get(el, key) {
            Some(Value::String(s)) => descriptions.extend(split_achievement(s)),
            Some(Value::Array(items)) => {
                for item in items {
                    match item {
                        Value::String(s) => descriptions.extend(split_achievement(s)),
                        Value::Object(_) => {
                            if let Some(text) = str_field(item, &["description", "text", "name"]) {
                                descriptions.extend(split_achievement(&text));
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => continue,
        }
        // first present achievement key wins
        break;
    }
    descriptions
        .into_iter()
        .enumerate()
        .map(|(m, description)| Achievement {
            id: format!("{exp_id}_b{m}"),
            description,
            emphasise: None,
            tags: Vec::new(),
            metrics: Vec::new(),
            evidence_strength: None,
        })
        .collect()
}

/// The four master-CV skill lists the miner buckets into.
struct SkillLists {
    programming_languages: Vec<Skill>,
    skills: Vec<Skill>,
    tools_technologies: Vec<Skill>,
    as_a_services: Vec<Skill>,
}

impl SkillLists {
    /// True when no skill was found in any of the four lists (feeds the Empty gate).
    fn is_empty(&self) -> bool {
        self.programming_languages.is_empty()
            && self.skills.is_empty()
            && self.tools_technologies.is_empty()
            && self.as_a_services.is_empty()
    }
}

/// Walk the skills synonym keys (R-INGEST-7), bucketing each array by source key into one of
/// the four master-CV lists, mapping string OR `{name, proficiency|level|rating}` elements
/// and applying the proficiency honesty default.
fn extract_skills(v: &Value) -> SkillLists {
    let mut lists = SkillLists {
        programming_languages: Vec::new(),
        skills: Vec::new(),
        tools_technologies: Vec::new(),
        as_a_services: Vec::new(),
    };
    // (source key, target bucket index). `languages` buckets to `skills` (v1 limitation).
    let plan: [(&str, u8); 8] = [
        ("programmingLanguages", 0),
        ("skills", 1),
        ("languages", 1),
        ("tools", 2),
        ("technologies", 2),
        ("toolsTechnologies", 2),
        ("asAServices", 3),
        ("services", 3),
    ];
    for (key, bucket) in plan {
        if let Some(arr) = lc_get(v, key).and_then(Value::as_array) {
            for el in arr {
                if let Some(skill) = skill_from(el) {
                    match bucket {
                        0 => lists.programming_languages.push(skill),
                        1 => lists.skills.push(skill),
                        2 => lists.tools_technologies.push(skill),
                        _ => lists.as_a_services.push(skill),
                    }
                }
            }
        }
    }
    lists
}

// ── small helpers ─────────────────────────────────────────────────────────────────────────

/// Case-insensitive object lookup — the primitive every synonym match builds on. Returns the
/// first value whose key matches `key` ignoring ASCII case. `None` for non-objects.
fn lc_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object()?
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, val)| val)
}

/// First present, non-empty (after trim) string synonym in `keys`, case-insensitive.
fn str_field(v: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = lc_get(v, key).and_then(Value::as_str) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// Coerce a date value to a string: a JSON number → its integer string (`2019` → `"2019"`);
/// a string → trimmed verbatim; anything else (incl. absent) → empty string.
fn coerce_date(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                // a non-integer number (e.g. 2019.5) — drop the fractional part honestly
                // by truncation; serde always exposes a finite number here.
                (n.as_f64().unwrap_or_default().trunc() as i64).to_string()
            }
        }
        Some(Value::String(s)) => s.trim().to_string(),
        _ => String::new(),
    }
}

/// Split an achievement string into bullets on newlines (trim, drop empties). A string with
/// no newline → exactly one bullet (R-INGEST-5).
fn split_achievement(s: &str) -> Vec<String> {
    s.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Map a skill element (string OR `{name, proficiency|level|rating}`) to a `Skill`. Returns
/// `None` when no usable name is found (honesty: never emit a blank skill).
fn skill_from(v: &Value) -> Option<Skill> {
    let name = match v {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                return None;
            }
            t.to_string()
        }
        Value::Object(_) => str_field(v, &["name"])?,
        _ => return None,
    };
    let proficiency = proficiency_of(v);
    Some(Skill {
        name,
        proficiency,
        aliases: Vec::new(),
        evidence_ids: Vec::new(),
    })
}

/// Resolve a skill's proficiency (R-INGEST-7 / DISCUSS-8a-3): the source rating
/// (`proficiency` → `level` → `rating`, first present) is used ONLY when it is already an
/// integer in `1..=5`; anything else (out of range, non-integer, non-number, absent) → the
/// honest default 3.
fn proficiency_of(v: &Value) -> u8 {
    for key in ["proficiency", "level", "rating"] {
        if let Some(n) = lc_get(v, key) {
            if let Some(i) = n.as_i64() {
                if (1..=5).contains(&i) {
                    return i as u8;
                }
            }
            // present but unusable (out of range / non-integer / non-number) → default.
            return DEFAULT_PROFICIENCY;
        }
    }
    DEFAULT_PROFICIENCY
}

#[cfg(test)]
mod tests {
    //! L1 — adaptive JSON miner unit tests (in-memory `serde_json::json!` literals; no
    //! fixture IO). Carries the bulk of coverage: every extractor + every priority-order
    //! arm + every honesty default + the Empty gate + the abuse/never-panic guarantee.
    use super::*;
    use serde_json::json;

    fn mine(v: Value) -> MasterCv {
        import_cv_json(&v).unwrap()
    }

    // ── person synonyms (R-INGEST-1) ───────────────────────────────────────────
    #[test]
    fn person_name_synonym_order() {
        // R-INGEST-1: name → fullName → candidateName; first present non-empty wins.
        assert_eq!(
            mine(json!({"name": "A", "fullName": "B"}))
                .person
                .name
                .as_deref(),
            Some("A")
        );
        assert_eq!(
            mine(json!({"fullName": "B", "candidateName": "C"}))
                .person
                .name
                .as_deref(),
            Some("B")
        );
        assert_eq!(
            mine(json!({"candidateName": "C"})).person.name.as_deref(),
            Some("C")
        );
    }

    #[test]
    fn person_title_and_description_synonym_order() {
        // R-INGEST-1
        let cv = mine(json!({"name": "N", "role": "Eng", "about": "does things"}));
        assert_eq!(cv.person.professional_title.as_deref(), Some("Eng"));
        assert_eq!(
            cv.person.professional_description.as_deref(),
            Some("does things")
        );
        // headline mirrors professional title
        assert_eq!(cv.headline.as_deref(), Some("Eng"));
        // title beats role/label
        let cv2 = mine(json!({"name":"N","title":"T","role":"R","label":"L"}));
        assert_eq!(cv2.person.professional_title.as_deref(), Some("T"));
    }

    #[test]
    fn person_website_url_synonym() {
        // R-INGEST-1: website → url
        assert_eq!(
            mine(json!({"name":"N","url":"https://x.example"}))
                .person
                .website
                .as_deref(),
            Some("https://x.example")
        );
        assert_eq!(
            mine(json!({"name":"N","website":"https://w.example","url":"https://u.example"}))
                .person
                .website
                .as_deref(),
            Some("https://w.example")
        );
    }

    #[test]
    fn person_keys_are_case_insensitive() {
        // R-INGEST-1: PascalCase keys (the DW_CV shape) match.
        let cv =
            mine(json!({"Name":"Dana","Email":"d@example.com","LinkedIn":"https://l.example"}));
        assert_eq!(cv.person.name.as_deref(), Some("Dana"));
        assert_eq!(cv.person.email.as_deref(), Some("d@example.com"));
        assert_eq!(cv.person.linkedin.as_deref(), Some("https://l.example"));
    }

    #[test]
    fn blank_person_field_becomes_none() {
        // honesty: empty/whitespace string → None (keeps person additionalProperties clean)
        let cv = mine(json!({"name":"N","email":"   ","phone":""}));
        assert_eq!(cv.person.email, None);
        assert_eq!(cv.person.phone, None);
    }

    // ── dedicated person object preference (R-INGEST-2) ─────────────────────────
    #[test]
    fn dedicated_person_object_preference_and_fallback() {
        // R-INGEST-2: `basics` is the person source; email falls back to top-level.
        let cv = mine(json!({
            "basics": {"name": "Rowan", "label": "Staff Eng"},
            "email": "rowan@example.com"
        }));
        assert_eq!(cv.person.name.as_deref(), Some("Rowan"));
        assert_eq!(cv.person.professional_title.as_deref(), Some("Staff Eng"));
        assert_eq!(cv.person.email.as_deref(), Some("rowan@example.com"));
    }

    #[test]
    fn person_object_priority_person_then_basics_then_profile() {
        // R-INGEST-2: `person` beats `basics` beats `profile`.
        let cv = mine(json!({
            "person": {"name": "P"},
            "basics": {"name": "B"},
            "profile": {"name": "Pr"}
        }));
        assert_eq!(cv.person.name.as_deref(), Some("P"));
        let cv2 = mine(json!({"basics": {"name": "B"}, "profile": {"name": "Pr"}}));
        assert_eq!(cv2.person.name.as_deref(), Some("B"));
        let cv3 = mine(json!({"profile": {"name": "Pr"}}));
        assert_eq!(cv3.person.name.as_deref(), Some("Pr"));
    }

    // ── experience synonyms (R-INGEST-3) ────────────────────────────────────────
    #[test]
    fn experience_synonym_mapping() {
        // R-INGEST-3: jobTitle/title/position/role; businessName/company/employer/organisation;
        // startDate/start/from; endDate/end/to.
        let cv = mine(json!({
            "experience": [
                {"position": "Eng", "company": "Acme", "from": "2020", "to": "2023"}
            ]
        }));
        assert_eq!(cv.experience.len(), 1);
        let e = &cv.experience[0];
        assert_eq!(e.job_title, "Eng");
        assert_eq!(e.business_name, "Acme");
        assert_eq!(e.start_date, "2020");
        assert_eq!(e.end_date.as_deref(), Some("2023"));
    }

    #[test]
    fn experience_business_name_from_lowest_priority_name_key() {
        // R-INGEST-3 / DISCUSS-8a-4: JSON Resume keys the employer as work[].name.
        // `name` is the LOWEST-priority businessName synonym, beaten by explicit keys.
        let cv = mine(json!({
            "work": [{"position": "Eng", "name": "JsonResume Co"}]
        }));
        assert_eq!(cv.experience[0].business_name, "JsonResume Co");
        // explicit businessName/company beats the generic `name`
        let cv2 = mine(json!({
            "experience": [{"jobTitle": "Eng", "company": "Acme", "name": "Ignored"}]
        }));
        assert_eq!(cv2.experience[0].business_name, "Acme");
    }

    // ── multi-array disambiguation (R-INGEST-4) ─────────────────────────────────
    #[test]
    fn multi_role_array_disambiguation_names_ignored() {
        // R-INGEST-4: experience wins; work + positions named ignored.
        let v = json!({
            "experience": [{"jobTitle": "Lead", "businessName": "N"}],
            "work": [{"position": "Junior", "name": "Old"}],
            "positions": [{"title": "Intern", "company": "Tiny"}]
        });
        let cv = import_cv_json(&v).unwrap();
        assert_eq!(cv.experience.len(), 1);
        assert_eq!(cv.experience[0].job_title, "Lead");
        let ignored = ignored_role_arrays(&v);
        assert!(ignored.contains(&"work".to_string()));
        assert!(ignored.contains(&"positions".to_string()));
        assert!(!ignored.contains(&"experience".to_string()));
    }

    #[test]
    fn empty_role_array_is_skipped_for_next_priority() {
        // R-INGEST-4: an EMPTY higher-priority array does not win; `work` wins.
        let v = json!({
            "experience": [],
            "work": [{"position": "Eng", "name": "Co"}]
        });
        let cv = import_cv_json(&v).unwrap();
        assert_eq!(cv.experience.len(), 1);
        assert_eq!(cv.experience[0].job_title, "Eng");
        // an empty array is not a "lost contest" array — nothing meaningful to surface
        let ignored = ignored_role_arrays(&v);
        assert!(!ignored.contains(&"experience".to_string()));
    }

    // ── achievements (R-INGEST-5) ───────────────────────────────────────────────
    #[test]
    fn achievement_newline_split_and_object_form() {
        // R-INGEST-5: a \n-joined string splits to N bullets; a no-\n string → 1 bullet;
        // object elements keyed description/text/name.
        let cv = mine(json!({
            "experience": [
                {"jobTitle": "T", "businessName": "B",
                 "achievements": "line one\nline two\n\n  line three  "},
                {"jobTitle": "T2", "businessName": "B2", "achievements": "single line"},
                {"jobTitle": "T3", "businessName": "B3",
                 "highlights": [{"description": "obj one"}, {"text": "obj two"}, {"name": "obj three"}, "plain"]}
            ]
        }));
        // newline blob → 3 bullets (empty line dropped, trimmed)
        assert_eq!(cv.experience[0].achievements_tasks.len(), 3);
        assert_eq!(
            cv.experience[0].achievements_tasks[0].description,
            "line one"
        );
        assert_eq!(
            cv.experience[0].achievements_tasks[2].description,
            "line three"
        );
        // no-newline → 1 bullet
        assert_eq!(cv.experience[1].achievements_tasks.len(), 1);
        assert_eq!(
            cv.experience[1].achievements_tasks[0].description,
            "single line"
        );
        // object elements via priority
        let third = &cv.experience[2].achievements_tasks;
        assert_eq!(third.len(), 4);
        assert_eq!(third[0].description, "obj one");
        assert_eq!(third[1].description, "obj two");
        assert_eq!(third[2].description, "obj three");
        assert_eq!(third[3].description, "plain");
    }

    #[test]
    fn achievement_array_of_strings_keeps_each() {
        // R-INGEST-5: an array of plain strings → one bullet each (no further split needed,
        // but a multi-line element within the array still splits).
        let cv = mine(json!({
            "experience": [
                {"jobTitle":"T","businessName":"B","achievements":["a\nb","c"]}
            ]
        }));
        // "a\nb" → 2, "c" → 1
        assert_eq!(cv.experience[0].achievements_tasks.len(), 3);
    }

    // ── dates (R-INGEST-6) ──────────────────────────────────────────────────────
    #[test]
    fn numeric_date_coerced_string_verbatim() {
        // R-INGEST-6: number → integer string; string → verbatim.
        let cv = mine(json!({
            "experience": [{"jobTitle":"T","businessName":"B","startDate": 2019, "endDate": 2022}]
        }));
        assert_eq!(cv.experience[0].start_date, "2019");
        assert_eq!(cv.experience[0].end_date.as_deref(), Some("2022"));
        let cv2 = mine(json!({
            "experience": [{"jobTitle":"T","businessName":"B","startDate": "Jan 2019"}]
        }));
        assert_eq!(cv2.experience[0].start_date, "Jan 2019");
    }

    // ── skills (R-INGEST-7) ─────────────────────────────────────────────────────
    #[test]
    fn skill_bucketing_by_source_key() {
        // R-INGEST-7: each source key → its master-CV list; default `skills`.
        let cv = mine(json!({
            "name":"N",
            "programmingLanguages": ["Rust"],
            "skills": ["Leadership"],
            "languages": ["English"],
            "tools": ["Docker"],
            "technologies": ["Kafka"],
            "toolsTechnologies": ["Terraform"],
            "asAServices": ["AWS"],
            "services": ["GCP"]
        }));
        let names = |v: &[aa_core::Skill]| v.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&cv.programming_languages), vec!["Rust"]);
        // skills default bucket also absorbs `languages`
        assert!(names(&cv.skills).contains(&"Leadership".to_string()));
        assert!(names(&cv.skills).contains(&"English".to_string()));
        let tools = names(&cv.tools_technologies);
        assert!(tools.contains(&"Docker".to_string()));
        assert!(tools.contains(&"Kafka".to_string()));
        assert!(tools.contains(&"Terraform".to_string()));
        let svcs = names(&cv.as_a_services);
        assert!(svcs.contains(&"AWS".to_string()));
        assert!(svcs.contains(&"GCP".to_string()));
    }

    #[test]
    fn skill_proficiency_honesty() {
        // R-INGEST-7 / DISCUSS-8a-3: source rating used only if integer in 1..=5, else 3.
        let cv = mine(json!({
            "name":"N",
            "skills": [
                {"name":"In4","proficiency":4},
                {"name":"In1","level":1},
                {"name":"In5","rating":5},
                {"name":"Zero","proficiency":0},
                {"name":"Seven","level":7},
                {"name":"Float","rating":4.5},
                {"name":"Word","proficiency":"expert"},
                {"name":"Bare"},
                "PlainString"
            ]
        }));
        let p = |n: &str| cv.skills.iter().find(|s| s.name == n).unwrap().proficiency;
        assert_eq!(p("In4"), 4);
        assert_eq!(p("In1"), 1);
        assert_eq!(p("In5"), 5);
        assert_eq!(p("Zero"), 3);
        assert_eq!(p("Seven"), 3);
        assert_eq!(p("Float"), 3);
        assert_eq!(p("Word"), 3);
        assert_eq!(p("Bare"), 3);
        assert_eq!(p("PlainString"), 3);
    }

    #[test]
    fn skill_proficiency_priority_order() {
        // R-INGEST-7: proficiency → level → rating (first present wins).
        let cv = mine(json!({
            "name":"N",
            "skills": [{"name":"S","proficiency":2,"level":5,"rating":1}]
        }));
        assert_eq!(cv.skills[0].proficiency, 2);
    }

    #[test]
    fn skill_object_without_name_is_dropped() {
        // honesty: a skill object with no usable name is not emitted as a blank skill.
        let cv = mine(json!({"name":"N","skills":[{"proficiency":4}, {"name":""}, "Keep"]}));
        let names: Vec<_> = cv.skills.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, vec!["Keep".to_string()]);
    }

    // ── id synthesis (R-INGEST-8) ───────────────────────────────────────────────
    #[test]
    fn synthetic_id_synthesis() {
        // R-INGEST-8: imp_exp_N / imp_exp_N_bM.
        let cv = mine(json!({
            "experience": [
                {"jobTitle":"A","businessName":"X","achievements":"b0\nb1"},
                {"jobTitle":"B","businessName":"Y","achievements":"c0"}
            ]
        }));
        assert_eq!(cv.experience[0].id, "imp_exp_0");
        assert_eq!(cv.experience[0].achievements_tasks[0].id, "imp_exp_0_b0");
        assert_eq!(cv.experience[0].achievements_tasks[1].id, "imp_exp_0_b1");
        assert_eq!(cv.experience[1].id, "imp_exp_1");
        assert_eq!(cv.experience[1].achievements_tasks[0].id, "imp_exp_1_b0");
    }

    #[test]
    fn import_is_deterministic() {
        // R-INGEST-8: same value → byte-identical serialisation.
        let v = json!({"name":"N","experience":[{"jobTitle":"T","businessName":"B"}]});
        let a = import_cv_json(&v).unwrap().to_json().unwrap();
        let b = import_cv_json(&v).unwrap().to_json().unwrap();
        assert_eq!(a, b);
    }

    // ── no-mutation (R-INGEST-9) ────────────────────────────────────────────────
    #[test]
    fn input_value_not_mutated() {
        // R-INGEST-9 / I1: the input Value is borrowed, never mutated.
        let v = json!({"name":"N","skills":["Rust"]});
        let before = v.clone();
        let _ = import_cv_json(&v).unwrap();
        assert_eq!(v, before);
    }

    // ── emission rule (R-INGEST-13) ─────────────────────────────────────────────
    #[test]
    fn experience_emitted_iff_nonempty_jobtitle() {
        // R-INGEST-13: emit iff non-empty jobTitle; absent businessName/startDate → "".
        let cv = mine(json!({
            "experience": [
                {"jobTitle":"Has Title"},
                {"businessName":"No Title Co"},
                {"jobTitle":"", "businessName":"Blank"},
                {"jobTitle":"Second"}
            ]
        }));
        // only the two with a non-empty jobTitle are emitted
        assert_eq!(cv.experience.len(), 2);
        assert_eq!(cv.experience[0].job_title, "Has Title");
        assert_eq!(cv.experience[0].business_name, "");
        assert_eq!(cv.experience[0].start_date, "");
        assert_eq!(cv.experience[1].job_title, "Second");
        // ids are dense over EMITTED experiences (0,1) — not source indices
        assert_eq!(cv.experience[0].id, "imp_exp_0");
        assert_eq!(cv.experience[1].id, "imp_exp_1");
    }

    // ── completeness (R-INGEST-11/12) ───────────────────────────────────────────
    #[test]
    fn completeness_flags_each_class() {
        // R-INGEST-11: each missing_* flag true/false, is_complete.
        // all present
        let full = mine(json!({
            "name":"N",
            "skills":["Rust"],
            "experience":[{"jobTitle":"T","businessName":"B","achievements":"did x"}]
        }));
        let r = completeness(&full, &[]);
        assert!(!r.missing_person_name);
        assert!(!r.missing_experience);
        assert!(!r.missing_achievement);
        assert!(!r.missing_skill);
        assert!(r.is_complete());

        // experience present but only jobTitle (no businessName) → missing_experience TRUE
        let part = mine(json!({"name":"N","skills":["Rust"],"experience":[{"jobTitle":"T"}]}));
        let r2 = completeness(&part, &[]);
        assert!(
            r2.missing_experience,
            "jobTitle-only does not satisfy experience"
        );
        assert!(r2.missing_achievement);
        assert!(!r2.is_complete());

        // no name, no skill
        let none = mine(json!({"experience":[{"jobTitle":"T","businessName":"B"}]}));
        let r3 = completeness(&none, &[]);
        assert!(r3.missing_person_name);
        assert!(r3.missing_skill);
        assert!(!r3.missing_experience);
    }

    #[test]
    fn completeness_lists_ignored_role_arrays() {
        // R-INGEST-12: ignored names threaded through verbatim.
        let cv = mine(json!({"name":"N","skills":["X"],
            "experience":[{"jobTitle":"T","businessName":"B","achievements":"a"}]}));
        let r = completeness(&cv, &["work".to_string(), "positions".to_string()]);
        assert_eq!(
            r.ignored_role_arrays,
            vec!["work".to_string(), "positions".to_string()]
        );
        assert!(r.is_complete(), "ignored arrays do not affect is_complete");
    }

    // ── empty gate + abuse (R-INGEST-14) ────────────────────────────────────────
    #[test]
    fn empty_gate_and_abuse_never_panics() {
        // R-INGEST-14: no name + no experience + no skill → Err(Empty); no panic on any shape.
        let empty_err = |v: Value| matches!(import_cv_json(&v), Err(ImportError::Empty));
        assert!(empty_err(json!({})));
        assert!(empty_err(json!({"notes": "hi"})));
        // hostile shapes: Ok or Err(Empty), never a panic
        for v in [
            json!([1, 2, 3]),
            json!("a bare string"),
            json!(42),
            json!(null),
            json!({"experience": "not-an-array"}),
            json!({"experience": [42, "x", {"jobTitle": "T"}]}),
            json!({"skills": {"name": "wrong-shape"}}),
            json!({"name": "   "}),
        ] {
            // must not panic; result is either Ok or Err(Empty)
            let _ = import_cv_json(&v);
        }
        // the {"experience":[42,"x",{jobTitle}]} case extracts exactly one experience
        let cv = import_cv_json(&json!({"experience": [42, "x", {"jobTitle": "T"}]})).unwrap();
        assert_eq!(cv.experience.len(), 1);
        assert_eq!(cv.experience[0].job_title, "T");
    }

    #[test]
    fn whitespace_name_alone_is_empty() {
        // R-INGEST-14: a whitespace-only name is not "recognisable content".
        let err = import_cv_json(&json!({"name": "   "})).unwrap_err();
        assert_eq!(err.to_string(), "résumé produced no recognisable content");
    }

    // ── total-helper edge arms (R-INGEST-14 robustness; coverage of every branch) ─────
    #[test]
    fn achievement_array_skips_unusable_elements() {
        // An achievement ARRAY may hold an object with no usable text key, and a
        // non-string/non-object element (number/bool/null) — both are skipped, no panic.
        let cv = mine(json!({
            "experience": [{
                "jobTitle": "T", "businessName": "B",
                "achievements": [
                    {"unrelated": "x"},   // object, no description/text/name → skipped
                    42,                    // number element → skipped
                    true,                  // bool element → skipped
                    null,                  // null element → skipped
                    "kept"                 // the only usable bullet
                ]
            }]
        }));
        assert_eq!(cv.experience[0].achievements_tasks.len(), 1);
        assert_eq!(cv.experience[0].achievements_tasks[0].description, "kept");
    }

    #[test]
    fn coerce_date_handles_float_and_huge() {
        // A non-integer numeric date (2019.5) truncates honestly; a value beyond i64 but
        // within u64 uses the u64 arm. Both via the public surface, no panic.
        let cv = mine(json!({
            "experience": [{
                "jobTitle": "T", "businessName": "B",
                "startDate": 2019.5,
                "endDate": 9223372036854775808u64  // i64::MAX + 1 → u64 arm
            }]
        }));
        assert_eq!(cv.experience[0].start_date, "2019");
        assert_eq!(
            cv.experience[0].end_date.as_deref(),
            Some("9223372036854775808")
        );
    }

    #[test]
    fn skill_blank_string_and_non_object_elements_dropped() {
        // A blank string skill and a non-string/non-object element (number) are dropped;
        // covers skill_from's empty-string and catch-all arms.
        let cv = mine(json!({
            "name": "N",
            "skills": ["  ", 7, true, null, "Keep"]
        }));
        let names: Vec<_> = cv.skills.iter().map(|s| s.name.clone()).collect();
        assert_eq!(names, vec!["Keep".to_string()]);
    }
}
