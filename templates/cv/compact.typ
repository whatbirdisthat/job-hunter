// ─────────────────────────────────────────────────────────────────────────────
// compact.typ — the "compact" job-hunter CV template (item #6, capability A).
//
// A SINGLE-COLUMN, ATS-friendly linear layout: a full-width header, then a linear
// sequence of standard sections — professional summary → skills (as inline grouped
// lists `Label: A · B · C`, NO rating dots/circles, which read as images to an ATS)
// → experience. The deliberate contrast with classic.typ's two-column sidebar grid
// is what the ATS column-reliance check (R-ATS-3) keys off.
//
// Same `sys.inputs.data` JSON contract + built-in placeholder block + bundled
// Liberation font stack as classic.typ. Design tokens documented in
// doc/design/pdf-look.md.
//
// Usage:
//   typst compile templates/cv/compact.typ out.pdf \
//     --input data=fixtures/personas/persona-001.cv.json --root .
//
// `data` is a path (relative to --root) to a Master CV JSON document conforming to
// doc/schemas/master-cv.schema.json. With no --input, a built-in placeholder
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

#set document(
  title: data.person.at("name", default: "Curriculum Vitae"),
  author: data.person.at("name", default: ""),
)
// ── item 8b: SAMPLE watermark (see classic.typ for the contract) ─────────────
#let isSample = sys.inputs.at("samples", default: "false") == "true"
#let sampleWatermark = "[SAMPLE — REPLACE BEFORE SENDING]"
#let watermarkBox = if isSample {
  place(center + horizon, rotate(-30deg, text(
    size: 40pt, fill: rgb(220, 60, 60, 36), weight: 700,
  )[#sampleWatermark]))
}
#set page(paper: "a4", margin: (x: 16pt, y: 22pt), background: watermarkBox)
#if isSample {
  text(size: 7pt, fill: rgb(220, 60, 60))[#sampleWatermark]
}
#set text(
  font: ("Liberation Sans", "Helvetica Neue", "Arial", "DejaVu Sans"),
  size: 10pt, fill: ink,
)
#set par(justify: false, leading: 0.62em)

// ── helpers ──────────────────────────────────────────────────────────────────
// A standard section heading: bold, with a thin rule under it (ATS-readable text,
// no graphics). Heading vocabulary is fixed and ⊆ the ATS standard allow-list.
#let sectionHead(title) = {
  block(below: 5pt, above: 10pt)[
    #text(weight: 700, size: 11pt)[#title]
    #v(2pt)
    #line(length: 100%, stroke: 0.5pt + rule)
  ]
}

// Skills as an inline grouped list: `Label: A · B · C` — NO rating dots (an image-ish
// smell to an ATS); plain text maximises extractability.
#let skillLine(label, items) = {
  if items.len() == 0 { return }
  let names = items.map(s => s.name)
  block(below: 3pt)[
    #text(weight: 700, size: 9.5pt)[#label: ]
    #text(size: 9.5pt)[#names.join("  ·  ")]
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
          #text(size: 9.5pt)[#sym.bullet #h(3pt) #a.description]
        ]
      }
    }
    #let tags = e.at("tags", default: ())
    #if tags.len() > 0 [
      #v(1pt)
      #text(size: 8pt, fill: faint)[#tags.join("  ·  ")]
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

// ── single-column linear body: summary → skills → experience ─────────────────
#let desc = data.person.at("professionalDescription", default: "")
#if desc != "" {
  sectionHead("Summary")
  block(below: 4pt)[#text(size: 10pt)[#desc]]
}

#let pl = data.at("programmingLanguages", default: ())
#let sk = data.at("skills", default: ())
#let tt = data.at("toolsTechnologies", default: ())
#let av = data.at("asAServices", default: ())
#if pl.len() + sk.len() + tt.len() + av.len() > 0 {
  sectionHead("Skills")
  skillLine("Languages", pl)
  skillLine("Skills", sk)
  skillLine("Tools & Technologies", tt)
  skillLine("Platforms & Services", av)
}

#sectionHead("Experience")
#for e in data.at("experience", default: ()) { jobEntry(e) }
