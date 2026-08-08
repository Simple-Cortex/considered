#!/usr/bin/env node
/**
 * Blind gold-standard panel judging via the Codex CLI — the token-efficient
 * stress harness. Deterministic Node assembles an anonymized panel (our shot
 * shuffled among gold reference screens, format-normalized so nothing
 * fingerprints the candidate), then a non-Anthropic judge (codex exec) ranks
 * all screens choosing its own criteria. No agent orchestration; the only
 * model cost is the Codex call.
 *
 * Usage:
 *   node codex-judge.mjs --shot <our-capture.png> --gold <dir with gold-1..N.jpg>
 *     --category "<judging label, e.g. 'daily-use admin settings page'>"
 *     --out <output dir> --seed <n> [--codex-model <m>]
 *
 * Output layout under --out:
 *   panel/screen-a..jpg   anonymized panel shown to the judge
 *   map.json              which screen is ours (never passed to the judge)
 *   codex-raw.json        judge's verbatim response
 *   verdict.json          decoded: ranking, our rank, winner, per-screen scores
 *
 * Host requirements: macOS `sips` for format normalization; `python3` with PIL
 * for the top-anchored crop (sips ignores --cropOffset); `codex` CLI on PATH.
 * The judge is never told a screen is generated, which one is the candidate,
 * or that a skill exists. Keep it that way.
 */
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';

const args = process.argv.slice(2);
const value = flag => { const i = args.indexOf(flag); return i === -1 ? null : args[i + 1] || null; };
const shot = value('--shot'); const goldDir = value('--gold'); const category = value('--category');
const outDir = value('--out'); const seed = value('--seed'); const codexModel = value('--codex-model');
if (!shot || !goldDir || !category || !outDir || !seed) {
  console.error('codex-judge requires --shot, --gold, --category, --out, --seed.');
  process.exit(2);
}

// Seeded shuffle position — same PRNG family as blind.mjs so runs are replayable.
function cyrb128(str) {
  let h1 = 1779033703, h2 = 3144134277, h3 = 1013904242, h4 = 2773480762;
  for (let i = 0, k; i < str.length; i++) {
    k = str.charCodeAt(i);
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067);
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233);
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213);
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179);
  }
  return (h1 ^ h2 ^ h3 ^ h4) >>> 0;
}
function mulberry32(a) {
  return function () {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const sips = (...a) => execFileSync('sips', a, { stdio: ['ignore', 'pipe', 'ignore'] }).toString();
const dim = (file, key) => parseInt(sips('-g', key, file).trim().split(/\s+/).pop(), 10);

const golds = readdirSync(goldDir).filter(f => /^gold-\d+\.jpe?g$/.test(f)).sort();
if (golds.length < 2) { console.error(`Need at least 2 gold-N.jpg refs in ${goldDir}.`); process.exit(2); }
const slots = 'abcdefgh'.slice(0, golds.length + 1).split('').map(s => `screen-${s}.jpg`);
const ourIndex = Math.floor(mulberry32(cyrb128(`${seed}:${category}`))() * slots.length);

const panelDir = join(resolve(outDir), 'panel');
rmSync(panelDir, { recursive: true, force: true });
mkdirSync(panelDir, { recursive: true });

// Normalize ours to the reference geometry: top-crop to the gold aspect, then
// resample to the gold width, so neither aspect nor resolution reveals the candidate.
const refW = dim(join(goldDir, golds[0]), 'pixelWidth');
const refH = dim(join(goldDir, golds[0]), 'pixelHeight');
const ourW = dim(shot, 'pixelWidth');
const cropH = Math.min(Math.round(ourW * refH / refW), dim(shot, 'pixelHeight'));
const tmp = join(tmpdir(), `codex-judge-${process.pid}.png`);
// sips silently ignores --cropOffset and always center-crops, which judged
// mid-page instead of the fold; PIL anchors the crop window at the top.
execFileSync('python3', ['-c',
  'import sys\nfrom PIL import Image\nim = Image.open(sys.argv[1])\nim.crop((0, 0, im.size[0], int(sys.argv[2]))).save(sys.argv[3])',
  shot, String(cropH), tmp]);

let g = 0;
for (let i = 0; i < slots.length; i++) {
  const target = join(panelDir, slots[i]);
  if (i === ourIndex) sips('-s', 'format', 'jpeg', '--resampleWidth', String(refW), tmp, '--out', target);
  else cpSync(join(goldDir, golds[g++]), target);
}
// Re-encode every panel image identically so tool metadata cannot fingerprint any of them.
for (const s of slots) {
  const f = join(panelDir, s);
  sips('-s', 'format', 'jpeg', '-s', 'formatOptions', '85', f, '--out', `${f}.tmp`);
  rmSync(f); cpSync(`${f}.tmp`, f); rmSync(`${f}.tmp`);
}
writeFileSync(join(resolve(outDir), 'map.json'), `${JSON.stringify({ ours: slots[ourIndex], seed, category }, null, 2)}\n`);

const order = slots.map((s, i) => `image ${i + 1} is ${s}`).join(', ');
const shape = slots.map(s => `"${s}":{"strongest":"...","weakest":"...","score":0}`).join(',');
const prompt = `You are shown ${slots.length} anonymized ${category} designs, attached in order: ${order}. ` +
  `You do not know where any of them came from. As a senior design director, rank them from strongest to weakest ` +
  `as ${category} design, judging whatever details you consider decisive. Respond with ONLY this JSON, no prose: ` +
  `{"ranking":[${slots.map(() => '"screen-?.jpg"').join(',')}],"winner":"screen-?.jpg","screens":{${shape}}}`;

const rawOut = join(resolve(outDir), 'codex-raw.json');
// `-i` is variadic and would swallow a trailing positional prompt, so the
// prompt-bearing flagless argument must be fenced behind a non-image flag.
const codexArgs = ['exec', '-s', 'read-only', '--color', 'never'];
if (codexModel) codexArgs.push('-m', codexModel);
for (const s of slots) codexArgs.push('-i', join(panelDir, s));
codexArgs.push('-o', rawOut, prompt);
execFileSync('codex', codexArgs, { stdio: ['ignore', 'ignore', 'inherit'] });

const raw = readFileSync(rawOut, 'utf8');
const match = raw.match(/\{[\s\S]*\}/);
if (!match) { console.error('Codex returned no JSON; see codex-raw.json.'); process.exit(1); }
const judged = JSON.parse(match[0]);
const ours = slots[ourIndex];
const verdict = {
  category, seed, ours,
  our_rank: judged.ranking.indexOf(ours) + 1,
  of: slots.length,
  winner: judged.winner,
  winner_is_ours: judged.winner === ours,
  our_score: judged.screens?.[ours]?.score ?? null,
  ranking: judged.ranking,
  screens: judged.screens,
};
writeFileSync(join(resolve(outDir), 'verdict.json'), `${JSON.stringify(verdict, null, 2)}\n`);
console.log(`${category}: ours=${ours} rank ${verdict.our_rank}/${slots.length}, winner ${judged.winner}${verdict.winner_is_ours ? ' (ours)' : ''}`);
