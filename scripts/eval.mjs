#!/usr/bin/env node
/**
 * Initialize one immutable condition manifest for the matched M0 evaluation.
 * This runner intentionally does not invoke an AI model or score outputs.
 */
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, access, readdir } from 'node:fs/promises';
import { constants } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, relative, resolve } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
const SCENARIOS = join(ROOT, 'evals', 'scenarios');
const args = process.argv.slice(2);
function value(flag) { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; }
function usage(message = '', code = 2) {
  const stream = code === 0 ? console.log : console.error;
  if (message) stream(`considered eval: ${message}`);
  stream('Usage: node eval.mjs --condition baseline|considered --evaluation-id <id> --out <directory>');
  process.exit(code);
}
if (args.includes('--help') || args.includes('-h')) { usage('', 0); }
const allowed = new Set(['--condition', '--evaluation-id', '--out']);
for (const token of args.filter(token => token.startsWith('--'))) if (!allowed.has(token)) usage(`unknown option ${token}`);
if (args.length !== 6) usage('each required option must appear exactly once');
const condition = value('--condition'), evaluationId = value('--evaluation-id'), out = value('--out');
if (!['baseline', 'considered'].includes(condition)) usage('--condition must be baseline or considered');
if (!/^[A-Za-z0-9][A-Za-z0-9._-]{2,127}$/.test(evaluationId || '')) usage('invalid evaluation id');
if (!out) usage('--out is required');
const path = resolve(out);
try { await access(path, constants.F_OK); usage(`${out} already exists; do not overwrite an evaluation condition`); } catch (error) { if (error.code !== 'ENOENT') usage(`cannot inspect ${out}: ${error.message}`); }

const hash = text => createHash('sha256').update(text).digest('hex');
const entries = (await readdir(SCENARIOS)).filter(name => name.endsWith('.json')).sort();
if (entries.length !== 10) usage(`expected 10 scenarios, found ${entries.length}`);
const scenarios = [];
for (const name of entries) {
  const text = await readFile(join(SCENARIOS, name), 'utf8');
  const data = JSON.parse(text);
  if (!data.id || !data.scenario_version) usage(`invalid scenario ${name}`);
  scenarios.push({ id: data.id, file: relative(ROOT, join(SCENARIOS, name)), sha256: hash(text), mode: data.mode, route: data.build?.route || '/' });
}

await mkdir(dirname(path), { recursive: true });
await mkdir(path, { recursive: false });
for (const scenario of scenarios) {
  const runDir = join(path, scenario.id, 'run-001');
  await mkdir(join(runDir, 'source'), { recursive: true });
  await mkdir(join(runDir, 'evidence'), { recursive: true });
  await writeFile(join(runDir, 'README.md'), `# ${scenario.id} ${condition} run\n\nStatus: incomplete.\n\nAdd source, desktop.png, mobile.png, interaction evidence, and state evidence.\nDo not modify the frozen scenario or manifest after collecting artifacts.\n`);
}

const manifest = {
  schema_version: 'considered.eval-collection/v1',
  evaluation_id: evaluationId,
  condition,
  created_at: new Date().toISOString(),
  catalog_version: '1.0.0',
  scenarios,
  condition_rules: condition === 'baseline'
    ? ['Do not load Considered SKILL.md, decks, contracts, rules, or prior outputs.', 'Use the matched environment described in evals/protocol.md.']
    : ['Use the current Considered workflow and retain contract and lint evidence.', 'Do not inspect baseline outputs or blind-review data.'],
  run_manifest_template: {
    status: 'incomplete',
    required: ['source', 'desktop.png', 'mobile.png', 'evidence/interactions', 'evidence/states', 'prompt.txt', 'environment.json']
  }
};
await writeFile(join(path, 'collection-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log(`Initialized ${condition} collection for ${evaluationId}: ${scenarios.length} frozen scenarios at ${out}.`);
console.log('Collection is incomplete until a runner adds evidence. This command does not score output.');
