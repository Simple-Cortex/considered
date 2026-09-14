#!/usr/bin/env node
/**
 * Validate the machine-readable facts in a CONSIDERED-CONTRACT block.
 * This checker proves internal consistency only. Visual rank, responsive
 * behavior, and state behavior are verified through REVIEW-PACKET.md and the
 * independent reviewer, never inferred from a text contract.
 */
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { loadRuleCatalog, severityFor, summarizeFindings } from './lib/rules.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
const REQUIRED = ['THESIS', 'DECISION', 'MODE', 'QUESTIONS', 'ZONES', 'HIERARCHY', 'ACTIONS', 'DELETED', 'ROLL'];
const MODES = ['persuade', 'operate', 'analyze', 'read', 'experience'];
const BANNED_HEADINGS = new Set(['overview', 'metrics', 'kpis', 'charts', 'tables', 'other', 'analytics', 'data', 'stats', 'misc']);
const args = process.argv.slice(2);
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node lint-contract.mjs <file> [--json] [--gate]\n\nValidates the machine-readable facts in a CONSIDERED-CONTRACT block for\ninternal consistency only (hierarchy, zones, questions, actions, roll\nprovenance). Visual rank, responsive behavior, and state behavior are\nverified through REVIEW-PACKET.md and the independent reviewer, never\ninferred from a text contract.');
  process.exit(0);
}
const json = args.includes('--json');
const gate = args.includes('--gate');
const positional = args.filter(arg => !['--json', '--gate'].includes(arg));

if (positional.length !== 1) {
  console.error('Usage: node lint-contract.mjs <file> [--json] [--gate]');
  process.exit(2);
}

const findings = [];
let catalog;
try { catalog = await loadRuleCatalog(); }
catch (error) { console.error(`Cannot load rule manifests: ${error.message}`); process.exit(2); }

function add(rule, message, hint, evidence = null) {
  const declared = catalog.byId.get(rule);
  if (!declared) throw new Error(`Checker emitted undeclared rule ${rule}.`);
  findings.push({ rule, severity: severityFor(catalog, rule), confidence: 'high', message, hint, evidence });
}

const stripComment = line => line
  .replace(/^\s*(\/\/+|\/\*+|\*+\/?|#+|<!--|-->)\s?/, '')
  .replace(/\s*(\*\/|-->)\s*$/, '')
  .trimEnd();

function extractContract(text) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex(line => /CONSIDERED-CONTRACT\s+v1/i.test(line));
  if (start === -1) return null;
  const body = [];
  let sawRoll = false;
  for (let i = start + 1; i < lines.length; i++) {
    const raw = stripComment(lines[i]);
    // A Markdown code-fence line (```, optionally with a language tag such as
    // ```html) is never contract content, whether it appears inside the
    // contract body or immediately after the comment closes (e.g. the
    // contract was pasted into a fenced code block in a design doc). Skip it
    // outright so it can never be swept into whatever field was last open.
    if (/^```/.test(raw.trim())) continue;
    if (/CONSIDERED-CONTRACT-END/i.test(raw)) break;
    // Leading whitespace must be tolerated here exactly like parseFields'
    // own field-start regex tolerates it: an indented contract body (e.g.
    // one written inside an HTML/JS comment for readability) otherwise never
    // sets sawRoll, so the break condition below never fires and every line
    // through end-of-file gets swept into whatever field was last open.
    if (sawRoll && (raw.trim() === '' || /^\s*(?:import|export|const|let|var|function|class|@|<[A-Za-z])/i.test(raw))) break;
    if (/^\s*ROLL\s*:/i.test(raw)) sawRoll = true;
    body.push(raw);
  }
  return body.join('\n');
}

function parseFields(block) {
  const fields = {}, syntax = [];
  let current = null;
  for (const raw of block.split('\n')) {
    const line = raw.trimEnd();
    if (!line.trim()) continue;
    const known = line.match(/^\s*([A-Z][A-Z-]{2,})\s*:\s*(.*)$/);
    if (known) {
      const name = known[1].toUpperCase();
      if (!REQUIRED.includes(name)) { syntax.push(`Unknown field ${name}.`); current = null; continue; }
      if (fields[name]) syntax.push(`Duplicate field ${name}.`);
      fields[name] = known[2].trim() ? [known[2].trim()] : [];
      current = name;
    } else if (current) {
      fields[current].push(line.trim());
    } else {
      syntax.push(`Unattached line: ${line.trim()}`);
    }
  }
  for (const key of REQUIRED) if (!fields[key] || !fields[key].join(' ').trim()) syntax.push(`Missing field ${key}.`);
  return { fields, syntax };
}

function parseQuestions(lines) {
  const result = [], errors = [];
  for (const line of lines || []) {
    const match = line.match(/^Q(\d+)\s+\[(P[0-4])\]\s+(.+?)\s*->\s*([A-Za-z][\w-]*)$/i);
    if (!match) { errors.push(`Invalid question row: ${line}`); continue; }
    result.push({ id: `Q${match[1]}`, tier: match[2].toUpperCase(), text: match[3].trim(), zone: match[4] });
  }
  const ids = new Set();
  for (const q of result) {
    if (ids.has(q.id)) errors.push(`Duplicate question id ${q.id}.`);
    ids.add(q.id);
  }
  return { result, errors };
}

function parseZones(lines) {
  const result = [], errors = [];
  for (const line of lines || []) {
    const match = line.match(/^([A-Za-z][\w-]*)\s+"([^"]+)"\s*::\s*([^:]+?)\s*::\s*(.+)$/);
    if (!match) { errors.push(`Invalid zone row: ${line}`); continue; }
    result.push({
      id: match[1], heading: match[2].trim(),
      questions: match[3].split(',').map(value => value.trim().toUpperCase()).filter(Boolean),
      elements: match[4].split(',').map(value => value.trim()).filter(Boolean)
    });
  }
  const ids = new Set();
  for (const zone of result) {
    if (ids.has(zone.id)) errors.push(`Duplicate zone id ${zone.id}.`);
    ids.add(zone.id);
  }
  return { result, errors };
}

function parseHierarchy(lines) {
  const tiers = { P0: [], P1: [], P2: [], P3: [], P4: [] }, errors = [];
  for (const line of lines || []) {
    const match = line.match(/^(P[0-4])\s+(.+)$/i);
    if (!match) { errors.push(`Invalid hierarchy row: ${line}`); continue; }
    const tier = match[1].toUpperCase();
    const raw = match[2].trim();
    // P4 is chrome. It may be written as "P4 chrome: nav, filters" and is
    // intentionally excluded from the question-to-zone element graph.
    // The optional " - reason" suffix is trailing free text and must never
    // be split on: a comma or semicolon inside the reason (e.g. "E1 - it
    // beats every rival, including the summary card") previously split the
    // row on the delimiter first and stripped the reason per-piece
    // afterward, turning reason text after the first delimiter into a
    // phantom extra element. Split the reason off the raw row first (on the
    // first " - "), then only split the remaining element list.
    const reasonSplit = raw.match(/^(.*?)\s+-\s+(.*)$/);
    const elementsSource = reasonSplit ? reasonSplit[1] : raw;
    const parts = tier === 'P4' && /^chrome\s*:/i.test(raw)
      ? raw.replace(/^chrome\s*:/i, '').split(/[,;]/).map(value => value.trim()).filter(Boolean)
      : elementsSource.split(/[,;]/).map(value => value.trim()).filter(Boolean);
    if (!parts.length) errors.push(`No elements in ${tier}.`);
    tiers[tier].push(...parts);
  }
  const elements = new Map();
  for (const [tier, names] of Object.entries(tiers)) {
    for (const name of names) {
      if (elements.has(name)) errors.push(`Element ${name} appears in ${elements.get(name)} and ${tier}.`);
      elements.set(name, tier);
    }
  }
  return { tiers, elements, errors };
}

function parseActions(lines) {
  const result = [], errors = [];
  for (const line of lines || []) {
    const match = line.match(/^S([0-5])\s+(primary|secondary|tertiary|overflow)\s+"([^"]+)"\s+(safe|reversible|destructive)(?:\s+(destruction-purpose|separated))?$/i);
    if (!match) { errors.push(`Invalid action row: ${line}`); continue; }
    result.push({ scope: Number(match[1]), tier: match[2].toLowerCase(), label: match[3].trim(), risk: match[4].toLowerCase(), qualifier: (match[5] || '').toLowerCase() });
  }
  return { result, errors };
}

async function checkRoll(value) {
  const parts = value.split(',').map(part => part.trim()).filter(Boolean);
  const tail = parts.pop() || '';
  const key = tail.match(/^(cns-[a-z0-9]{8})\/(\d+)$/i);
  if (parts.length !== 4 || !key) {
    add('ROLL-01', 'ROLL must contain axis, depth, framing, direction, and cns-<8 chars>/<generation>.', 'Copy the exact ROLL line printed by roll.mjs.');
    return;
  }
  const [structures, directions] = await Promise.all(['structures', 'directions'].map(async name =>
    JSON.parse(await readFile(join(ROOT, 'assets', 'decks', `${name}.json`), 'utf8')).entries
  ));
  const tiers = ['organizing-axis', 'depth-strategy', 'framing'];
  for (let index = 0; index < 3; index++) {
    if (!structures.some(entry => entry.id === parts[index] && entry.tier === tiers[index])) {
      add('ROLL-01', `ROLL item "${parts[index]}" is not a valid ${tiers[index]} id.`, 'Record the assigned ids exactly as printed by roll.mjs.');
    }
  }
  if (!directions.some(entry => entry.id === parts[3])) {
    add('ROLL-01', `ROLL item "${parts[3]}" is not a valid direction id.`, 'Record the assigned direction id exactly as printed by roll.mjs.');
  }
}

const target = positional[0];
let text;
try { text = await readFile(target, 'utf8'); }
catch (error) { console.error(`Cannot read ${target}: ${error.message}`); process.exit(2); }

const block = extractContract(text);
let details = { mode: null, questions: 0, zones: 0 };
if (!block) {
  add('CONTRACT-01', 'No CONSIDERED-CONTRACT v1 block found.', 'Run the structure phase and add a contract comment or use .considered/STRUCTURE.md.');
} else {
  const { fields, syntax } = parseFields(block);
  if (syntax.length) add('CONTRACT-01', syntax.join(' '), 'Use assets/templates/contract-block.md exactly.');

  const mode = (fields.MODE || []).join(' ').trim().toLowerCase();
  details.mode = mode;
  if (mode && !MODES.includes(mode)) add('DASH-02', `MODE "${mode}" must be exactly one supported mode.`, `Choose one of: ${MODES.join(', ')}.`);
  const thesis = (fields.THESIS || []).join(' ').toLowerCase();
  if (thesis && !/refus|reject|avoid|not\s+/.test(thesis)) add('CONTRACT-01', 'THESIS does not state a default it refuses.', 'Name the organizing idea and the conventional alternative it rejects.');
  const decision = (fields.DECISION || []).join(' ').toLowerCase();
  if (decision && (!/decid/.test(decision) || !/within|before|under|in\s+\d+/.test(decision) || !/cost|risk|consequence|wrong/.test(decision))) add('CONTRACT-01', 'DECISION lacks a decision verb, time budget, or consequence.', 'State who decides what, within what time, and what being wrong costs.');

  const questions = parseQuestions(fields.QUESTIONS);
  const zones = parseZones(fields.ZONES);
  const hierarchy = parseHierarchy(fields.HIERARCHY);
  const actions = parseActions(fields.ACTIONS);
  details.questions = questions.result.length;
  details.zones = zones.result.length;
  const syntaxErrors = [...questions.errors, ...zones.errors, ...hierarchy.errors, ...actions.errors];
  if (syntaxErrors.length) add('CONTRACT-01', syntaxErrors.join(' '), 'Correct every malformed, duplicate, or conflicting row.');

  const questionById = new Map(questions.result.map(question => [question.id, question]));
  const zoneById = new Map(zones.result.map(zone => [zone.id, zone]));
  const elements = new Map();
  for (const zone of zones.result) {
    if (BANNED_HEADINGS.has(zone.heading.toLowerCase())) add('IA-09', `Zone heading "${zone.heading}" is a container label.`, 'Name the question, object, or decision served.');
    if (zone.elements.length < 3 || zone.elements.length > 7) add('IA-06', `Zone "${zone.heading}" has ${zone.elements.length} elements.`, 'Keep each zone between 3 and 7 elements.');
    if (!zone.questions.length) add('IA-01', `Zone "${zone.heading}" has elements but no ranked question.`, 'Map every zone to one or more declared questions.');
    for (const id of zone.questions) {
      const question = questionById.get(id);
      if (!question) add('IA-01', `Zone "${zone.heading}" references unknown question ${id}.`, 'Use only question ids declared in QUESTIONS.');
      else if (question.zone !== zone.id) add('IA-01', `${id} maps to ${question.zone}, but appears in zone ${zone.id}.`, 'Keep question arrows and zone membership identical.');
    }
    for (const element of zone.elements) {
      if (elements.has(element)) add('IA-07', `Element "${element}" appears in both "${elements.get(element)}" and "${zone.heading}".`, 'Assign it to one zone.');
      else elements.set(element, zone.heading);
    }
  }
  for (const question of questions.result) {
    if (!zoneById.has(question.zone)) add('IA-01', `${question.id} maps to unknown zone ${question.zone}.`, 'Create that zone or update the question mapping.');
    else if (!zoneById.get(question.zone).questions.includes(question.id)) add('IA-17', `${question.id} is not listed in its declared zone ${question.zone}.`, 'The default zone must answer every declared ranked question.');
  }
  const budget = catalog.manifests.contract.modeModuleBudget?.[mode];
  const moduleCount = zones.result.reduce((sum, zone) => sum + zone.elements.length, 0);
  if (budget && (moduleCount < budget[0] || moduleCount > budget[1])) add('DASH-03', `${moduleCount} declared elements is outside the ${mode} module budget of ${budget[0]} to ${budget[1]}.`, 'Delete, defer, or add only elements that answer ranked questions.');

  if (hierarchy.tiers.P0.length !== 1) add('HIER-05', `${hierarchy.tiers.P0.length} P0 elements declared.`, 'Declare exactly one P0.');
  if (hierarchy.tiers.P1.length < 3 || hierarchy.tiers.P1.length > 5) add('HIER-06', `${hierarchy.tiers.P1.length} P1 elements declared.`, 'Declare 3 to 5 P1 elements.');
  const p0Line = (fields.HIERARCHY || []).find(line => /^P0\b/i.test(line)) || '';
  if (hierarchy.tiers.P0.length === 1 && !/\s-\s|because|beats/i.test(p0Line)) add('CONTRACT-01', 'P0 has no stated reason it wins the focal position.', 'Append " - <why it beats every rival>" to the P0 row.');
  for (const [element] of elements) if (!hierarchy.elements.has(element)) add('IA-01', `Element "${element}" has no hierarchy tier.`, 'Give every zone element exactly one P0 to P3 tier.');
  for (const [element, tier] of hierarchy.elements) if (tier !== 'P4' && !elements.has(element)) add('IA-01', `Hierarchy element "${element}" is not placed in a zone.`, 'Place it in the zone that answers its question, or remove it.');

  const emphasis = { primary: 3, secondary: 2, tertiary: 1, overflow: 0 };
  const primaries = new Map();
  const byScope = new Map();
  for (const action of actions.result) {
    if (action.tier === 'primary') (primaries.get(action.scope) || primaries.set(action.scope, []).get(action.scope)).push(action);
    if (!byScope.has(action.scope)) byScope.set(action.scope, []);
    byScope.get(action.scope).push(action);
    if (action.risk === 'destructive' && action.tier === 'primary' && action.qualifier !== 'destruction-purpose') add('ACT-08', `Destructive action "${action.label}" is primary at S${action.scope}.`, 'Demote it or declare a destruction-purpose scope.');
    if (action.label.split(/\s+/).length > 3 || /^(submit|ok|yes|no|go|continue|manage|update|click here)$/i.test(action.label)) add('ACT-05', `Action label "${action.label}" does not name a concise outcome.`, 'Use a verb plus object in three words or fewer.');
  }
  for (const [scope, rows] of primaries) if (rows.length > 1) add('ACT-01', `S${scope} has ${rows.length} primary actions.`, 'Keep one primary action per scope.');
  const scopes = [...byScope.keys()].sort((a, b) => a - b);
  for (const scope of scopes) {
    const narrower = Math.max(...byScope.get(scope).map(action => emphasis[action.tier]));
    for (const parent of scopes.filter(candidate => candidate < scope)) {
      const wider = Math.max(...byScope.get(parent).map(action => emphasis[action.tier]));
      if (narrower >= wider) add('ACT-03', `S${scope} has emphasis ${narrower}, not strictly lower than enclosing S${parent} at ${wider}.`, 'Demote the narrower-scope action treatment.');
    }
    const visible = byScope.get(scope).filter(action => action.tier !== 'overflow');
    if (visible.length > 3) add('ACT-04', `S${scope} has ${visible.length} visible actions.`, 'Move surplus actions into overflow.');
  }

  const deleted = (fields.DELETED || []).join(' ').trim();
  const deletions = deleted.split(';').map(value => value.trim()).filter(Boolean);
  if (!deletions.length || deletions.some(value => !/^.+\s-\s.+$/.test(value)) || /^(none|nothing|n\/a)$/i.test(deleted)) add('LOOP-04', 'DELETED must name one or more removed items and a reason for each.', 'Use "item - reason; item - reason".');
  await checkRoll((fields.ROLL || []).join(' ').trim());
}

const counts = summarizeFindings(findings);
const output = { file: target, checker: 'contract', scope: 'contract-internal', details, counts, findings };
if (json) console.log(JSON.stringify(output, null, 2));
else {
  console.log(`\n  CONTRACT AUDIT  ${target}`);
  console.log(`  mode ${details.mode || 'undeclared'}   questions ${details.questions}   zones ${details.zones}`);
  if (!findings.length) console.log('\n  No findings in contract-internal checks. Run the independent review gate before shipping.\n');
  else for (const finding of findings) console.log(`  ${finding.rule.padEnd(12)} ${finding.severity}  ${finding.message}\n               ${finding.hint}`);
  console.log(`\n  ${counts.S1 || 0} blocking, ${counts.S2 || 0} major, ${counts.S3 || 0} minor\n`);
}
process.exit(gate ? ((counts.S1 || 0) || (counts.S2 || 0) > 2 || (counts.S3 || 0) > 6 ? 1 : 0) : ((counts.S1 || 0) ? 1 : 0));
