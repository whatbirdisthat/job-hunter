//! L4 system — the résumé-import command path (R-CVI-9, R-CVI-10). Drives the
//! ACTUAL command layer (`Session::import_resume`) → review JSON → routes through
//! `import_master_cv` to install; asserts the installed master CV is present and
//! that import NEVER mutates a loaded master CV (I1). Bad kind / garbage → typed
//! `CommandError`, never a panic. DOCX synthesised + PDF rendered at test time
//! (I4: no committed binary fixtures). Each path records a perf sample.

use aa_desktop::{CommandError, Session};
use std::io::Cursor;
use std::time::Instant;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn persona_json(name: &str) -> serde_json::Value {
    let s = std::fs::read_to_string(root().join("fixtures/personas").join(name)).unwrap();
    serde_json::from_str(&s).unwrap()
}

fn render_pdf(name: &str) -> Vec<u8> {
    let r = root();
    let out = r.join(format!(
        "target/l4-resume-{}-{}.pdf",
        name,
        std::process::id()
    ));
    let status = std::process::Command::new("typst")
        .arg("compile")
        .arg(r.join("templates/cv/classic.typ"))
        .arg(&out)
        .arg("--input")
        .arg(format!("data=fixtures/personas/{name}"))
        .arg("--root")
        .arg(&r)
        .status()
        .expect("typst runs");
    assert!(status.success());
    let b = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    b
}

fn synth_docx(name: &str) -> Vec<u8> {
    use docx_rs::*;
    let cv = persona_json(name);
    let p = &cv["person"];
    let mut d = Docx::new();
    let para = |t: String| Paragraph::new().add_run(Run::new().add_text(t));
    let s = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_string();
    d = d.add_paragraph(para(s(&p["name"])));
    d = d.add_paragraph(para(s(&p["professionalTitle"])));
    for (label, key) in [
        ("Languages", "programmingLanguages"),
        ("Skills", "skills"),
        ("Tools & Technologies", "toolsTechnologies"),
        ("Platforms & Services", "asAServices"),
    ] {
        if let Some(arr) = cv[key].as_array() {
            if arr.is_empty() {
                continue;
            }
            d = d.add_paragraph(para(label.to_string()));
            d = d.add_paragraph(para(
                arr.iter()
                    .map(|x| s(&x["name"]))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }
    d = d.add_paragraph(para("Experience".to_string()));
    if let Some(exps) = cv["experience"].as_array() {
        for e in exps {
            let end = e["endDate"].as_str().unwrap_or("Present");
            d = d.add_paragraph(para(format!(
                "{} {} – {}",
                s(&e["jobTitle"]),
                s(&e["startDate"]),
                end
            )));
            let loc = e["location"].as_str().unwrap_or("");
            d = d.add_paragraph(para(if loc.is_empty() {
                s(&e["businessName"])
            } else {
                format!("{} · {}", s(&e["businessName"]), loc)
            }));
            if let Some(achs) = e["achievementsTasks"].as_array() {
                for a in achs {
                    d = d.add_paragraph(para(s(&a["description"])));
                }
            }
        }
    }
    let mut buf = Cursor::new(Vec::new());
    d.build().pack(&mut buf).unwrap();
    buf.into_inner()
}

#[test]
fn import_resume_docx_returns_review_json_and_installs_via_existing_path() {
    // R-CVI-10: command returns review JSON; installation reuses import_master_cv.
    let docx = synth_docx("persona-001.cv.json");
    let s = Session::new();
    let t0 = Instant::now();
    let review = s.import_resume(&docx, "docx").expect("docx import");
    eprintln!("[L4 perf] Session::import_resume(docx): {:?}", t0.elapsed());

    // the returned JSON is a real master CV with the recovered name
    let parsed: serde_json::Value = serde_json::from_str(&review).unwrap();
    assert_eq!(parsed["person"]["name"], "Devin Voss");

    // install it through the EXISTING validated path (no duplicate validation)
    let mut install = Session::new();
    install
        .import_master_cv(&review)
        .expect("review JSON installs via slice-1 validation");
}

#[test]
fn import_resume_pdf_path_through_command_layer() {
    // R-CVI-1/10 — the PDF path also returns installable review JSON.
    let pdf = render_pdf("persona-001.cv.json");
    let s = Session::new();
    let t0 = Instant::now();
    let review = s.import_resume(&pdf, "pdf").expect("pdf import");
    eprintln!("[L4 perf] Session::import_resume(pdf): {:?}", t0.elapsed());
    let mut install = Session::new();
    install.import_master_cv(&review).expect("installs");
}

#[test]
fn import_never_mutates_an_installed_master_cv() {
    // R-CVI-9 / I1 — install master CV A; import a DIFFERENT résumé; A is untouched.
    // Finding 6: prove "A unmutated" by byte-equality of A's observable JSON BEFORE and
    // AFTER the import — not merely that B was never installed.
    const JD: &str = "We are hiring a Senior Backend Engineer at Acme Group. Required: \
                      Strong TypeScript or Python; Stakeholder management; AWS or GCP. \
                      Nice to have: GraphQL; Fintech domain knowledge.";

    let mut s = Session::new();
    let cv_a =
        std::fs::read_to_string(root().join("fixtures/personas/persona-002.cv.json")).unwrap();
    s.import_master_cv(&cv_a).unwrap();
    s.parse_job(JD).unwrap();

    // Snapshot A's full observable content (the tailored view's CV is a deterministic
    // function of the installed master + job) BEFORE importing a different résumé.
    let a_before = s.tailored_view().unwrap().cv.to_json().unwrap();

    // import a résumé built from a DIFFERENT persona — returns review JSON only,
    // does NOT touch the installed master CV (the method takes &self).
    let docx_b = synth_docx("persona-001.cv.json");
    let review_b = s.import_resume(&docx_b, "docx").unwrap();
    let parsed_b: serde_json::Value = serde_json::from_str(&review_b).unwrap();
    assert_eq!(parsed_b["person"]["name"], "Devin Voss");

    // A is byte-for-byte unchanged after the import: nothing about the installed master CV
    // moved (not just "B not installed").
    let a_after = s.tailored_view().unwrap().cv.to_json().unwrap();
    assert_eq!(
        a_before, a_after,
        "import_resume must not mutate the installed master CV A"
    );

    // and B was not installed: the installed master is still A, not B (Devin Voss).
    let a_name = serde_json::from_str::<serde_json::Value>(&a_after).unwrap()["person"]["name"]
        .as_str()
        .map(str::to_string);
    assert_ne!(
        a_name.as_deref(),
        Some("Devin Voss"),
        "the installed master CV must remain A, never the imported B"
    );
}

#[test]
fn import_resume_does_not_panic_on_expanding_lowercase_title_chars() {
    // Finding 1 (CRITICAL) at the command boundary: a DOCX whose experience job-title line
    // begins with `ẞ` (U+1E9E → "ss") or `İ` (U+0130 → "i̇") — chars whose lowercase has a
    // different byte length — must yield a typed Ok review JSON, never a UTF-8 slice panic.
    use docx_rs::*;
    use std::io::Cursor;
    let para = |t: &str| Paragraph::new().add_run(Run::new().add_text(t));
    for prefix in ["ẞé ", "İ "] {
        let mut buf = Cursor::new(Vec::new());
        Docx::new()
            .add_paragraph(para("Devin Voss"))
            .add_paragraph(para("Engineer"))
            .add_paragraph(para("Experience"))
            .add_paragraph(para(&format!("{prefix}Engineer Jan 2020 – Present")))
            .add_paragraph(para("Acme Co · Sydney"))
            .add_paragraph(para("Did a thing"))
            .build()
            .pack(&mut buf)
            .unwrap();
        let s = Session::new();
        let review = s
            .import_resume(&buf.into_inner(), "docx")
            .unwrap_or_else(|e| panic!("import_resume Ok for prefix {prefix:?}, got {e:?}"));
        let parsed: serde_json::Value = serde_json::from_str(&review).unwrap();
        assert_eq!(parsed["person"]["name"], "Devin Voss");
    }
}

#[test]
fn unknown_kind_returns_typed_command_error_not_panic() {
    // R-CVI-8/10 — an unsupported kind at the boundary → typed CommandError.
    let s = Session::new();
    let err = s.import_resume(b"anything", "xlsx").unwrap_err();
    assert!(matches!(err, CommandError::Import(_)), "got {err:?}");
    assert!(err.to_string().contains("unsupported résumé kind"));
}

#[test]
fn garbage_bytes_return_typed_command_error_not_panic() {
    // R-CVI-8/10 — undecodable bytes → typed CommandError, never a panic.
    let s = Session::new();
    let err = s.import_resume(b"not a pdf at all", "pdf").unwrap_err();
    assert!(matches!(err, CommandError::Import(_)), "got {err:?}");
}
