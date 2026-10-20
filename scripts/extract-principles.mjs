#!/usr/bin/env node

/**
 * Maintainer-only. Build assets/principles-index.json from assets/rules/guidance.json.
 *
 * The index is the compose stage's active-principles source. Per-rule text comes from
 * the rule manifest (name, check, fix), never from prose around the rule in a
 * reference. Mode and decision tags are declared below, one decision per rule; a rule
 * with no declared decision fails the build rather than getting a guessed tag.
 *
 * Usage:
 *   node scripts/extract-principles.mjs           write assets/principles-index.json
 *   node scripts/extract-principles.mjs --check   exit 1 if the committed file is stale
 *
 * Not shipped to installed skills (absent from package.json files and
 * runtime-manifest.json include).
 */

import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const RULES_PATH = 'assets/rules/guidance.json';
const OUT_PATH = 'assets/principles-index.json';

// Prefixes that are not composition guidance:
//   EVAL  evaluation harness (scoring and judging the build, not building it)
//   LOOP  chain process gates (no code before the gate, artifact IDs, produced artifacts)
const EXCLUDED_PREFIXES = new Set(['EVAL', 'LOOP']);

// Mode families. Anything not listed applies to every mode.
const PREFIX_MODES = {
  PERS: ['persuade'],
  READ: ['read'],
  EXP: ['experience'],
  // dense.md serves operate and analyze (workflow mode table).
  DASH: ['operate', 'analyze'],
};
const ID_MODES = {
  'MODE-08': ['operate'], // operate chrome stays quiet
};

const DECISION_TYPES = [
  'pricing', 'hierarchy', 'action-placement', 'proof', 'objection-handling', 'narrative',
  'density', 'state', 'form', 'navigation', 'emphasis', 'disclosure',
];

const DECISIONS = {
  pricing: ['PERS-07', 'PERS-09', 'PERS-13'],
  proof: ['PERS-02', 'PERS-03', 'PERS-08', 'PERS-10', 'PERS-12', 'READ-05', 'DASH-19', 'HON-01', 'HON-02', 'HON-03'],
  'objection-handling': ['PERS-05'],
  'action-placement': ['PERS-01', 'ACT-09', 'ACT-12', 'ACT-13', 'MODE-04', 'EXP-03', 'CONT-06'],
  hierarchy: ['HIER-13', 'DASH-14', 'IA-04', 'IA-08', 'EXP-01', 'READ-01'],
  state: ['ACT-18', 'ACT-19', 'IA-05', 'IA-25', 'STATE-03', 'A11Y-08', 'READ-06', 'EXP-04', 'HON-04'],
  navigation: [
    'ACT-07', 'IA-10', 'IA-11', 'IA-13', 'IA-14', 'IA-15', 'IA-16', 'IA-21', 'IA-22', 'IA-23', 'IA-24',
    'DASH-15', 'DASH-16', 'DASH-17', 'READ-03',
  ],
  disclosure: ['PERS-11', 'IA-20', 'COMP-14', 'DASH-18', 'READ-04'],
  density: ['DASH-11', 'DASH-12', 'COMP-08', 'COMP-12', 'A11Y-10', 'MODE-03', 'EXP-06'],
  form: ['ACT-20', 'COMP-15', 'COMP-19', 'A11Y-07', 'FORM-01', 'FORM-02', 'FORM-03'],
  narrative: [
    'PERS-04', 'PERS-06', 'MODE-01', 'MODE-02', 'MODE-06', 'MODE-07', 'READ-02', 'EXP-05',
    'CONT-03', 'HON-05', 'HON-06',
  ],
  emphasis: [
    'HIER-04c', 'HIER-04d', 'ACT-10', 'DASH-08', 'DASH-09', 'COMP-01', 'MODE-05', 'MODE-08',
    'MODE-09', 'MODE-10', 'READ-07', 'EXP-02', 'EXP-07',
  ],
};
const DECISION_BY_ID = new Map();
for (const [type, ids] of Object.entries(DECISIONS)) {
  for (const id of ids) {
    if (DECISION_BY_ID.has(id)) throw new Error(`${id} is declared under two decisions`);
    DECISION_BY_ID.set(id, type);
  }
}

const prefixOf = id => id.split('-')[0];

// Tiers are stated only when the rule text names them (P0..P4, ranges expanded).
// No mention means the rule applies to every tier, so the field is omitted.
function tiersOf(text) {
  const tiers = new Set();
  for (const m of text.matchAll(/\bP([0-4])\s*(?:-|–|to)\s*P?([0-4])\b/g)) {
    for (let n = +m[1]; n <= +m[2]; n++) tiers.add(n);
  }
  for (const m of text.matchAll(/\bP([0-4])\b/g)) tiers.add(+m[1]);
  return [...tiers].sort().map(n => `P${n}`);
}

// Surfaces are narrowed only where the mode itself is page-shaped; otherwise omitted (all).
function surfacesOf(modes) {
  if (modes.length !== 1) return undefined;
  if (modes[0] === 'persuade' || modes[0] === 'experience') return ['page', 'flow'];
  if (modes[0] === 'read') return ['page'];
  return undefined;
}

// Where an ID is defined, by descending confidence. Section = nearest heading above.
//   1. a line that opens with the backticked ID ("`ID` (name): ...", bold or list-marked)
//   2. a rule-table row "| `ID` |", a bold lead "**... (`ID`).**", or a bracket tag "[ID]"
//   3. first other mention outside the review/diagnosis/routing references
// Within a tier the first reference file in sorted order wins.
const MENTION_ONLY_SKIP = new Set(['critique.md', 'failures.md', 'workflow.md']);

function locateDefinitions(ids) {
  const dir = join(ROOT, 'references');
  const files = readdirSync(dir).filter(f => f.endsWith('.md')).sort();
  const tiers = [new Map(), new Map(), new Map()];
  const note = (tier, id, f, section) => {
    if (!tiers[tier].has(id)) tiers[tier].set(id, { file: `references/${f}`, section });
  };
  for (const f of files) {
    let section = '';
    for (const line of readFileSync(join(dir, f), 'utf8').split('\n')) {
      const h = line.match(/^#{2,3}\s+(.*)/);
      if (h) { section = h[1].replace(/`/g, '').trim(); continue; }
      const open = line.match(/^\s*(?:[-*]\s+)?(?:\*\*)?`([A-Z0-9]+-\d{2}[a-z]?)`/);
      if (open && ids.has(open[1])) note(0, open[1], f, section);
      const row = line.match(/^\|\s*`([A-Z0-9]+-\d{2}[a-z]?)`\s*\|/);
      if (row && ids.has(row[1])) note(1, row[1], f, section);
      for (const m of line.matchAll(/\*\*[^*]*\(`([A-Z0-9]+-\d{2}[a-z]?)`\)[^*]*\*\*|\[([A-Z0-9]+-\d{2}[a-z]?)\]/g)) {
        const id = m[1] ?? m[2];
        if (ids.has(id)) note(1, id, f, section);
      }
      if (!MENTION_ONLY_SKIP.has(f)) {
        for (const m of line.matchAll(/`([A-Z0-9]+-\d{2}[a-z]?)`/g)) {
          if (ids.has(m[1])) note(2, m[1], f, section);
        }
      }
    }
  }
  const found = new Map();
  for (const id of ids) found.set(id, tiers[0].get(id) ?? tiers[1].get(id) ?? tiers[2].get(id));
  return found;
}

function build() {
  const manifest = JSON.parse(readFileSync(join(ROOT, RULES_PATH), 'utf8'));
  const version = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version;
  const rules = manifest.rules.filter(r => !EXCLUDED_PREFIXES.has(prefixOf(r.id)));

  const missing = rules.filter(r => !DECISION_BY_ID.has(r.id)).map(r => r.id);
  if (missing.length) throw new Error(`No decision declared for: ${missing.join(', ')}`);
  const stale = [...DECISION_BY_ID.keys()].filter(id => !rules.some(r => r.id === id));
  if (stale.length) throw new Error(`Decision declared for unknown rule: ${stale.join(', ')}`);

  const defs = locateDefinitions(new Set(rules.map(r => r.id)));
  const principles = rules.map(r => {
    const modes = ID_MODES[r.id] ?? PREFIX_MODES[prefixOf(r.id)] ?? ['*'];
    const guidance = `${r.check} ${r.fix}`;
    const entry = {
      id: r.id,
      modes,
      decision: DECISION_BY_ID.get(r.id),
    };
    const tiers = tiersOf(guidance);
    if (tiers.length) entry.tiers = tiers;
    const surfaces = surfacesOf(modes);
    if (surfaces) entry.surfaces = surfaces;
    entry.principle = r.name;
    entry.guidance = guidance;
    entry.severity = r.severity;
    const def = defs.get(r.id);
    entry.source = def ? { file: def.file, ...(def.section && { section: def.section }) } : { file: RULES_PATH };
    return entry;
  }).sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));

  const index = {
    $schema: 'considered-principles/v2',
    version,
    source: RULES_PATH,
    excludedPrefixes: [...EXCLUDED_PREFIXES].sort(),
    decisionTypes: DECISION_TYPES,
    principles,
  };
  return `${JSON.stringify(index, null, 2)}\n`;
}

const next = build();
const outPath = join(ROOT, OUT_PATH);

if (process.argv.includes('--check')) {
  let current = '';
  try { current = readFileSync(outPath, 'utf8'); } catch { /* missing counts as stale */ }
  if (current !== next) {
    console.error(`${OUT_PATH} is stale. Run: node scripts/extract-principles.mjs`);
    process.exit(1);
  }
  console.log(`${OUT_PATH} is current.`);
} else {
  writeFileSync(outPath, next);
  console.log(`Wrote ${OUT_PATH} (${JSON.parse(next).principles.length} principles, ${Buffer.byteLength(next)} bytes).`);
}
