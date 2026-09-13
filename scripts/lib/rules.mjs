import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const RULE_DIR = join(ROOT, 'assets', 'rules');
const KINDS = ['contract', 'source', 'render', 'guidance'];
// Guidance rules are an advisory catalog read by humans and agents; they carry prose
// in `check` and have no executable counterpart. Only these kinds dispatch a checker.
const EXECUTABLE_KINDS = new Set(['contract', 'source', 'render']);

export async function loadGatePolicy() {
  const data = JSON.parse(await readFile(join(RULE_DIR, 'gate.json'), 'utf8'));
  if (data.kind !== 'gate' || !data.findingLimits || !data.review) {
    throw new Error('Invalid gate policy manifest.');
  }
  const { review } = data;
  if (!Array.isArray(review.criticalDimensions) || !review.criticalDimensions.length) {
    throw new Error('Gate policy has no critical dimensions.');
  }
  if (!Array.isArray(review.recognizedVerdicts) || !review.recognizedVerdicts.includes(review.shipVerdict)) {
    throw new Error('Gate policy ship verdict is not among the recognized verdicts.');
  }
  for (const key of ['S1', 'S2', 'S3']) {
    if (!Number.isInteger(data.findingLimits[key])) throw new Error(`Gate policy ${key} limit must be an integer.`);
  }
  for (const key of ['minimumScore', 'dimensionFloor']) {
    if (!Number.isFinite(review[key])) throw new Error(`Gate policy ${key} must be a number.`);
  }
  return data;
}

// Loaded once so evaluateGate stays synchronous for its callers.
export const GATE_POLICY = await loadGatePolicy();
export const GATE_LIMITS = GATE_POLICY.findingLimits;

export async function loadRuleCatalog() {
  const records = await Promise.all(KINDS.map(async kind => {
    const file = join(RULE_DIR, `${kind}.json`);
    const data = JSON.parse(await readFile(file, 'utf8'));
    if (data.kind !== kind || !Array.isArray(data.rules)) {
      throw new Error(`Invalid ${kind} rule manifest.`);
    }
    return [kind, data];
  }));

  const byId = new Map();
  for (const [kind, data] of records) {
    for (const rule of data.rules) {
      if (!rule.id || !rule.severity) {
        throw new Error(`${kind} manifest has a rule missing id or severity.`);
      }
      if (EXECUTABLE_KINDS.has(kind) && !rule.checker) {
        throw new Error(`${kind} rule ${rule.id} has no checker.`);
      }
      if (byId.has(rule.id)) throw new Error(`Duplicate rule id ${rule.id} across manifests.`);
      byId.set(rule.id, { ...rule, kind });
    }
  }
  return { byId, manifests: Object.fromEntries(records) };
}

export function severityFor(catalog, id, fallback = 'S2') {
  return catalog.byId.get(id)?.severity || fallback;
}

export function ruleFor(catalog, id) {
  return catalog.byId.get(id) || null;
}

export function summarizeFindings(findings) {
  return findings.reduce((counts, finding) => {
    counts[finding.severity] = (counts[finding.severity] || 0) + 1;
    return counts;
  }, {});
}

export function evaluateGate({ findings = [], review = null, requireReview = true, policy = GATE_POLICY }) {
  const limits = policy.findingLimits;
  const rules = policy.review;
  // Source regex findings are leads for a reviewer, not proof. Only high-confidence
  // findings may block automatically; all findings remain in the output.
  const gatingFindings = findings.filter(finding => finding.confidence !== 'heuristic');
  const counts = summarizeFindings(gatingFindings);
  const heuristicCounts = summarizeFindings(findings.filter(finding => finding.confidence === 'heuristic'));
  const blockers = (counts.S1 || 0) > limits.S1;
  const majors = (counts.S2 || 0) > limits.S2;
  const minors = (counts.S3 || 0) > limits.S3;
  const reasons = [];
  if (blockers) reasons.push(`${counts.S1} S1 finding(s)`);
  if (majors) reasons.push(`${counts.S2} S2 findings (limit ${limits.S2})`);
  if (minors) reasons.push(`${counts.S3} S3 findings (limit ${limits.S3})`);

  if (requireReview && !review) {
    return { outcome: 'REVIEW-REQUIRED', counts, heuristicCounts, reasons: ['Independent review result is missing.'] };
  }

  if (review) {
    if (review.stale === true) {
      return {
        outcome: 'REVISE',
        counts,
        heuristicCounts,
        reasons: ['Independent review is stale; rerun it against the current source and contract.']
      };
    }
    // A review without an interpretable verdict is not a review. Never infer one:
    // an absent verdict previously passed the gate, which let a build self-certify.
    const verdict = typeof review.outcome === 'string' ? review.outcome.trim().toUpperCase() : null;
    if (!verdict) {
      return {
        outcome: 'REVIEW-REQUIRED',
        counts,
        heuristicCounts,
        reasons: ['Independent review states no outcome.']
      };
    }
    if (!rules.recognizedVerdicts.includes(verdict)) {
      return {
        outcome: 'REVIEW-REQUIRED',
        counts,
        heuristicCounts,
        reasons: [`Reviewer outcome "${review.outcome}" is not one of ${rules.recognizedVerdicts.join(', ')}.`]
      };
    }

    const score = Number(review.weightedScore);
    const missingDimension = rules.criticalDimensions.find(key => !Number.isFinite(Number(review.dimensions?.[key])));
    const lowDimension = rules.criticalDimensions.find(key => Number(review.dimensions?.[key]) < rules.dimensionFloor);
    if (!Number.isFinite(score)) reasons.push('Reviewer weightedScore is missing.');
    else if (score < rules.minimumScore) reasons.push(`Reviewer score ${score}/${rules.maxScore} is below ${rules.minimumScore}.`);
    if (missingDimension) reasons.push(`Reviewer ${missingDimension} score is missing.`);
    else if (lowDimension) reasons.push(`${lowDimension} is below the floor of ${rules.dimensionFloor}.`);
    if (verdict !== rules.shipVerdict) reasons.push(`Reviewer outcome is ${review.outcome}.`);
  }

  if (blockers || majors || minors || reasons.length) {
    return { outcome: 'REVISE', counts, heuristicCounts, reasons };
  }
  return { outcome: 'SHIP', counts, heuristicCounts, reasons: [] };
}

export function rootPath(...parts) {
  return join(ROOT, ...parts);
}
