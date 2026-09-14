#!/usr/bin/env node
/**
 * Validate renderer or reviewer-produced visual evidence.
 *
 * This deliberately does not pretend that Node can infer hierarchy from PNG
 * bytes without a vision system. Capture tooling or the independent reviewer
 * produces a render-evidence/v1 JSON record from screenshots at declared
 * viewports; this script makes its measurements and assertions gateable.
 */
import { readFile } from 'node:fs/promises';
import { loadRuleCatalog, severityFor, summarizeFindings } from './lib/rules.mjs';

const args = process.argv.slice(2);
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node lint-render.mjs <render-evidence.json> [--json]\n\nValidates a render-evidence/v1 JSON record (from capture tooling or the\nindependent reviewer) against hierarchy, whitespace, and responsive-reflow\nassertions measured at declared viewports.');
  process.exit(0);
}
const json = args.includes('--json');
const positional = args.filter(arg => arg !== '--json');
if (positional.length !== 1) {
  console.error('Usage: node lint-render.mjs <render-evidence.json> [--json]');
  process.exit(2);
}

let catalog, evidence;
try {
  catalog = await loadRuleCatalog();
  evidence = JSON.parse(await readFile(positional[0], 'utf8'));
} catch (error) {
  console.error(`Cannot read render evidence: ${error.message}`);
  process.exit(2);
}

const findings = [];
const add = (rule, message, hint, viewport = null) => findings.push({
  rule, severity: severityFor(catalog, rule), confidence: 'attested', message, hint, viewport
});

if (evidence.kind !== 'render-evidence/v1' || !Array.isArray(evidence.viewports) || !evidence.viewports.length) {
  add('HIER-01', 'Render evidence is missing a non-empty viewports array.', 'Use assets/templates/RENDER-EVIDENCE.json and attach screenshots to the independent review.');
} else {
  for (const view of evidence.viewports) {
    const label = view.name || `${view.width || '?'}px`;
    if (!Number.isInteger(view.focalRegions) || view.focalRegions !== 1) add('HIER-01', `${label}: ${view.focalRegions ?? 'no'} dominant focal regions recorded.`, 'Blur or squint-test the screenshot and leave one P0 dominant.', label);
    for (const zone of view.zones || []) {
      if (!Number.isFinite(zone.intraGap) || !Number.isFinite(zone.interGap) || zone.intraGap > zone.interGap / 2) add('HIER-07', `${label}: zone "${zone.name || 'unnamed'}" fails the 2:1 proximity ratio.`, 'Tighten intra-zone gaps or increase inter-zone gaps.', label);
    }
    if (!view.p0AboveFold || !view.p1AboveFold) add('HIER-08', `${label}: P0 and all P1 do not fit in the declared first viewport.`, 'Reduce chrome or defer lower-ranked material.', label);
    if (view.dangerAndPrimaryVisible && !view.statusOutranksAccent) add('HIER-10', `${label}: a visible danger state does not outrank the primary action.`, 'Give the consequential status the stronger treatment.', label);
    if (!view.grayscaleHierarchy || !view.grayscaleStatus) add('HIER-11', `${label}: hierarchy or status fails in grayscale.`, 'Encode meaning with position, size, weight, text, or shape in addition to color.', label);
    if (!view.fiveSecondP0Recognized) add('HIER-12', `${label}: the intended P0 was not identified in the five-second test.`, 'Strengthen P0 and subtract competing emphasis.', label);
    for (const metric of view.metrics || []) if (!(metric.valueWeight > metric.labelWeight)) add('DASH-06', `${label}: metric "${metric.name || 'unnamed'}" label is not weaker than its value.`, 'Increase value prominence or reduce label emphasis.', label);
    const whitespace = Number(view.whitespaceRatio);
    if (Number.isFinite(whitespace)) {
      const range = evidence.mode === 'operate' ? [0.20, 0.30] : [0.30, 0.40];
      if (whitespace < range[0] || whitespace > range[1]) add('COMP-06', `${label}: whitespace ratio ${whitespace} is outside ${range[0]} to ${range[1]} for ${evidence.mode || 'this'} mode.`, 'Adjust density only after confirming question and tier priorities.', label);
    }
    if (view.priorityOrderHolds === false) add('COMP-09', `${label}: breakpoint reflow changed the declared priority order.`, 'Reflow by tier order, not component convenience.', label);
  }
}

const counts = summarizeFindings(findings);
const output = { checker: 'render-evidence', evidence: positional[0], counts, findings };
if (json) console.log(JSON.stringify(output, null, 2));
else {
  console.log(`\n  RENDER EVIDENCE AUDIT  ${positional[0]}\n`);
  if (!findings.length) console.log('  No attested render findings. Evidence must come from an independent reviewer or capture pipeline.\n');
  else for (const finding of findings) console.log(`  ${finding.rule.padEnd(10)} ${finding.severity}  ${finding.message}\n               ${finding.hint}\n`);
}
process.exit((counts.S1 || 0) || (counts.S2 || 0) > 2 || (counts.S3 || 0) > 6 ? 1 : 0);
