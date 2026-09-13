#!/usr/bin/env node
/**
 * Combine checker reports and an independent review into the real ship gate.
 * Reports are JSON output from `considered contract --json` and `considered lint --json`.
 */
import { readFile } from 'node:fs/promises';
import { evaluateGate } from './lib/rules.mjs';

const args = process.argv.slice(2);
const json = args.includes('--json');
const reviewIndex = args.indexOf('--review');
let reviewPath = null;
if (reviewIndex !== -1) {
  reviewPath = args[reviewIndex + 1];
  if (!reviewPath || reviewPath.startsWith('--')) {
    console.error('Usage: node gate.mjs <report.json> [...report.json] --review <REVIEW.json> [--json]');
    process.exit(2);
  }
}
const reportPaths = args.filter((arg, index) => arg !== '--json' && arg !== '--review' && index !== reviewIndex + 1);
if (!reportPaths.length) {
  console.error('Usage: node gate.mjs <report.json> [...report.json] --review <REVIEW.json> [--json]');
  process.exit(2);
}

async function readJson(path) {
  try { return JSON.parse(await readFile(path, 'utf8')); }
  catch (error) { throw new Error(`Cannot read JSON from ${path}: ${error.message}`); }
}

let reports;
let review = null;
try {
  reports = await Promise.all(reportPaths.map(readJson));
  if (reviewPath) review = await readJson(reviewPath);
} catch (error) {
  console.error(error.message);
  process.exit(2);
}

const findings = reports.flatMap(report => Array.isArray(report.findings) ? report.findings : []);
// Accept the portable REVIEW.json template and the smaller legacy review shape.
const normalizedReview = review ? {
  weightedScore: review.weightedScore ?? review.gate?.score,
  dimensions: review.dimensions ?? review.gate?.dimensions,
  outcome: review.outcome ?? review.gate?.decision,
  stale: review.reviewValidity?.stale === true || review.freshContext === false
} : null;
const result = evaluateGate({ findings, review: normalizedReview, requireReview: true });
const output = { reports: reportPaths, review: reviewPath, findings, ...result };

if (json) console.log(JSON.stringify(output, null, 2));
else {
  console.log(`\n  CONSIDERED GATE  ${result.outcome}`);
  console.log(`  ${result.counts.S1 || 0} S1, ${result.counts.S2 || 0} S2, ${result.counts.S3 || 0} S3`);
  const h = result.heuristicCounts || {};
  if ((h.S1 || 0) + (h.S2 || 0) + (h.S3 || 0)) console.log(`  reviewer leads: ${h.S1 || 0} S1, ${h.S2 || 0} S2, ${h.S3 || 0} S3 heuristic`);
  for (const reason of result.reasons) console.log(`  - ${reason}`);
  console.log('');
}

process.exit(result.outcome === 'SHIP' ? 0 : 1);
