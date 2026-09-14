#!/usr/bin/env node
/** Validate rule manifests and dice decks without external dependencies. */
import { readFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { loadRuleCatalog, loadGatePolicy } from './lib/rules.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
if (process.argv.slice(2).some(arg => arg === '--help' || arg === '-h')) {
  console.log('Usage: node validate-assets.mjs\n\nValidates packaged rule manifests, the gate policy, and the structure and\ndirection decks: internal consistency, and that every declared rule id is\nboth taught in a reference and reachable, with no dangling citations.');
  process.exit(0);
}
const MODES = new Set(['persuade', 'operate', 'analyze', 'read', 'experience']);
const TIERS = new Set(['organizing-axis', 'depth-strategy', 'framing']);
const failures = [];
const fail = message => failures.push(message);

let catalog;
try {
  catalog = await loadRuleCatalog();
  for (const [kind, data] of Object.entries(catalog.manifests)) {
    if (data.ruleCount !== data.rules.length) fail(`${kind}: ruleCount does not match rules length.`);
    for (const rule of data.rules) {
      if (!['S1', 'S2', 'S3', 'S4'].includes(rule.severity)) fail(`${kind}: ${rule.id} has invalid severity.`);
    }
  }
} catch (error) {
  fail(`Rule manifests: ${error.message}`);
}

let gatePolicy;
try {
  gatePolicy = await loadGatePolicy();
  const { review } = gatePolicy;
  if (review.minimumScore > review.maxScore) fail('gate: minimumScore exceeds maxScore.');
  if (review.dimensionFloor < 0) fail('gate: dimensionFloor must not be negative.');
  const duplicateDimension = review.criticalDimensions.find(
    (key, index) => review.criticalDimensions.indexOf(key) !== index
  );
  if (duplicateDimension) fail(`gate: duplicate critical dimension ${duplicateDimension}.`);
} catch (error) {
  fail(`Gate policy: ${error.message}`);
}

for (const name of ['structures', 'directions']) {
  try {
    const deck = JSON.parse(await readFile(join(ROOT, 'assets', 'decks', `${name}.json`), 'utf8'));
    if (deck.kind !== name || !Array.isArray(deck.entries)) {
      fail(`${name}: invalid deck kind or entries.`);
      continue;
    }
    const ids = new Set();
    for (const entry of deck.entries) {
      if (!entry.id || ids.has(entry.id)) fail(`${name}: duplicate or empty id "${entry.id || '(empty)'}".`);
      ids.add(entry.id);
      if (!Number.isInteger(entry.rating) || entry.rating < 1 || entry.rating > 3) fail(`${name}: ${entry.id} rating must be 1 to 3.`);
      if (!Array.isArray(entry.modes) || entry.modes.some(mode => !MODES.has(mode))) fail(`${name}: ${entry.id} has invalid modes.`);
      if (name === 'structures' && !TIERS.has(entry.tier)) fail(`${name}: ${entry.id} has invalid tier.`);
      if (!entry.thesis || !entry.laws || Object.keys(entry.laws).length < 4) fail(`${name}: ${entry.id} is missing thesis or laws.`);
    }
  } catch (error) {
    fail(`${name}: ${error.message}`);
  }
}

// A rule the linter or gate can fire, but that no reference teaches, is a rule an agent
// cannot act on. It surfaces only as a late failure with no guidance. Both directions of
// the reference must hold: every cited id resolves, and every defined rule is taught.
const RULE_ID = /\b([A-Z][A-Z0-9]{1,8})-(\d{2}[a-z]?)\b/g;
const TAUGHT_IN = ['references', 'assets/templates', 'docs', 'evals'];
const citations = new Map();

async function collectCitations(dir) {
  let entries;
  try { entries = await readdir(dir, { withFileTypes: true }); }
  catch (error) { if (error.code === 'ENOENT') return; throw error; }
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) { await collectCitations(path); continue; }
    if (!/\.(md|json)$/.test(entry.name)) continue;
    const text = await readFile(path, 'utf8');
    for (const match of text.matchAll(RULE_ID)) {
      const id = `${match[1]}-${match[2]}`;
      if (!citations.has(id)) citations.set(id, new Set());
      citations.get(id).add(path);
    }
  }
}

if (catalog) {
  try {
    for (const dir of TAUGHT_IN) await collectCitations(join(ROOT, dir));
    const untaught = [...catalog.byId.keys()].filter(id => !citations.has(id)).sort();
    if (untaught.length) {
      fail(`Rules defined but taught in no reference: ${untaught.join(', ')}.`);
    }
    // The inverse: a cited id that resolves to nothing is a broken reference.
    const dangling = [...citations.keys()]
      .filter(id => !catalog.byId.has(id) && !/^(A|Q|B|F|D|E|G|P)-?\d/.test(id))
      .sort();
    if (dangling.length) {
      fail(`Rule ids cited but defined in no manifest: ${dangling.join(', ')}.`);
    }
  } catch (error) {
    fail(`Rule coverage: ${error.message}`);
  }
}

if (failures.length) {
  console.error('ASSET VALIDATION FAILED');
  for (const message of failures) console.error(`- ${message}`);
  process.exit(1);
}
console.log(
  `Asset validation passed: ${catalog.byId.size} unique rules, two valid decks, ` +
  `and a gate policy at ${gatePolicy.review.minimumScore}/${gatePolicy.review.maxScore}.`
);
