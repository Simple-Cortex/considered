#!/usr/bin/env node
/** Emit the per-scenario condition order for one evaluation, before any build runs. */
import { readFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const SCENARIOS = resolve(HERE, '..', 'scenarios');

const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
if (args.includes('--help') || args.includes('-h')) { console.log('Usage: node order.mjs --seed <n> [--json]'); process.exit(0); }
const seed = value('--seed');
if (!seed) { console.error('order requires --seed <n>. Record the seed in the evaluation manifest.'); process.exit(2); }

// Same PRNG pair as scripts/roll.mjs: a draw is a replayable fact, not a coin toss.
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

const ids = [];
for (const file of (await readdir(SCENARIOS)).filter(name => name.endsWith('.json')).sort()) {
  ids.push(JSON.parse(await readFile(join(SCENARIOS, file), 'utf8')).id);
}

// Each scenario draws from its own stream, so adding or reordering scenarios cannot
// silently reshuffle the order already recorded for the others.
const order = ids.map(id => {
  const first = mulberry32(cyrb128(`${seed}:${id}`))() < 0.5 ? 'baseline' : 'considered';
  return { scenario_id: id, first, second: first === 'baseline' ? 'considered' : 'baseline' };
});

if (args.includes('--json')) console.log(JSON.stringify({ seed, order }, null, 2));
else for (const entry of order) console.log(`${entry.scenario_id} -> ${entry.first}`);
