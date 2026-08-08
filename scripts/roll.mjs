#!/usr/bin/env node
/**
 * roll.mjs - the dice.
 *
 * Assigns a structure and a direction for a build. It does not present a menu.
 *
 * Why: a model asked to generate design directions produces a genuinely varied
 * shortlist and then crowns the same winner nearly every run. Handing that
 * shortlist to any selector, including the model itself, a scoring pass, or a
 * simulated user, collapses back to option one. Assignment is the only
 * mechanism that survives. See impeccable.style/research, lesson 3.
 *
 * Draws are deterministic from a reproduction key, so a bad draw in the field
 * is a replayable bug report rather than a mystery. Re-rolls chain: everything
 * dealt in earlier generations under the same key is excluded, so "roll again"
 * explores instead of reshuffling.
 */

import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const DECKS = join(HERE, '..', 'assets', 'decks');

const MODES = ['persuade', 'operate', 'analyze', 'read', 'experience'];
const STRUCTURE_TIERS = ['organizing-axis', 'depth-strategy', 'framing'];

/* ---------- deterministic PRNG ---------- */

function cyrb128(str) {
  let h1 = 1779033703, h2 = 3144134277, h3 = 1013904242, h4 = 2773480762;
  for (let i = 0; i < str.length; i++) {
    const k = str.charCodeAt(i);
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067);
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233);
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213);
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179);
  }
  h1 = Math.imul(h3 ^ (h1 >>> 18), 597399067);
  h2 = Math.imul(h4 ^ (h2 >>> 22), 2869860233);
  h3 = Math.imul(h1 ^ (h3 >>> 17), 951274213);
  h4 = Math.imul(h2 ^ (h4 >>> 19), 2716044179);
  return (h1 ^ h2 ^ h3 ^ h4) >>> 0;
}

function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a = (a + 0x6D2B79F5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/* ---------- weighting ---------- */

// Ratings express odds, not eligibility. Every reviewed direction must remain
// reachable: one-star = 1, two-star = 2, three-star = 4.
function weightOf(entry) {
  const r = Number(entry.rating) || 1;
  if (r >= 3) return 4;
  if (r === 2) return 2;
  return 1;
}

function weightedPick(pool, rnd) {
  if (!pool.length) return null;
  let weights = pool.map(weightOf);
  let total = weights.reduce((s, w) => s + w, 0);
  if (total === 0) { weights = pool.map(() => 1); total = pool.length; }
  let roll = rnd() * total;
  for (let i = 0; i < pool.length; i++) {
    roll -= weights[i];
    if (roll <= 0) return pool[i];
  }
  return pool[pool.length - 1];
}

/* ---------- deck loading ---------- */

const DECK_LAWS = {
  structures: ['grouping', 'hierarchy', 'actions', 'empty', 'motion'],
  directions: ['palette', 'type', 'topology', 'controls', 'motion']
};

const nonEmptyString = value => typeof value === 'string' && value.trim().length > 0;

function validateDeck(deck, name) {
  const problems = [];
  const problem = message => problems.push(message);

  if (!deck || typeof deck !== 'object' || Array.isArray(deck)) {
    throw new Error('root must be an object');
  }
  if (deck.$schema !== 'considered-deck/v1') problem('$schema must be "considered-deck/v1"');
  if (deck.kind !== name) problem(`kind must be "${name}"`);
  if (!nonEmptyString(deck.version)) problem('version must be a non-empty string');
  if (!Array.isArray(deck.entries) || deck.entries.length === 0) problem('entries must be a non-empty array');

  if (name === 'structures') {
    if (!Array.isArray(deck.tiers) ||
        deck.tiers.length !== STRUCTURE_TIERS.length ||
        STRUCTURE_TIERS.some(tier => !deck.tiers.includes(tier))) {
      problem(`tiers must contain exactly: ${STRUCTURE_TIERS.join(', ')}`);
    }
  }

  const ids = new Set();
  if (Array.isArray(deck.entries)) {
    deck.entries.forEach((entry, index) => {
      const at = `entries[${index}]`;
      if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        problem(`${at} must be an object`);
        return;
      }
      if (!nonEmptyString(entry.id)) problem(`${at}.id must be a non-empty string`);
      else if (ids.has(entry.id)) problem(`${at}.id duplicates "${entry.id}"`);
      else ids.add(entry.id);
      for (const field of ['name', 'thesis']) {
        if (!nonEmptyString(entry[field])) problem(`${at}.${field} must be a non-empty string`);
      }
      if (!Number.isInteger(entry.rating) || entry.rating < 1 || entry.rating > 3) {
        problem(`${at}.rating must be an integer from 1 to 3`);
      }
      if (!Array.isArray(entry.modes) || entry.modes.length === 0 ||
          entry.modes.some(mode => !MODES.includes(mode)) ||
          new Set(entry.modes).size !== entry.modes.length) {
        problem(`${at}.modes must contain unique supported modes`);
      }
      if (!entry.laws || typeof entry.laws !== 'object' || Array.isArray(entry.laws)) {
        problem(`${at}.laws must be an object`);
      } else {
        for (const law of DECK_LAWS[name]) {
          if (!nonEmptyString(entry.laws[law])) problem(`${at}.laws.${law} must be a non-empty string`);
        }
      }

      if (name === 'structures') {
        if (!STRUCTURE_TIERS.includes(entry.tier)) problem(`${at}.tier is unsupported`);
        for (const field of ['refuses', 'avoid']) {
          if (!nonEmptyString(entry[field])) problem(`${at}.${field} must be a non-empty string`);
        }
        if (!Array.isArray(entry.fits) || entry.fits.length === 0 || entry.fits.some(value => !nonEmptyString(value))) {
          problem(`${at}.fits must be a non-empty array of strings`);
        }
      } else {
        for (const field of ['source', 'strength', 'verb', 'spark', 'borrowSkeleton']) {
          if (!nonEmptyString(entry[field])) problem(`${at}.${field} must be a non-empty string`);
        }
      }
    });
  }

  if (name === 'structures' && Array.isArray(deck.entries)) {
    for (const tier of STRUCTURE_TIERS) {
      if (!deck.entries.some(entry => entry?.tier === tier)) problem(`entries have no "${tier}" tier`);
    }
  }

  if (problems.length) {
    const shown = problems.slice(0, 12);
    const rest = problems.length - shown.length;
    throw new Error(`${shown.join('; ')}${rest ? `; and ${rest} more` : ''}`);
  }
  return deck.entries;
}

async function loadDeck(name) {
  const path = join(DECKS, `${name}.json`);
  try {
    const deck = JSON.parse(await readFile(path, 'utf8'));
    return validateDeck(deck, name);
  } catch (err) {
    if (err.code === 'ENOENT') {
      console.error(`Deck not found: ${path}`);
      console.error('The skill is installed incompletely. Reinstall or restore assets/decks/.');
      process.exit(2);
    }
    console.error(`Deck ${name}.json is unreadable or invalid: ${err.message}`);
    process.exit(2);
  }
}

const fitsMode = (entry, mode) => !mode || entry.modes.includes(mode);

/* ---------- the draw ---------- */

function drawGeneration(structures, directions, key, gen) {
  const rnd = mulberry32(cyrb128(`${key}::${gen}`));
  const dealt = { structure: {}, direction: null };

  for (const tier of STRUCTURE_TIERS) {
    dealt.structure[tier] = weightedPick(structures.filter(e => e.tier === tier), rnd);
  }
  dealt.direction = weightedPick(directions, rnd);
  return dealt;
}

/**
 * Replay generations 0..gen-1 to build the exclusion set, then draw `gen`
 * from what remains. This is what makes a re-roll explore rather than reshuffle,
 * without persisting any state to disk.
 */
function generationCapacity(structures, directions, mode) {
  const eligibleStructures = structures.filter(entry => fitsMode(entry, mode));
  const eligibleDirections = directions.filter(entry => fitsMode(entry, mode));
  return Math.min(
    ...STRUCTURE_TIERS.map(tier => eligibleStructures.filter(entry => entry.tier === tier).length),
    eligibleDirections.length
  );
}

function drawChained(structures, directions, mode, key, gen) {
  let s = structures.filter(entry => fitsMode(entry, mode));
  let d = directions.filter(entry => fitsMode(entry, mode));

  for (let g = 0; g < gen; g++) {
    const prev = drawGeneration(s, d, key, g);
    const usedS = new Set(Object.values(prev.structure).map(entry => entry.id));
    s = s.filter(entry => !usedS.has(entry.id));
    d = d.filter(entry => entry.id !== prev.direction.id);
  }

  return { dealt: drawGeneration(s, d, key, gen) };
}

/* ---------- presentation ---------- */

function newKey() {
  const alphabet = 'abcdefghijkmnpqrstuvwxyz23456789';
  let k = '';
  for (let i = 0; i < 8; i++) k += alphabet[Math.floor(Math.random() * alphabet.length)];
  return `cns-${k}`;
}

function renderStructure(e) {
  if (!e) return '  (none available)\n';
  const L = e.laws || {};
  return [
    `  ${e.name}  [${e.id}]  ${'*'.repeat(Number(e.rating) || 1)}`,
    `    Thesis:   ${e.thesis || ''}`,
    e.refuses ? `    Refuses:  ${e.refuses}` : null,
    L.grouping  ? `    Grouping: ${L.grouping}`  : null,
    L.hierarchy ? `    Focal:    ${L.hierarchy}` : null,
    L.actions   ? `    Actions:  ${L.actions}`   : null,
    L.empty     ? `    Empty:    ${L.empty}`     : null,
    L.motion    ? `    Motion:   ${L.motion}`    : null,
    ''
  ].filter(Boolean).join('\n');
}

function renderDirection(e) {
  if (!e) return '  (none available)\n';
  const L = e.laws || {};
  return [
    `  ${e.name}  [${e.id}]  ${'*'.repeat(Number(e.rating) || 1)}`,
    e.source ? `    Source:   ${e.source}` : null,
    e.verb || e.strength ? `    Carries:  ${e.strength || ''}${e.verb ? `, ${e.verb}` : ''}` : null,
    `    Thesis:   ${e.thesis || ''}`,
    L.palette   ? `    Palette:  ${L.palette}`   : null,
    L.type      ? `    Type:     ${L.type}`      : null,
    L.topology  ? `    Topology: ${L.topology}`  : null,
    L.controls  ? `    Controls: ${L.controls}`  : null,
    L.motion    ? `    Motion:   ${L.motion}`    : null,
    e.spark          ? `    Spark:    ${e.spark}` : null,
    e.borrowSkeleton ? `    Skeleton: ${e.borrowSkeleton}` : null,
    ''
  ].filter(Boolean).join('\n');
}

function usage() {
  console.log(`considered roll - assigns a structure and direction for a build

Usage:
  node roll.mjs [--mode <mode>] [--key <key>] [--gen <n>] [--json]
  node roll.mjs [--mode <mode>] --key <key> --reroll [--json]

Options:
  --mode    ${MODES.join(' | ')}   (optional; unfiltered if omitted)
  --key     reproduction key. Reuses a previous draw exactly.
  --reroll  advance to generation 1; requires --key and forbids --gen.
  --gen     replay generation n, bounded by the selected mode's pool.
  --json    machine-readable output.
  -h, --help
            show this help.

The assignment is not a suggestion. Build what was dealt.`);
}

function usageError(message) {
  console.error(`considered roll: ${message}`);
  console.error('Run "considered roll --help" for usage.');
  process.exit(2);
}

function parseArgs(argv) {
  const values = new Map();
  const switches = new Set();
  const valueFlags = new Set(['--mode', '--key', '--gen']);
  const switchFlags = new Set(['--reroll', '--json', '--help', '-h']);

  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    if (token.includes('=')) usageError(`malformed option "${token}"; use a space before its value`);
    if (valueFlags.has(token)) {
      if (values.has(token)) usageError(`option "${token}" may only be used once`);
      const value = argv[++i];
      if (value === undefined || value.length === 0 || value.startsWith('-')) usageError(`option "${token}" requires a value`);
      values.set(token, value);
    } else if (switchFlags.has(token)) {
      if (switches.has(token)) usageError(`option "${token}" may only be used once`);
      switches.add(token);
    } else if (token.startsWith('-')) {
      usageError(`unknown option "${token}"`);
    } else {
      usageError(`unexpected argument "${token}"`);
    }
  }

  if (switches.has('--help') || switches.has('-h')) {
    if (argv.length !== 1) usageError('help cannot be combined with other options');
    return { help: true };
  }

  return {
    help: false,
    mode: values.get('--mode') || null,
    key: values.get('--key') || null,
    gen: values.get('--gen'),
    reroll: switches.has('--reroll'),
    json: switches.has('--json')
  };
}

/* ---------- main ---------- */

const options = parseArgs(process.argv.slice(2));
if (options.help) {
  usage();
  process.exit(0);
}

if (options.mode && !MODES.includes(options.mode)) {
  usageError(`unknown mode "${options.mode}"; expected one of: ${MODES.join(', ')}`);
}
if (options.reroll && !options.key) usageError('--reroll requires --key');
if (options.reroll && options.gen !== undefined) usageError('--reroll cannot be combined with --gen');

let gen = options.reroll ? 1 : 0;
if (options.gen !== undefined) {
  if (!/^\d+$/.test(options.gen)) usageError('--gen must be a non-negative integer');
  gen = Number(options.gen);
  if (!Number.isSafeInteger(gen)) usageError('--gen is too large');
}
const key = options.key || newKey();

const [structures, directions] = await Promise.all([loadDeck('structures'), loadDeck('directions')]);
const capacity = generationCapacity(structures, directions, options.mode);
if (capacity === 0) {
  console.error(`Deck has no complete hand for mode "${options.mode || 'any'}".`);
  process.exit(3);
}
if (gen >= capacity) {
  console.error(`Deck exhausted at generation ${gen} for mode "${options.mode || 'any'}".`);
  console.error(`Available generations are 0 through ${capacity - 1}. Start a fresh key instead.`);
  process.exit(3);
}

const { dealt } = drawChained(structures, directions, options.mode, key, gen);

if (options.json) {
  console.log(JSON.stringify({
    key,
    generation: gen,
    mode: options.mode,
    capacity,
    warnings: [],
    structure: dealt.structure,
    direction: dealt.direction
  }, null, 2));
  process.exit(0);
}

console.log(`
ASSIGNED HAND
  key ${key}   generation ${gen}   mode ${options.mode || 'any'}

STRUCTURE
`);
for (const tier of STRUCTURE_TIERS) {
  console.log(`  ${tier.toUpperCase()}`);
  console.log(renderStructure(dealt.structure[tier]));
}
console.log('DIRECTION\n');
console.log(renderDirection(dealt.direction));
const modeArg = options.mode ? ` --mode ${options.mode}` : '';
const rerollCommand = gen === 0
  ? `node roll.mjs${modeArg} --key ${key} --reroll`
  : `node roll.mjs${modeArg} --key ${key} --gen ${gen + 1}`;
console.log(`  Reproduce:  node roll.mjs${modeArg} --key ${key} --gen ${gen}
  Re-roll:    ${gen + 1 < capacity ? rerollCommand : '(deck exhausted for this mode)'}

  Build what was dealt. Borrow the skeleton, not the clothes.
  Record "ROLL: <structure ids>, <direction id>, ${key}/${gen}" in the contract block.
`);
