#!/usr/bin/env node
// ─────────────────────────────────────────────────────────────────────────────
// generate.js — deterministic, seedable synthetic-data generator.
//
// Emits 100% FAKE Australian personas (Master CV schema) and synthetic job ads
// (LinkedIn- and Seek-shaped). This is the testing backbone that lets a PUBLIC
// repo be built and CI-tested with NO real PII. Everything here is invented;
// emails use the reserved example.com domain, phones use the reserved 04xx range.
//
// Usage:
//   node tools/fake-data/generate.js [--seed 42] [--personas 4] [--jobs 6] [--out fixtures]
//
// Deterministic: same seed -> byte-identical output. Zero dependencies.
// ─────────────────────────────────────────────────────────────────────────────

const fs = require("fs");
const path = require("path");

// ── arg parsing ──────────────────────────────────────────────────────────────
function arg(name, def) {
  const i = process.argv.indexOf("--" + name);
  return i !== -1 && process.argv[i + 1] ? process.argv[i + 1] : def;
}
const SEED = parseInt(arg("seed", "42"), 10);
const N_PERSONAS = parseInt(arg("personas", "4"), 10);
const N_JOBS = parseInt(arg("jobs", "6"), 10);
const OUT = path.resolve(arg("out", "fixtures"));

// ── deterministic PRNG (mulberry32) ──────────────────────────────────────────
function mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
let rnd = mulberry32(SEED);
const pick = (arr) => arr[Math.floor(rnd() * arr.length)];
const int = (lo, hi) => lo + Math.floor(rnd() * (hi - lo + 1));
const sample = (arr, n) => {
  const c = [...arr];
  const out = [];
  while (out.length < n && c.length) out.push(c.splice(Math.floor(rnd() * c.length), 1)[0]);
  return out;
};

// ── synthetic pools (all invented) ───────────────────────────────────────────
const FIRST = ["Jordan", "Avery", "Riley", "Morgan", "Quinn", "Harper", "Rowan", "Sasha", "Devin", "Marlowe", "Indra", "Tobias", "Noa", "Elliot"];
const LAST = ["Calder", "Hawthorne", "Beckett", "Ashford", "Marsh", "Voss", "Linden", "Okafor", "Nakamura", "Delacroix", "Romano", "Singh", "Bramwell"];
const CITIES = ["Melbourne, Australia", "Sydney, Australia", "Brisbane, Australia", "Perth, Australia", "Adelaide, Australia", "Canberra, Australia"];

const ARCHETYPES = {
  "backend-engineer": {
    title: "Senior Backend Engineer",
    summary: "Backend engineer focused on resilient services, clean domain models, and pragmatic delivery.",
    languages: [["Python", 5], ["Go", 4], ["TypeScript", 4], ["SQL", 5], ["Rust", 2]],
    skills: [["System Design", 5], ["Distributed Systems", 4], ["Mentoring", 4], ["Incident Response", 4]],
    tools: [["Docker", 5], ["Kubernetes", 4], ["PostgreSQL", 5], ["Kafka", 3], ["Terraform", 3]],
    services: [["AWS", 4], ["GCP", 3], ["Datadog", 3]],
    roles: ["Senior Backend Engineer", "Backend Engineer", "Platform Engineer", "Software Engineer"],
    bullets: [
      "Designed and shipped a payments reconciliation service handling 4M events/day",
      "Cut p99 API latency by 38% by reworking the caching and query layer",
      "Led the migration from a monolith to three bounded-context services",
      "Introduced contract tests that eliminated a recurring integration outage class",
      "Mentored four engineers through their first on-call rotations",
    ],
  },
  "frontend-engineer": {
    title: "Frontend Engineer",
    summary: "Frontend engineer who cares about accessibility, performance budgets, and design-system rigour.",
    languages: [["TypeScript", 5], ["JavaScript", 5], ["HTML", 5], ["CSS", 5], ["Python", 2]],
    skills: [["Accessibility (WCAG)", 4], ["Design Systems", 4], ["Web Performance", 4], ["UX Collaboration", 4]],
    tools: [["React", 5], ["Vite", 4], ["Playwright", 4], ["Storybook", 4], ["Figma", 3]],
    services: [["Vercel", 4], ["Cloudflare", 3]],
    roles: ["Frontend Engineer", "UI Engineer", "Web Developer", "Software Engineer"],
    bullets: [
      "Rebuilt the checkout flow, lifting mobile conversion by 12%",
      "Drove the component library to WCAG 2.2 AA across 40+ components",
      "Reduced the main bundle by 220KB via route-level code splitting",
      "Set up visual-regression CI that caught 30+ layout regressions pre-merge",
      "Paired with design to ship a dark-mode theme with zero contrast failures",
    ],
  },
  "product-manager": {
    title: "Product Manager",
    summary: "Product manager who turns ambiguous problems into shipped, measured outcomes.",
    languages: [["SQL", 4], ["Python", 2]],
    skills: [["Product Discovery", 5], ["Stakeholder Management", 5], ["Roadmapping", 5], ["Experimentation", 4], ["Analytics", 4]],
    tools: [["Jira", 4], ["Amplitude", 4], ["Figma", 3], ["Looker", 3]],
    services: [["AWS", 2]],
    roles: ["Senior Product Manager", "Product Manager", "Associate Product Manager", "Product Owner"],
    bullets: [
      "Owned a roadmap that grew activated users 28% over three quarters",
      "Ran a discovery program that killed two low-value bets before build",
      "Defined the north-star metric and instrumented the funnel end to end",
      "Negotiated scope with engineering to ship an MVP six weeks early",
      "Stood up an experimentation cadence of two A/B tests per sprint",
    ],
  },
  "career-changer": {
    title: "Junior Software Developer",
    summary: "Career-changer from operations into software, strong on communication and fundamentals.",
    languages: [["JavaScript", 3], ["Python", 3], ["SQL", 3], ["HTML", 4], ["CSS", 4]],
    skills: [["Problem Solving", 4], ["Stakeholder Communication", 5], ["Process Improvement", 4]],
    tools: [["Git", 3], ["React", 3], ["Node.js", 3], ["Excel", 5]],
    services: [["AWS", 1]],
    roles: ["Junior Software Developer", "Operations Analyst", "Support Lead", "Coordinator"],
    bullets: [
      "Built an internal dashboard that saved the ops team ~6 hours a week",
      "Completed a 6-month immersive software course while working full time",
      "Automated a manual reporting process with a small Node script",
      "Acted as the bridge between support tickets and the engineering backlog",
      "Reduced onboarding time for new coordinators by rewriting the runbook",
    ],
  },
};

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const TAG_POOL = ["TypeScript", "Python", "AWS", "Docker", "Kubernetes", "React", "PostgreSQL", "Kafka", "Terraform", "Go", "CI/CD", "GraphQL", "Redis", "gRPC"];

function dateRange(endYear, lenMonths) {
  const endMonth = int(0, 11);
  const startTotal = endYear * 12 + endMonth - lenMonths;
  const sY = Math.floor(startTotal / 12), sM = ((startTotal % 12) + 12) % 12;
  return { start: `${MONTHS[sM]} ${sY}`, end: `${MONTHS[endMonth]} ${endYear}` };
}

// ── persona builder ──────────────────────────────────────────────────────────
function buildPersona(idx, archKey) {
  const a = ARCHETYPES[archKey];
  const name = `${pick(FIRST)} ${pick(LAST)}`;
  const handle = name.toLowerCase().replace(/[^a-z]+/g, "-");
  const skillList = (pairs) => pairs.map(([nm, p], i) => ({
    name: nm, proficiency: p, aliases: [], evidenceIds: [],
  }));

  const nJobs = int(3, 5);
  let year = 2025;
  const experience = [];
  for (let j = 0; j < nJobs; j++) {
    const len = int(12, 36);
    const { start, end } = dateRange(year, len);
    year -= Math.ceil(len / 12);
    const id = `exp_${idx}_${j}`;
    const nBul = int(2, 4);
    const chosen = sample(a.bullets, nBul);
    experience.push({
      id,
      jobTitle: pick(a.roles),
      businessName: `${pick(["Northwind", "Acme", "Helios", "Vantage", "Brightwater", "Meridian", "Cobalt"])} ${pick(["Labs", "Digital", "Group", "Systems", "Co"])}`,
      consultancy: rnd() < 0.3 ? "via Talent Partners (contract)" : "",
      location: pick(CITIES),
      employmentType: rnd() < 0.3 ? "Client" : "Employer",
      startDate: start,
      endDate: j === 0 ? "Present" : end,
      domain: pick(["Fintech", "Healthtech", "E-commerce", "GovTech", "SaaS"]),
      hide: false,
      contact: { name: "", email: "" }, // PII intentionally empty even for fakes
      tags: sample(TAG_POOL, int(3, 6)),
      achievementsTasks: chosen.map((text, bi) => ({
        id: `${id}_b${bi}`,
        description: text,
        emphasise: bi === 0,
        tags: [],
        metrics: (text.match(/\d+%|\d+M|\d+KB|\d+ hours/g) || []),
        evidenceStrength: +(0.6 + rnd() * 0.4).toFixed(2),
      })),
    });
  }

  return {
    schemaVersion: "1.0.0",
    person: {
      name,
      professionalTitle: a.title,
      professionalDescription: a.summary,
      location: pick(CITIES),
      email: `${handle}@example.com`,
      phone: `04${int(10, 99)} ${int(100, 999)} ${int(100, 999)}`,
      linkedin: `linkedin.com/in/${handle}`,
      github: `github.com/${handle}`,
      website: "",
      image: "",
    },
    headline: a.title,
    summaryVariants: [a.summary],
    programmingLanguages: skillList(a.languages),
    skills: skillList(a.skills),
    toolsTechnologies: skillList(a.tools),
    asAServices: skillList(a.services),
    experience,
    projects: [],
    education: [{
      id: `edu_${idx}_0`,
      institution: pick(["University of Melbourne", "RMIT", "University of Sydney", "QUT", "Monash University"]),
      qualification: pick(["BSc Computer Science", "BInfoTech", "BEng (Software)", "Grad Cert in Computing"]),
      field: "Computing",
      startDate: "Feb 2014", endDate: "Nov 2017",
    }],
    certifications: [], awards: [],
    preferences: {
      roleTypes: [a.title], industries: ["SaaS", "Fintech"], locations: ["Remote", "Melbourne, Australia"],
    },
  };
}

// ── job-ad builder (Normalized Job JSON per design brief) ─────────────────────
function buildJob(idx, source) {
  const role = pick(["Senior Backend Engineer", "Frontend Engineer", "Product Manager", "Platform Engineer", "Full Stack Developer"]);
  const company = `${pick(["Northwind", "Acme", "Helios", "Vantage", "Brightwater", "Meridian"])} ${pick(["Labs", "Digital", "Group", "Systems"])}`;
  const must = sample(["5+ years building production services", "Strong TypeScript or Python", "AWS or GCP experience", "CI/CD ownership", "Stakeholder management"], 3);
  const nice = sample(["Kubernetes", "Event-driven architecture", "Mentoring experience", "Fintech domain knowledge", "GraphQL"], 2);
  const tools = sample(TAG_POOL, 5);
  return {
    source,
    sourceUrl: source === "linkedin"
      ? `https://www.linkedin.com/jobs/view/${3900000000 + idx}`
      : `https://www.seek.com.au/job/${70000000 + idx}`,
    capturedAt: "2026-01-01T00:00:00Z", // fixed for determinism
    jobId: `${source}-${idx}`,
    company,
    title: role,
    location: pick(CITIES),
    employmentType: pick(["Full-time", "Contract"]),
    workModel: pick(["remote", "hybrid", "onsite"]),
    salary: { min: int(110, 140) * 1000, max: int(150, 200) * 1000, currency: "AUD" },
    descriptionRaw: `We are hiring a ${role} at ${company}. You will own delivery end to end. Required: ${must.join("; ")}. Nice to have: ${nice.join("; ")}.`,
    descriptionClean: `We are hiring a ${role} at ${company}. You will own delivery end to end.`,
    responsibilities: ["Own delivery of features end to end", "Collaborate with product and design", "Improve reliability and observability"],
    requirements: { mustHave: must, niceToHave: nice, tools, domains: ["SaaS"], senioritySignals: ["senior", "lead"] },
    keywords: tools,
    applicationMethod: source === "seek" ? "seek-quick-apply" : "external",
    recruiter: { name: "", title: "", profileUrl: "" }, // PII empty
  };
}

// ── emit ─────────────────────────────────────────────────────────────────────
function writeJson(p, obj) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, JSON.stringify(obj, null, 2) + "\n");
}

const archKeys = Object.keys(ARCHETYPES);
const manifest = { seed: SEED, personas: [], jobs: [] };

for (let i = 1; i <= N_PERSONAS; i++) {
  const archKey = archKeys[(i - 1) % archKeys.length];
  const persona = buildPersona(i, archKey);
  const rel = `personas/persona-${String(i).padStart(3, "0")}.cv.json`;
  writeJson(path.join(OUT, rel), persona);
  manifest.personas.push({ file: rel, archetype: archKey, name: persona.person.name });
}

for (let i = 1; i <= N_JOBS; i++) {
  const source = i % 2 === 0 ? "seek" : "linkedin";
  const job = buildJob(i, source);
  const rel = `jobs/job-${source}-${String(i).padStart(3, "0")}.json`;
  writeJson(path.join(OUT, rel), job);
  manifest.jobs.push({ file: rel, source, title: job.title });
}

writeJson(path.join(OUT, "manifest.json"), manifest);
console.log(`Generated ${N_PERSONAS} personas + ${N_JOBS} jobs (seed=${SEED}) -> ${OUT}`);
