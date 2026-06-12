//! Map [`Segments`](crate::segment::Segments) → a NEW [`aa_core::MasterCv`]
//! (R-CVI-3..7, R-CVI-9). Pure, deterministic. Assigns synthetic stable ids
//! (`imp_exp_N`, `imp_exp_N_bM` — R-CVI-6) and only emits schema-required/known
//! fields so the output validates (R-CVI-7); unknown fields are omitted, never
//! invented (I3).

use crate::segment::{Segments, SkillBucket};
use aa_core::{Achievement, Experience, MasterCv, Person, Skill};

/// Default proficiency for an imported skill. The résumé carries no rating, but the
/// schema requires `proficiency` to be an integer 1–5 (see master-cv.schema.json), so
/// we assign a neutral mid-scale 3 — honest ("unrated/imported, neutral"), never an
/// inflated rating (I3). The user re-rates during review.
const IMPORTED_PROFICIENCY: u8 = 3;

/// A placeholder start date for an experience block whose date tail could not be
/// parsed. The schema requires `startDate`; an empty string keeps the output valid
/// without inventing a real date.
const UNKNOWN_START_DATE: &str = "";

fn skills(names: Vec<String>) -> Vec<Skill> {
    names
        .into_iter()
        .map(|name| Skill {
            name,
            proficiency: IMPORTED_PROFICIENCY,
            aliases: Vec::new(),
            evidence_ids: Vec::new(),
        })
        .collect()
}

/// Map segments into a fresh `MasterCv`. Always returns a schema-valid document
/// (required fields present) — even a sparse résumé yields a valid, mostly-empty CV.
pub(crate) fn to_master_cv(seg: Segments) -> MasterCv {
    let person = Person {
        name: seg.name,
        professional_title: seg.title.clone(),
        professional_description: None,
        location: None,
        email: None,
        phone: None,
        linkedin: None,
        github: None,
        website: None,
        image: None,
    };

    let mut cv = MasterCv {
        schema_version: "1.0.0".to_string(),
        person,
        headline: seg.title,
        summary_variants: Vec::new(),
        programming_languages: Vec::new(),
        skills: Vec::new(),
        tools_technologies: Vec::new(),
        as_a_services: Vec::new(),
        experience: Vec::new(),
        projects: Vec::new(),
        education: Vec::new(),
        certifications: Vec::new(),
        awards: Vec::new(),
        preferences: None,
    };

    for (bucket, names) in seg.skills {
        let list = skills(names);
        match bucket {
            SkillBucket::ProgrammingLanguages => cv.programming_languages.extend(list),
            SkillBucket::Skills => cv.skills.extend(list),
            SkillBucket::ToolsTechnologies => cv.tools_technologies.extend(list),
            SkillBucket::AsAServices => cv.as_a_services.extend(list),
        }
    }

    for (n, block) in seg.experience.into_iter().enumerate() {
        let exp_id = format!("imp_exp_{n}");
        let achievements_tasks = block
            .bullets
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
            .collect();

        let start_date = if block.start_date.is_empty() {
            UNKNOWN_START_DATE.to_string()
        } else {
            block.start_date
        };

        cv.experience.push(Experience {
            id: exp_id,
            job_title: block.job_title,
            business_name: block.business_name,
            consultancy: None,
            location: block.location,
            employment_type: None,
            start_date,
            end_date: block.end_date,
            domain: None,
            hide: None,
            contact: None,
            tags: Vec::new(),
            achievements_tasks,
        });
    }

    cv
}

#[cfg(test)]
mod tests {
    //! L1 — map unit tests: Segments → MasterCv field mapping + synthetic ids.
    use super::*;
    use crate::segment::{ExperienceBlock, Segments, SkillBucket};

    fn seg() -> Segments {
        Segments {
            name: Some("Devin Voss".into()),
            title: Some("Senior Backend Engineer".into()),
            skills: vec![
                (
                    SkillBucket::ProgrammingLanguages,
                    vec!["Python".into(), "Go".into()],
                ),
                (SkillBucket::AsAServices, vec!["AWS".into()]),
            ],
            experience: vec![ExperienceBlock {
                job_title: "Backend Engineer".into(),
                business_name: "Acme Co".into(),
                start_date: "Nov 2022".into(),
                end_date: None,
                location: Some("Sydney".into()),
                bullets: vec!["Cut latency".into(), "Added tests".into()],
            }],
        }
    }

    #[test]
    fn maps_person_and_headline() {
        // R-CVI-3
        let cv = to_master_cv(seg());
        assert_eq!(cv.person.name.as_deref(), Some("Devin Voss"));
        assert_eq!(
            cv.person.professional_title.as_deref(),
            Some("Senior Backend Engineer")
        );
        assert_eq!(cv.headline.as_deref(), Some("Senior Backend Engineer"));
        assert_eq!(cv.schema_version, "1.0.0");
    }

    #[test]
    fn maps_skills_into_correct_lists_with_default_proficiency() {
        // R-CVI-4
        let cv = to_master_cv(seg());
        assert_eq!(cv.programming_languages.len(), 2);
        assert_eq!(cv.programming_languages[0].name, "Python");
        assert_eq!(cv.programming_languages[0].proficiency, 3);
        assert_eq!(cv.as_a_services.len(), 1);
        assert_eq!(cv.as_a_services[0].name, "AWS");
        assert!(cv.skills.is_empty());
        assert!(cv.tools_technologies.is_empty());
    }

    #[test]
    fn maps_experience_and_achievements() {
        // R-CVI-5
        let cv = to_master_cv(seg());
        assert_eq!(cv.experience.len(), 1);
        let e = &cv.experience[0];
        assert_eq!(e.job_title, "Backend Engineer");
        assert_eq!(e.business_name, "Acme Co");
        assert_eq!(e.start_date, "Nov 2022");
        assert_eq!(e.location.as_deref(), Some("Sydney"));
        assert_eq!(e.achievements_tasks.len(), 2);
        assert_eq!(e.achievements_tasks[0].description, "Cut latency");
    }

    #[test]
    fn assigns_deterministic_synthetic_ids() {
        // R-CVI-6: imp_exp_N / imp_exp_N_bM
        let mut s = seg();
        s.experience.push(ExperienceBlock {
            job_title: "Earlier".into(),
            business_name: "Beta".into(),
            start_date: "Jan 2019".into(),
            end_date: Some("Dec 2020".into()),
            location: None,
            bullets: vec!["did x".into()],
        });
        let cv = to_master_cv(s);
        assert_eq!(cv.experience[0].id, "imp_exp_0");
        assert_eq!(cv.experience[0].achievements_tasks[0].id, "imp_exp_0_b0");
        assert_eq!(cv.experience[0].achievements_tasks[1].id, "imp_exp_0_b1");
        assert_eq!(cv.experience[1].id, "imp_exp_1");
        assert_eq!(cv.experience[1].achievements_tasks[0].id, "imp_exp_1_b0");
    }

    #[test]
    fn id_assignment_is_deterministic_across_runs() {
        // R-CVI-6 / I5: same input → identical ids + identical serialisation
        let a = to_master_cv(seg()).to_json().unwrap();
        let b = to_master_cv(seg()).to_json().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn sparse_segments_still_yield_required_fields() {
        // R-CVI-7: only a name → still schemaVersion + person + (empty) experience
        let cv = to_master_cv(Segments {
            name: Some("Solo".into()),
            ..Default::default()
        });
        assert_eq!(cv.schema_version, "1.0.0");
        assert_eq!(cv.person.name.as_deref(), Some("Solo"));
        assert!(cv.experience.is_empty());
        assert_eq!(cv.headline, None);
    }

    #[test]
    fn unknown_start_date_is_empty_not_invented() {
        // a block whose date tail was unparseable still produces a valid (empty) startDate
        let cv = to_master_cv(Segments {
            name: Some("N".into()),
            experience: vec![ExperienceBlock {
                job_title: "Eng".into(),
                business_name: "Biz".into(),
                start_date: String::new(),
                end_date: None,
                location: None,
                bullets: vec![],
            }],
            ..Default::default()
        });
        assert_eq!(cv.experience[0].start_date, "");
    }
}
