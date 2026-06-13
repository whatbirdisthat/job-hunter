// ─────────────────────────────────────────────────────────────────────────────
// classic.typ — the "classic" job-hunter CV template.
//
// Faithfully re-authors the finished look of the original DW_CV app (a React SPA
// printed to PDF) as a deterministic Typst document: a left skills/ratings
// sidebar beside a right work-experience column, pt typography, per-job
// keep-together, monospace micro-text for achievements, emphasised-italic
// highlights. Design tokens are documented in doc/design/pdf-look.md.
//
// Usage:
//   typst compile templates/cv/classic.typ out.pdf \
//     --input data=fixtures/personas/persona-001.cv.json --root .
//
// `data` is a path (relative to --root) to a Master CV JSON document conforming
// to doc/schemas/master-cv.schema.json. With no --input, a built-in placeholder
// renders so the template is always compilable in isolation.
// ─────────────────────────────────────────────────────────────────────────────

#let placeholder = (
  schemaVersion: "1.0.0",
  person: (
    name: "Your Name",
    professionalTitle: "Your Professional Title",
    professionalDescription: "Pass --input data=<path-to-master-cv.json> to render a real document.",
    location: "City, Country",
    email: "you@example.com", linkedin: "", github: "", phone: "", website: "",
  ),
  programmingLanguages: (), skills: (), toolsTechnologies: (), asAServices: (),
  experience: (),
)

#let data = {
  let p = sys.inputs.at("data", default: none)
  if p == none {
    placeholder
  } else {
    // Resolve the input path relative to --root (leading slash), so callers can
    // pass repo-relative paths like fixtures/personas/persona-001.cv.json.
    json(if p.starts-with("/") { p } else { "/" + p })
  }
}

// ── design tokens (see doc/design/pdf-look.md) ───────────────────────────────
#let ink        = rgb("#1a1a1a")
#let muted      = rgb("#5a5a5a")
#let faint      = rgb("#9a9a9a")
#let rule       = rgb("#d8d8d8")
#let dotOn      = rgb("#2a2a2a")
#let dotOff     = rgb("#d0d0d0")
#let sidebarW   = 32%

#set document(
  title: data.person.at("name", default: "Curriculum Vitae"),
  author: data.person.at("name", default: ""),
)
// ── item 8b: SAMPLE watermark ────────────────────────────────────────────────
// A SECOND input `samples` (default "false") drives a visible, repeated overlay so a
// sample document cannot be mistaken for a finished one. The text is the exact sentinel
// `aa_core::samples::SAMPLE_WATERMARK`; a render-level test asserts it is extractable.
#let isSample = sys.inputs.at("samples", default: "false") == "true"
#let sampleWatermark = "[SAMPLE — REPLACE BEFORE SENDING]"
#let watermarkBox = if isSample {
  place(center + horizon, rotate(-30deg, text(
    size: 40pt, fill: rgb(220, 60, 60, 36), weight: 700,
  )[#sampleWatermark]))
}
#set page(paper: "a4", margin: (x: 14pt, y: 20pt), background: watermarkBox)
// Also emit the sentinel once at the top of the flow so text extraction is robust
// across rotated-background quirks (the guarantee is "the text is present", R-INGEST-CLI-5).
#if isSample {
  text(size: 7pt, fill: rgb(220, 60, 60))[#sampleWatermark]
}
#set text(
  font: ("Liberation Sans", "Helvetica Neue", "Arial", "DejaVu Sans"),
  size: 10pt, fill: ink,
)
#set par(justify: false, leading: 0.62em)

// ── helpers ──────────────────────────────────────────────────────────────────
#let rating(n) = box(baseline: 1pt)[#{
  for i in range(5) {
    box(circle(radius: 2pt, fill: if i < n { dotOn } else { dotOff }, stroke: none))
    h(1.5pt)
  }
}]

#let skillBlock(title, items) = {
  if items.len() == 0 { return }
  block(breakable: false, below: 10pt)[
    #text(weight: 700, size: 10pt)[#title]
    #v(3pt)
    #for s in items {
      block(below: 3pt)[
        #grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
          text(size: 9pt)[#s.name], rating(s.at("proficiency", default: 0)))
      ]
    }
  ]
}

#let contactLine(person) = {
  let parts = ()
  for k in ("location", "email", "phone", "linkedin", "github", "website") {
    let v = person.at(k, default: "")
    if v != "" { parts.push(v) }
  }
  if parts.len() > 0 {
    text(size: 8.5pt, fill: muted)[#parts.join("  ·  ")]
  }
}

#let jobEntry(e) = {
  if e.at("hide", default: false) { return }
  block(breakable: false, below: 9pt)[
    #grid(columns: (1fr, auto), align: (left, right + horizon),
      text(weight: 700, size: 10.5pt)[#e.jobTitle],
      text(size: 8.5pt, fill: muted)[#e.at("startDate", default: "") #if e.at("endDate", default: "") != "" [– #e.endDate]])
    #text(size: 9pt, fill: muted)[
      #e.businessName#if e.at("consultancy", default: "") != "" [ · #e.consultancy]#if e.at("location", default: "") != "" [ · #e.location]
    ]
    #v(2pt)
    #for a in e.at("achievementsTasks", default: ()) {
      if a.at("emphasise", default: false) {
        block(below: 3pt)[#text(style: "italic", size: 9.5pt)[#a.description]]
      } else {
        block(below: 2pt)[
          #text(font: ("Liberation Mono", "DejaVu Sans Mono", "Courier New"), size: 7.5pt, fill: rgb("#333"))[
            #sym.triangle.r.small #h(2pt) #a.description
          ]
        ]
      }
    }
    #let tags = e.at("tags", default: ())
    #if tags.len() > 0 [
      #v(1pt)
      #text(size: 7.5pt, fill: faint)[#tags.join("  ")]
    ]
  ]
}

// ── header (full width) ──────────────────────────────────────────────────────
#text(size: 22pt, weight: 700)[#data.person.at("name", default: "")]
#v(-4pt)
#text(size: 12pt, fill: muted)[#data.person.at("professionalTitle", default: "")]
#v(3pt)
#contactLine(data.person)
#v(5pt)
#line(length: 100%, stroke: 0.5pt + rule)
#v(6pt)

// ── two-column body: skills sidebar | experience column ──────────────────────
#grid(
  columns: (sidebarW, 1fr),
  gutter: 18pt,

  // left: skills sidebar
  {
    let desc = data.person.at("professionalDescription", default: "")
    if desc != "" {
      block(below: 12pt)[#text(size: 8.5pt, fill: muted)[#desc]]
    }
    skillBlock("Languages", data.at("programmingLanguages", default: ()))
    skillBlock("Skills", data.at("skills", default: ()))
    skillBlock("Tools & Technologies", data.at("toolsTechnologies", default: ()))
    skillBlock("Platforms & Services", data.at("asAServices", default: ()))
  },

  // right: work experience
  {
    text(weight: 700, size: 12pt)[Experience]
    v(4pt)
    for e in data.at("experience", default: ()) { jobEntry(e) }
  },
)
