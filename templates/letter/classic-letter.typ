// ─────────────────────────────────────────────────────────────────────────────
// classic-letter.typ — the cover-letter companion to classic.typ (§G).
//
// Matches the "classic" CV look (doc/design/pdf-look.md): same A4 page, narrow
// margins, Liberation Sans, ink/muted palette, thin rule. Structure (§G):
//   greeting → why-this-role/company (scaffold) → 2–3 strength paragraphs (each
//   wrapping one selected achievement, carrying its evidence id) → close.
//
// Usage (CLI parity with the CV template):
//   typst compile templates/letter/classic-letter.typ out.pdf \
//     --input data=<path-to-letter.json> --root .
//
// `data` is a path (relative to --root) to a cover-letter JSON document:
//   { greeting, whyRole, strengths:[{text, sourceEvidenceId}], closing, candidateName }
// With no --input, a built-in placeholder renders so the template compiles alone.
// ─────────────────────────────────────────────────────────────────────────────

#let placeholder = (
  greeting: "Dear Hiring Team,",
  whyRole: "Pass --input data=<path-to-letter.json> to render a real cover letter.",
  strengths: (),
  closing: "Kind regards,",
  candidateName: "Your Name",
)

#let data = {
  let p = sys.inputs.at("data", default: none)
  if p == none {
    placeholder
  } else {
    json(if p.starts-with("/") { p } else { "/" + p })
  }
}

// ── design tokens (shared with classic.typ — see doc/design/pdf-look.md) ──────
#let ink   = rgb("#1a1a1a")
#let muted = rgb("#5a5a5a")
#let faint = rgb("#9a9a9a")
#let rule  = rgb("#d8d8d8")

#set document(title: "Cover Letter", author: data.at("candidateName", default: ""))
// ── item 8b: SAMPLE watermark (see templates/cv/classic.typ for the contract) ──
#let isSample = sys.inputs.at("samples", default: "false") == "true"
#let sampleWatermark = "[SAMPLE — REPLACE BEFORE SENDING]"
#let watermarkBox = if isSample {
  place(center + horizon, rotate(-30deg, text(
    size: 40pt, fill: rgb(220, 60, 60, 36), weight: 700,
  )[#sampleWatermark]))
}
#set page(paper: "a4", margin: (x: 22pt, y: 26pt), background: watermarkBox)
#if isSample {
  text(size: 7pt, fill: rgb(220, 60, 60))[#sampleWatermark]
}
#set text(
  font: ("Liberation Sans", "Helvetica Neue", "Arial", "DejaVu Sans"),
  size: 10.5pt, fill: ink,
)
// item 9: tightened leading/spacing so greeting + why-role + bulleted strengths +
// close reliably fit one A4 page (content is bounded in build_cover_letter too).
#set par(justify: false, leading: 0.55em, spacing: 6pt)

// ── header: candidate name + rule ────────────────────────────────────────────
// item 9: 14pt header (down from 18pt) to reclaim vertical space.
#text(size: 14pt, weight: 700)[#data.at("candidateName", default: "")]
#v(3pt)
#line(length: 100%, stroke: 0.5pt + rule)
#v(6pt)

// ── greeting (scaffold) ──────────────────────────────────────────────────────
#text(size: 10.5pt)[#data.at("greeting", default: "")]
#v(4pt)

// ── why this role/company (scaffold) ─────────────────────────────────────────
#block(below: 6pt)[#text(fill: ink)[#data.at("whyRole", default: "")]]

// ── strengths — a compact bulleted list (each carries its evidence id, faint) ─
// item 9: render strengths as a tight bulleted list rather than full paragraphs.
#block(below: 6pt)[
  #list(
    spacing: 5pt,
    indent: 2pt,
    body-indent: 4pt,
    ..data.at("strengths", default: ()).map(s => [
      #text(size: 10.5pt)[#s.at("text", default: "")]
      #if s.at("sourceEvidenceId", default: "") != "" [
        #h(4pt)#text(size: 7pt, fill: faint)[[evidence: #s.sourceEvidenceId]]
      ]
    ])
  )
]

// ── close ────────────────────────────────────────────────────────────────────
#v(2pt)
#text(fill: muted)[#data.at("closing", default: "")]
