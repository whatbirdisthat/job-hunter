//! Shared test support: synthesise résumé files from a persona AT TEST TIME (I4 —
//! no committed binary fixtures). DOCX via the `docx-rs` dev-dep; PDF via the
//! project's `typst` CLI rendering `templates/cv/classic.typ`.

#![allow(dead_code)] // each test binary uses a subset of these helpers

use std::io::Cursor;
use std::path::PathBuf;

/// Repo root (…/crates/cvimport/ → ../..).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Load a persona master CV (the test oracle) as parsed JSON.
pub fn persona_json(name: &str) -> serde_json::Value {
    let p = repo_root().join("fixtures/personas").join(name);
    let s = std::fs::read_to_string(p).unwrap();
    serde_json::from_str(&s).unwrap()
}

/// Render a persona to a PDF byte vector via the `typst` CLI (the same path the
/// `foundation` CI smoke uses). Deterministic; offline.
pub fn render_persona_pdf(name: &str) -> Vec<u8> {
    let root = repo_root();
    let out = root.join(format!(
        "target/cvimport-it-{}-{}.pdf",
        name,
        std::process::id()
    ));
    let status = std::process::Command::new("typst")
        .arg("compile")
        .arg(root.join("templates/cv/classic.typ"))
        .arg(&out)
        .arg("--input")
        .arg(format!("data=fixtures/personas/{name}"))
        .arg("--root")
        .arg(&root)
        .status()
        .expect("typst CLI runs");
    assert!(status.success(), "typst compile failed for {name}");
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    bytes
}

/// Synthesise a résumé-shaped DOCX from a persona, AT TEST TIME, via `docx-rs`.
/// Lays out: name / title / each skill section (label paragraph + one comma-joined
/// list paragraph) / "Experience" / per-job (`<title> <start> – <end>`,
/// `<business> · <location>`, one paragraph per achievement). Mirrors the
/// classic.typ layout so the same segmenter recovers both formats.
pub fn synth_persona_docx(name: &str) -> Vec<u8> {
    use docx_rs::*;
    let cv = persona_json(name);
    let person = &cv["person"];

    let mut docx = Docx::new();
    let para = |text: String| Paragraph::new().add_run(Run::new().add_text(text));

    docx = docx.add_paragraph(para(string(&person["name"])));
    docx = docx.add_paragraph(para(string(&person["professionalTitle"])));

    let skill_sections = [
        ("Languages", "programmingLanguages"),
        ("Skills", "skills"),
        ("Tools & Technologies", "toolsTechnologies"),
        ("Platforms & Services", "asAServices"),
    ];
    for (label, key) in skill_sections {
        if let Some(arr) = cv[key].as_array() {
            if arr.is_empty() {
                continue;
            }
            let names: Vec<String> = arr.iter().map(|s| string(&s["name"])).collect();
            docx = docx.add_paragraph(para(label.to_string()));
            docx = docx.add_paragraph(para(names.join(", ")));
        }
    }

    docx = docx.add_paragraph(para("Experience".to_string()));
    if let Some(exps) = cv["experience"].as_array() {
        for e in exps {
            let end = e["endDate"].as_str().unwrap_or("Present");
            let job_line = format!(
                "{} {} – {}",
                string(&e["jobTitle"]),
                string(&e["startDate"]),
                end
            );
            docx = docx.add_paragraph(para(job_line));
            let loc = e["location"].as_str().unwrap_or("");
            let biz_line = if loc.is_empty() {
                string(&e["businessName"])
            } else {
                format!("{} · {}", string(&e["businessName"]), loc)
            };
            docx = docx.add_paragraph(para(biz_line));
            if let Some(achs) = e["achievementsTasks"].as_array() {
                for a in achs {
                    docx = docx.add_paragraph(para(string(&a["description"])));
                }
            }
        }
    }

    let mut buf = Cursor::new(Vec::new());
    docx.build().pack(&mut buf).unwrap();
    buf.into_inner()
}

fn string(v: &serde_json::Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}
