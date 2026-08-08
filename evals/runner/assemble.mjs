#!/usr/bin/env node
/** Merge run manifests, review forms, and the decoded label map into one result record. */
import { execFile } from 'node:child_process';
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { dirname, join, relative, resolve } from 'node:path';
import { promisify } from 'node:util';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, '..', '..');
const SCENARIOS = join(REPO, 'evals', 'scenarios');
const CONDITIONS = ['baseline', 'considered'];
const WEIGHTS = { D1: 5, D2: 5, D3: 5, D4: 4, D5: 4, D6: 3, D7: 4, D8: 3, D9: 2 };
const FLOORS = ['D1', 'D2', 'D3', 'D7'];
const SEVERITIES = ['S1', 'S2', 'S3', 'S4'];
const GATES = ['Ship', 'Revise', 'Redesign', 'Not assessable'];

const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
if (args.includes('--help') || args.includes('-h')) { console.log('Usage: node assemble.mjs --results <result-root>\n\nReads <root>/environment.json, <root>/<condition>/<scenario>/<run>/manifest.json,\n<root>/label-map.json, and <root>/reviews/<packet-id>/<reviewer-id>.json.'); process.exit(0); }
const resultsArg = value('--results');
if (!resultsArg) { console.error('assemble requires --results <result-root>.'); process.exit(2); }
const root = resolve(resultsArg);
const errors = [];
const error = message => errors.push(message);
const sha256 = text => createHash('sha256').update(text).digest('hex');
const rel = path => relative(root, path).split('\\').join('/');
const readJson = async path => JSON.parse(await readFile(path, 'utf8'));
const exists = async path => { try { await stat(path); return true; } catch { return false; } };
const mean = list => (list.length ? list.reduce((sum, item) => sum + item, 0) / list.length : null);
const median = list => {
  if (!list.length) return null;
  const sorted = [...list].sort((a, b) => a - b);
  const middle = sorted.length >> 1;
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};
const rate = (numerator, denominator, notes) => ({ value: denominator ? numerator / denominator : null, numerator, denominator, ...(notes ? { notes } : {}) });

/* ---------- catalog and environment ---------- */

// The frozen copies inside the result root are the catalog of record (protocol.md
// collection step 1). The live evals/scenarios/ directory also carries stress-round
// variants outside any frozen catalog; it is only a fallback for legacy roots.
const frozenDir = join(root, 'scenarios');
const catalogDir = await stat(frozenDir).then(entry => entry.isDirectory()).catch(() => false) ? frozenDir : SCENARIOS;
const catalog = { version: '1.0.0', scenario_ids: [], sha256: {} };
for (const file of (await readdir(catalogDir)).filter(name => name.endsWith('.json')).sort()) {
  const text = await readFile(join(catalogDir, file), 'utf8');
  const scenario = JSON.parse(text);
  catalog.scenario_ids.push(scenario.id);
  catalog.sha256[scenario.id] = sha256(text);
}

let environment;
try { environment = await readJson(join(root, 'environment.json')); }
catch { console.error(`Cannot read ${join(root, 'environment.json')}. Fill the skeleton stage.mjs wrote before assembling.`); process.exit(2); }
for (const [key, item] of Object.entries({ model: environment.model, browser_version: environment.browser_version, time_budget_seconds: environment.time_budget_seconds, randomization_seed: environment.randomization_seed })) {
  if (item === null || item === undefined) error(`environment.${key} is still unset; the record must state what actually ran.`);
}

/* ---------- runs ---------- */

const runs = [];
const exclusions = [];
const runById = new Map();
for (const condition of CONDITIONS) {
  const conditionDir = join(root, condition);
  for (const scenarioId of (await readdir(conditionDir).catch(() => [])).sort()) {
    for (const runName of (await readdir(join(conditionDir, scenarioId)).catch(() => [])).sort()) {
      const runDir = join(conditionDir, scenarioId, runName);
      let manifest;
      try { manifest = await readJson(join(runDir, 'manifest.json')); }
      catch { error(`${rel(runDir)}: no readable manifest.json.`); continue; }
      const run = {
        run_id: manifest.run_id,
        scenario_id: manifest.scenario_id ?? scenarioId,
        condition: manifest.condition ?? condition,
        status: manifest.status ?? 'incomplete',
        started_at: manifest.started_at,
        prompt_sha256: manifest.prompt_sha256,
        scenario_sha256: manifest.scenario_sha256
      };
      if (manifest.finished_at) run.finished_at = manifest.finished_at;
      if (manifest.condition_context_sha256) run.condition_context_sha256 = manifest.condition_context_sha256;
      if (manifest.retry_of) run.retry_of = manifest.retry_of;
      if (run.status === 'completed') {
        const archive = join(runDir, 'source.tar.gz');
        // artifact_paths are derived from what is on disk, not from what a manifest claims.
        if (!await exists(archive) && await exists(join(runDir, 'source'))) {
          await promisify(execFile)('tar', ['-czf', archive, '-C', runDir, 'source']);
        }
        const evidence = (await readdir(join(runDir, 'evidence')).catch(() => [])).sort();
        const paths = {
          source: rel(archive),
          desktop_screenshot: rel(join(runDir, 'desktop.png')),
          mobile_screenshot: rel(join(runDir, 'mobile.png')),
          interaction_evidence: evidence.filter(name => name.startsWith('interaction')).map(name => rel(join(runDir, 'evidence', name))),
          state_evidence: evidence.filter(name => /^(state|unreachable)/.test(name)).map(name => rel(join(runDir, 'evidence', name)))
        };
        for (const [field, file] of [['contract', 'contract.txt'], ['lint_source', 'lint-source.json'], ['lint_contract', 'lint-contract.json']]) {
          if (await exists(join(runDir, file))) paths[field] = rel(join(runDir, file));
        }
        if (await exists(join(runDir, 'run.log'))) paths.run_log = rel(join(runDir, 'run.log'));
        if (!paths.interaction_evidence.length) error(`${run.run_id}: completed run has no interaction evidence.`);
        if (!paths.state_evidence.length) error(`${run.run_id}: completed run has no state evidence.`);
        if (run.condition === 'considered') for (const field of ['contract', 'lint_source', 'lint_contract']) if (!paths[field]) error(`${run.run_id}: considered run is missing ${field}.`);
        run.artifact_paths = paths;
      }
      if (['failed', 'excluded'].includes(run.status)) {
        run.failure_reason = manifest.failure_reason || 'reason not recorded';
        exclusions.push({ scenario_id: run.scenario_id, condition: run.condition, run_id: run.run_id, reason: run.failure_reason, ...(manifest.replacement_run_id ? { replacement_run_id: manifest.replacement_run_id } : {}) });
      }
      runs.push(run);
      runById.set(run.run_id, run);
    }
  }
}

/* ---------- packets and reviews ---------- */

let labelMap;
try { labelMap = await readJson(join(root, 'label-map.json')); }
catch { console.error(`Cannot read ${join(root, 'label-map.json')}. Run blind.mjs first.`); process.exit(2); }

const blindPackets = [];
for (const [packetId, entry] of Object.entries(labelMap.packets || {})) {
  blindPackets.push({
    packet_id: packetId,
    scenario_id: entry.scenario_id,
    label_a_run_id: entry.run_id_by_label?.A ?? '',
    label_b_run_id: entry.run_id_by_label?.B ?? '',
    packet_manifest: `blind-packets/${packetId}/manifest.json`,
    label_map_revealed: true,
    decoded_condition_by_label: entry.condition_by_label
  });
}

const weightedTotal = scores => {
  const parts = Object.entries(WEIGHTS).map(([key, weight]) => (scores?.[key] === null || scores?.[key] === undefined ? null : Number(scores[key]) * weight));
  return parts.some(part => part === null) ? null : parts.reduce((sum, part) => sum + part, 0);
};

const reviews = [];
const reviewsDir = join(root, 'reviews');
for (const packetId of (await readdir(reviewsDir).catch(() => [])).sort()) {
  const packet = labelMap.packets?.[packetId];
  if (!packet) { error(`reviews/${packetId} has no entry in label-map.json.`); continue; }
  for (const file of (await readdir(join(reviewsDir, packetId)).catch(() => [])).filter(name => name.endsWith('.json')).sort()) {
    const form = await readJson(join(reviewsDir, packetId, file));
    const reviewerId = form.reviewer_id || file.replace(/\.json$/, '');
    const scores = {};
    for (const label of ['A', 'B']) {
      const given = form.scores?.[label] || {};
      const dimensions = Object.fromEntries(Object.keys(WEIGHTS).map(key => [key, given[key] ?? null]));
      scores[label] = { ...dimensions, weighted_total: weightedTotal(dimensions) };
    }
    const review = {
      review_id: form.review_id || `${packetId}-${reviewerId}`,
      reviewer_id: reviewerId,
      reviewer_role: form.reviewer_role || 'reviewer',
      packet_id: packetId,
      scenario_id: packet.scenario_id,
      conflict_declared: Boolean(form.conflict_declared),
      suspected_unblinding: Boolean(form.suspected_unblinding),
      evidence_viewed: form.evidence_viewed?.length ? form.evidence_viewed : ['other'],
      pairwise_choice: form.pairwise_choice,
      choice_reason: form.choice_reason || 'reason not recorded',
      unsupported_claim_label: form.unsupported_claim_label || 'cannot_assess',
      scores,
      gate_by_label: form.gate_by_label || { A: 'Not assessable', B: 'Not assessable' },
      findings: form.findings || []
    };
    if (form.unblinding_note) review.unblinding_note = form.unblinding_note;
    if (form.unsupported_claim_note) review.unsupported_claim_note = form.unsupported_claim_note;
    if (review.conflict_declared) error(`${review.review_id}: reviewer declared a conflict and may not rate this packet.`);
    reviews.push(review);
  }
}

/* ---------- metrics ---------- */

const conditionOf = (packetId, label) => labelMap.packets?.[packetId]?.condition_by_label?.[label] ?? null;
const observations = { baseline: [], considered: [] };
let consideredWins = 0, baselineWins = 0, ties = 0, abstentions = 0;
const scoreDeltas = [];
const dimensionDeltas = Object.fromEntries(Object.keys(WEIGHTS).map(key => [key, []]));
const severityDeltas = Object.fromEntries(SEVERITIES.map(key => [key, []]));

for (const review of reviews) {
  if (review.pairwise_choice === 'tie') ties++;
  else if (review.pairwise_choice === 'A' || review.pairwise_choice === 'B') {
    if (conditionOf(review.packet_id, review.pairwise_choice) === 'considered') consideredWins++; else baselineWins++;
  } else abstentions++;

  const perLabel = {};
  for (const label of ['A', 'B']) {
    const condition = conditionOf(review.packet_id, label);
    if (!condition) { error(`${review.review_id}: cannot decode label ${label}.`); continue; }
    const findings = review.findings.filter(finding => (finding.label ?? finding.output_label) === label);
    const unsupportedField = review.unsupported_claim_label;
    const unsupported = unsupportedField && typeof unsupportedField === 'object'
      ? (unsupportedField[label] ? 1 : 0)
      : (['both', label].includes(unsupportedField) ? 1 : (unsupportedField === 'cannot_assess' ? null : 0));
    const observation = {
      scores: review.scores[label],
      gate: review.gate_by_label?.[label] ?? 'Not assessable',
      severity: Object.fromEntries(SEVERITIES.map(key => [key, findings.filter(finding => finding.severity === key).length])),
      unsupported
    };
    observations[condition].push(observation);
    perLabel[condition] = observation;
  }
  if (perLabel.baseline && perLabel.considered) {
    const c = perLabel.considered.scores.weighted_total;
    const b = perLabel.baseline.scores.weighted_total;
    if (c !== null && b !== null) scoreDeltas.push(c - b);
    for (const key of Object.keys(WEIGHTS)) {
      const cv = perLabel.considered.scores[key], bv = perLabel.baseline.scores[key];
      if (cv !== null && bv !== null) dimensionDeltas[key].push(cv - bv);
    }
    for (const key of SEVERITIES) severityDeltas[key].push(perLabel.considered.severity[key] - perLabel.baseline.severity[key]);
  }
}

const completedByCondition = condition => runs.filter(run => run.condition === condition && run.status === 'completed');
const conditionMetrics = condition => {
  const rows = observations[condition];
  const totals = rows.map(row => row.scores.weighted_total).filter(total => total !== null);
  const claimRows = rows.filter(row => row.unsupported !== null);
  const completed = completedByCondition(condition);
  const evidenceComplete = completed.filter(run => {
    const paths = run.artifact_paths || {};
    const required = ['source', 'desktop_screenshot', 'mobile_screenshot', 'interaction_evidence', 'state_evidence', ...(condition === 'considered' ? ['contract', 'lint_source', 'lint_contract'] : [])];
    return required.every(field => (Array.isArray(paths[field]) ? paths[field].length : Boolean(paths[field])));
  });
  return {
    mean_weighted_score: { value: mean(totals), numerator: Math.round(totals.reduce((sum, total) => sum + total, 0)), denominator: totals.length, notes: 'not-assessable dimensions leave the output unscored rather than zero' },
    median_weighted_score: median(totals),
    blocker_free_rate: rate(rows.filter(row => row.severity.S1 === 0).length, rows.length),
    unsupported_claim_rate: rate(claimRows.filter(row => row.unsupported === 1).length, claimRows.length, 'cannot-assess reviews are excluded from the denominator'),
    evidence_complete_rate: rate(evidenceComplete.length, completed.length, 'collection integrity, not output quality'),
    gate_distribution: Object.fromEntries(GATES.map(gate => [gate, rows.filter(row => row.gate === gate).length])),
    floor_pass_rate: Object.fromEntries(FLOORS.map(dimension => {
      const scored = rows.filter(row => row.scores[dimension] !== null);
      return [dimension, rate(scored.filter(row => row.scores[dimension] >= 3).length, scored.length)];
    }))
  };
};

const nonAbstaining = consideredWins + baselineWins + ties;
const baselineMetrics = conditionMetrics('baseline');
const consideredMetrics = conditionMetrics('considered');
const metrics = {
  computed_at: new Date().toISOString(),
  blinded_preference: {
    considered_wins: consideredWins,
    baseline_wins: baselineWins,
    ties,
    abstentions,
    preference_rate: rate(consideredWins + ties * 0.5, nonAbstaining, 'half-tie convention; 0.50 is neutral'),
    net_preference: { value: nonAbstaining ? (consideredWins - baselineWins) / nonAbstaining : null, numerator: consideredWins - baselineWins, denominator: nonAbstaining }
  },
  baseline: baselineMetrics,
  considered: consideredMetrics,
  deltas: {
    mean_weighted_score: mean(scoreDeltas),
    median_weighted_score: median(scoreDeltas),
    severity: Object.fromEntries(SEVERITIES.map(key => [key, mean(severityDeltas[key])])),
    by_dimension: Object.fromEntries(Object.keys(WEIGHTS).map(key => [key, mean(dimensionDeltas[key])]))
  }
};
// net_preference.numerator is a count, and the schema forbids a negative one.
if (metrics.blinded_preference.net_preference.numerator < 0) {
  metrics.blinded_preference.net_preference.numerator = baselineWins - consideredWins;
  metrics.blinded_preference.net_preference.notes = 'numerator is the absolute win margin; the sign is carried by value (baseline ahead)';
}
// Tripwire: a silently-empty findings join once zeroed every severity metric while the
// forms carried scores of findings. If reviews record findings but every per-pair
// severity delta and both blocker counts came out empty, the join is broken — refuse.
const totalFindings = reviews.reduce((sum, review) => sum + (review.findings?.length ?? 0), 0);
const countedFindings = [...observations.baseline, ...observations.considered]
  .reduce((sum, row) => sum + SEVERITIES.reduce((s, key) => s + row.severity[key], 0), 0);
if (totalFindings > 0 && countedFindings === 0) {
  console.error(`assemble: reviews carry ${totalFindings} findings but none matched a label — the findings join is broken.`);
  process.exit(1);
}

const result = {
  schema_version: '1.0.0',
  evaluation_id: environment.evaluation_id || labelMap.evaluation_id || root.split(/[\\/]/).pop(),
  protocol_version: '1.0.0',
  created_at: new Date().toISOString(),
  scenario_catalog: catalog,
  environment: Object.fromEntries(Object.entries(environment).filter(([key]) => key !== 'evaluation_id')),
  runs,
  blind_packets: blindPackets,
  reviews,
  metrics,
  exclusions
};

await writeFile(join(root, 'result.json'), `${JSON.stringify(result, null, 2)}\n`);
if (errors.length) {
  console.error('ASSEMBLY WARNINGS (result.json written; npm run check will fail until these are fixed)');
  for (const message of errors) console.error(`- ${message}`);
}
console.log(`Wrote ${join(root, 'result.json')}: ${runs.length} runs, ${blindPackets.length} packets, ${reviews.length} reviews.`);
process.exit(errors.length ? 1 : 0);
