#!/usr/bin/env node
/** Validate a completed matched evaluation record without scoring its quality. */
import { readFile, stat, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve, relative, sep } from 'node:path';
import { createHash } from 'node:crypto';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
const SCENARIOS = join(ROOT, 'evals', 'scenarios');
const args = process.argv.slice(2);
const json = args.includes('--json');
const positional = args.filter(arg => arg !== '--json');
if (positional.length !== 1) { console.error('Usage: node check.mjs <evaluation-result-directory> [--json]'); process.exit(2); }
const root = resolve(positional[0]);
const errors = [];
const error = message => errors.push(message);
const sha256 = text => createHash('sha256').update(text).digest('hex');
const isSafeRelative = path => typeof path === 'string' && path.length > 0 && !path.startsWith('/') && !path.split(/[\\/]/).includes('..');
async function exists(path) { try { return (await stat(path)).isFile(); } catch { return false; } }

let result;
try { result = JSON.parse(await readFile(join(root, 'result.json'), 'utf8')); }
catch (cause) { console.error(`Cannot read ${join(root, 'result.json')}: ${cause.message}`); process.exit(2); }

// The frozen copies inside the result root are the catalog of record (protocol.md
// collection step 1); the live evals/scenarios/ directory also carries stress-round
// variants that were never part of any frozen catalog, so it is only a fallback for
// records that predate frozen copies.
const frozenDir = join(root, 'scenarios');
const catalogDir = await stat(frozenDir).then(entry => entry.isDirectory()).catch(() => false) ? frozenDir : SCENARIOS;
const declaredIds = new Set(Array.isArray(result.scenario_catalog?.scenario_ids) ? result.scenario_catalog.scenario_ids : []);
const ids = [];
try {
  for (const file of (await readdir(catalogDir)).filter(name => name.endsWith('.json')).sort()) {
    const text = await readFile(join(catalogDir, file), 'utf8');
    const id = JSON.parse(text).id;
    // In the fallback dir, non-catalog stress scenarios sit beside the catalog; hash
    // integrity is still enforced for every scenario the record declares.
    if (catalogDir !== frozenDir && !declaredIds.has(id)) continue;
    ids.push({ id, hash: sha256(text) });
  }
} catch (cause) { error(`Cannot load scenario catalog: ${cause.message}`); }
const expectedIds = new Set(ids.map(entry => entry.id));
if (result.schema_version !== '1.0.0') error('schema_version must be 1.0.0.');
if (result.protocol_version !== '1.0.0') error('protocol_version must be 1.0.0.');
const receivedIds = result.scenario_catalog?.scenario_ids;
if (!Array.isArray(receivedIds) || receivedIds.length !== 10 || new Set(receivedIds).size !== 10 || receivedIds.some(id => !expectedIds.has(id))) error('scenario_catalog.scenario_ids must contain the current ten scenarios exactly once.');
for (const scenario of ids) if (result.scenario_catalog?.sha256?.[scenario.id] !== scenario.hash) error(`scenario hash mismatch for ${scenario.id}.`);

const runs = Array.isArray(result.runs) ? result.runs : [];
if (runs.length < 20) error('runs must contain at least 20 paired condition runs.');
const pairCount = new Map();
for (const run of runs) {
  if (!expectedIds.has(run.scenario_id)) { error(`unknown scenario in run ${run.run_id || '(unnamed)'}`); continue; }
  if (!['baseline', 'considered'].includes(run.condition)) { error(`invalid condition in run ${run.run_id || '(unnamed)'}`); continue; }
  const key = `${run.scenario_id}/${run.condition}`;
  pairCount.set(key, (pairCount.get(key) || 0) + 1);
  if (run.status === 'completed') {
    const paths = run.artifact_paths || {};
    const required = ['source', 'desktop_screenshot', 'mobile_screenshot', 'interaction_evidence', 'state_evidence'];
    if (run.condition === 'considered') required.push('contract', 'lint_source', 'lint_contract');
    for (const field of required) {
      const value = paths[field];
      const values = Array.isArray(value) ? value : [value];
      if (!values.length || values.some(item => !isSafeRelative(item))) { error(`${run.run_id}: ${field} must be safe relative path(s).`); continue; }
      for (const item of values) if (!await exists(join(root, item))) error(`${run.run_id}: missing ${field} at ${item}.`);
    }
  }
}
for (const id of expectedIds) for (const condition of ['baseline', 'considered']) if (!pairCount.has(`${id}/${condition}`)) error(`missing ${condition} run for ${id}.`);

for (const review of result.reviews || []) {
  for (const label of ['A', 'B']) {
    const scores = review.scores?.[label];
    if (!scores) { error(`${review.review_id || 'review'} lacks scores for ${label}.`); continue; }
    const weights = { D1: 5, D2: 5, D3: 5, D4: 4, D5: 4, D6: 3, D7: 4, D8: 3, D9: 2 };
    const values = Object.entries(weights).map(([key, weight]) => scores[key] === null ? null : Number(scores[key]) * weight);
    if (values.every(value => value !== null)) {
      const expected = values.reduce((sum, value) => sum + value, 0);
      if (scores.weighted_total !== expected) error(`${review.review_id || 'review'} ${label} weighted_total must equal ${expected}.`);
    }
  }
}

const output = { valid: errors.length === 0, directory: root, errors };
if (json) console.log(JSON.stringify(output, null, 2));
else if (errors.length) {
  console.error('EVALUATION CHECK FAILED');
  for (const message of errors) console.error(`- ${message}`);
} else console.log('Evaluation result is structurally valid. This does not establish an effectiveness result.');
process.exit(errors.length ? 1 : 0);
