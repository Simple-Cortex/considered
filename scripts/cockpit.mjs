#!/usr/bin/env node
/**
 * Static review cockpit generator.
 *
 * Reads JSON artifacts from a result directory and produces a self-contained
 * HTML review cockpit. No JavaScript, no external dependencies, no live
 * functionality — static inspection only.
 *
 * Usage: node cockpit.mjs <result-directory> [--output <path>]
 *
 * Expected artifacts in the result directory (all optional, gracefully handled):
 *   run-record.json   — run record (decision, roll, stage, reviewValidity)
 *   contract.json     — lint-contract --json output
 *   source.json       — lint-source --json output
 *   review.json       — review/Gate result with findings
 *   known-limitations.json — structured limitations (optional)
 */
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');

// --- Helpers -----------------------------------------------------------------

/** Escape a string for safe inclusion in HTML text content and attributes. */
function esc(value) {
  if (value === null || value === undefined) return '';
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

/** Read and parse a JSON file. Returns { data, error } — never throws. */
async function readJson(filePath) {
  try {
    const text = await readFile(filePath, 'utf8');
    return { data: JSON.parse(text), error: null };
  } catch (error) {
    return { data: null, error: error.message };
  }
}

/** Severity sort key — lower is more severe. */
function severityRank(sev) {
  const order = { S1: 1, S2: 2, S3: 3, S4: 4 };
  return order[sev] ?? 99;
}

/** Sort findings by severity, then file, then line. */
function sortFindings(findings) {
  return [...findings].sort((a, b) => {
    const sev = severityRank(a.severity) - severityRank(b.severity);
    if (sev !== 0) return sev;
    const file = (a.file || '').localeCompare(b.file || '');
    if (file !== 0) return file;
    return (a.line || 0) - (b.line || 0);
  });
}

// --- Data loading ------------------------------------------------------------

async function loadArtifacts(dir) {
  const warnings = [];
  const [runRecord, contract, source, review] = await Promise.all([
    readJson(join(dir, 'run-record.json')),
    readJson(join(dir, 'contract.json')),
    readJson(join(dir, 'source.json')),
    readJson(join(dir, 'review.json')),
  ]);

  // Try known-limitations from docs/ as fallback
  let knownLimitations = await readJson(join(dir, 'known-limitations.json'));
  if (knownLimitations.error) {
    knownLimitations = await readJson(join(ROOT, 'docs', 'known-limitations.md'));
    // We only use the JSON form; markdown is noted but not parsed
    if (knownLimitations.error) knownLimitations = { data: null, error: null };
  }

  if (runRecord.error) warnings.push(`run-record.json: ${runRecord.error}`);
  if (contract.error) warnings.push(`contract.json: ${contract.error}`);
  if (source.error) warnings.push(`source.json: ${source.error}`);
  if (review.error) warnings.push(`review.json: ${review.error}`);

  return { runRecord: runRecord.data, contract: contract.data, source: source.data, review: review.data, knownLimitations: knownLimitations.data, warnings };
}

// --- Section renderers -------------------------------------------------------

function renderWarnings(warnings) {
  if (!warnings.length) return '';
  return `<section class="warnings">
<h2>Missing or unreadable artifacts</h2>
<ul>
${warnings.map(w => `<li>${esc(w)}</li>`).join('\n')}
</ul>
</section>`;
}

function renderDecision(runRecord) {
  if (!runRecord) return '<section class="empty"><h2>Decision &amp; P0</h2><p>No run record available.</p></section>';
  const decisions = runRecord.decisions || [];
  const primary = decisions[0] || null;
  return `<section class="decision">
<h2>Decision &amp; P0</h2>
${primary ? `<dl>
<dt>Summary</dt><dd>${esc(primary.summary)}</dd>
${primary.evidence ? `<dt>Evidence</dt><dd>${esc(primary.evidence)}</dd>` : ''}
</dl>` : '<p>No decisions recorded.</p>'}
</section>`;
}

function renderRoll(runRecord) {
  if (!runRecord) return '<section class="empty"><h2>Assigned Roll</h2><p>No run record available.</p></section>';
  // Roll info may be in decisions, artifacts, or a top-level roll field
  const roll = runRecord.roll || null;
  const rollDecision = (runRecord.decisions || []).find(d => /roll/i.test(d.id || '') || /roll/i.test(d.summary || ''));
  if (!roll && !rollDecision) return `<section class="roll">
<h2>Assigned Roll</h2>
<p>No roll information in run record.</p>
</section>`;
  if (roll) {
    return `<section class="roll">
<h2>Assigned Roll</h2>
<dl>
${roll.direction ? `<dt>Direction</dt><dd>${esc(roll.direction)}</dd>` : ''}
${roll.structure ? `<dt>Structure</dt><dd>${esc(typeof roll.structure === 'string' ? roll.structure : JSON.stringify(roll.structure))}</dd>` : ''}
</dl>
</section>`;
  }
  return `<section class="roll">
<h2>Assigned Roll</h2>
<p>${esc(rollDecision.summary)}</p>
${rollDecision.evidence ? `<p>Evidence: ${esc(rollDecision.evidence)}</p>` : ''}
</section>`;
}

function renderQuestionTraceability(runRecord) {
  if (!runRecord) return '<section class="empty"><h2>Question-to-Element Traceability</h2><p>No run record available.</p></section>';
  const questions = runRecord.questions || null;
  if (!questions) return `<section class="traceability">
<h2>Question-to-Element Traceability</h2>
<p>No question mapping available in run record.</p>
</section>`;
  const entries = Array.isArray(questions) ? questions : Object.entries(questions).map(([k, v]) => ({ question: k, elements: v }));
  return `<section class="traceability">
<h2>Question-to-Element Traceability</h2>
<table>
<thead><tr><th>Question</th><th>Elements</th></tr></thead>
<tbody>
${entries.map(e => `<tr><td>${esc(e.question || e.id || '')}</td><td>${esc(Array.isArray(e.elements) ? e.elements.join(', ') : (e.elements || ''))}</td></tr>`).join('\n')}
</tbody>
</table>
</section>`;
}

function renderContractSourceDiff(contract, source) {
  const contractFindings = contract?.findings || [];
  const sourceFindings = source?.findings || [];
  if (!contractFindings.length && !sourceFindings.length) {
    return `<section class="diff">
<h2>Contract / Source Differences</h2>
<p>No findings from contract or source checks.</p>
</section>`;
  }
  return `<section class="diff">
<h2>Contract / Source Differences</h2>
${contractFindings.length ? `<h3>Contract findings (${contractFindings.length})</h3>
<table>
<thead><tr><th>Rule</th><th>Severity</th><th>Message</th></tr></thead>
<tbody>
${sortFindings(contractFindings).map(f => `<tr><td>${esc(f.rule)}</td><td><span class="sev ${esc(f.severity)}">${esc(f.severity)}</span></td><td>${esc(f.message || '')}</td></tr>`).join('\n')}
</tbody>
</table>` : '<p>No contract findings.</p>'}
${sourceFindings.length ? `<h3>Source findings (${sourceFindings.length})</h3>
<table>
<thead><tr><th>Rule</th><th>Severity</th><th>File</th><th>Line</th><th>Message</th></tr></thead>
<tbody>
${sortFindings(sourceFindings).map(f => `<tr><td>${esc(f.rule)}</td><td><span class="sev ${esc(f.severity)}">${esc(f.severity)}</span></td><td>${esc(f.file || '')}</td><td>${esc(f.line || '')}</td><td>${esc(f.message || '')}</td></tr>`).join('\n')}
</tbody>
</table>` : '<p>No source findings.</p>'}
</section>`;
}

function renderFreshness(runRecord) {
  if (!runRecord?.reviewValidity) {
    return `<section class="freshness">
<h2>Review Freshness</h2>
<p>No review validity information available.</p>
</section>`;
  }
  const v = runRecord.reviewValidity;
  const staleClass = v.stale ? 'stale' : 'fresh';
  const staleLabel = v.stale ? 'STALE' : 'FRESH';
  return `<section class="freshness ${staleClass}">
<h2>Review Freshness</h2>
<p class="freshness-badge ${staleClass}">${staleLabel}</p>
<dl>
${v.reviewId ? `<dt>Review ID</dt><dd>${esc(v.reviewId)}</dd>` : ''}
${v.sourceHash ? `<dt>Source hash</dt><dd><code>${esc(v.sourceHash)}</code></dd>` : ''}
${v.contractHash ? `<dt>Contract hash</dt><dd><code>${esc(v.contractHash)}</code></dd>` : ''}
</dl>
${v.stale ? '<p class="stale-warning">The source has changed since this review was performed. The findings below may no longer reflect the current state of the code.</p>' : ''}
</section>`;
}

function renderEvidence(runRecord) {
  if (!runRecord?.artifacts || !Object.keys(runRecord.artifacts).length) {
    return `<section class="evidence">
<h2>Evidence</h2>
<p>No artifact links recorded.</p>
</section>`;
  }
  const entries = Object.entries(runRecord.artifacts);
  return `<section class="evidence">
<h2>Evidence</h2>
<table>
<thead><tr><th>Artifact</th><th>Path</th><th>Hash</th></tr></thead>
<tbody>
${entries.map(([name, art]) => `<tr><td>${esc(name)}</td><td>${art.link ? `<a href="${esc(art.link)}">${esc(art.path || '')}</a>` : esc(art.path || '')}</td><td><code>${esc(art.hash || '')}</code></td></tr>`).join('\n')}
</tbody>
</table>
</section>`;
}

function renderLimitations(knownLimitations) {
  if (!knownLimitations) {
    return `<section class="limitations">
<h2>Limitations</h2>
<p>See <code>docs/known-limitations.md</code> for the maintained limitations register.</p>
</section>`;
  }
  const items = Array.isArray(knownLimitations) ? knownLimitations : (knownLimitations.items || []);
  if (!items.length) {
    return `<section class="limitations">
<h2>Limitations</h2>
<p>No structured limitations recorded.</p>
</section>`;
  }
  return `<section class="limitations">
<h2>Limitations</h2>
<ul>
${items.map(item => `<li>${esc(typeof item === 'string' ? item : (item.description || JSON.stringify(item)))}</li>`).join('\n')}
</ul>
</section>`;
}

function renderFindings(contract, source, review) {
  // Collect all findings from all sources, normalizing field names
  const allFindings = [];
  for (const f of (contract?.findings || [])) {
    allFindings.push({ ...f, rule: f.rule || f.rule_id || '', source: 'contract', disposition: f.disposition || 'pending' });
  }
  for (const f of (source?.findings || [])) {
    allFindings.push({ ...f, rule: f.rule || f.rule_id || '', source: 'source', disposition: f.disposition || 'pending' });
  }
  for (const f of (review?.findings || [])) {
    allFindings.push({ ...f, rule: f.rule || f.rule_id || '', description: f.description || f.message || '', source: 'review', disposition: f.disposition || 'pending' });
  }

  if (!allFindings.length) {
    return `<section class="findings">
<h2>Ordered Findings</h2>
<p>No findings from any source.</p>
</section>`;
  }

  const sorted = sortFindings(allFindings);
  const counts = { S1: 0, S2: 0, S3: 0, S4: 0 };
  for (const f of sorted) if (counts[f.severity] !== undefined) counts[f.severity]++;

  return `<section class="findings">
<h2>Ordered Findings</h2>
<p class="finding-summary">${counts.S1} S1, ${counts.S2} S2, ${counts.S3} S3, ${counts.S4} S4 — ${sorted.length} total</p>
<table>
<thead><tr><th>#</th><th>Severity</th><th>Rule</th><th>Source</th><th>File</th><th>Line</th><th>Description</th><th>Status</th></tr></thead>
<tbody>
${sorted.map((f, i) => `<tr class="finding-row ${esc(f.disposition)}">
<td>${i + 1}</td>
<td><span class="sev ${esc(f.severity)}">${esc(f.severity)}</span></td>
<td>${esc(f.rule || '')}</td>
<td>${esc(f.source)}</td>
<td>${esc(f.file || '')}</td>
<td>${esc(f.line || '')}</td>
<td>${esc(f.description || f.message || '')}</td>
<td><span class="disposition ${esc(f.disposition)}">${esc(f.disposition)}</span></td>
</tr>`).join('\n')}
</tbody>
</table>
</section>`;
}

function renderGateOutcome(review) {
  if (!review) return '';
  const outcome = review.outcome || review.gate?.decision || null;
  if (!outcome) return '';
  const cls = outcome === 'SHIP' ? 'ship' : outcome === 'REVISE' ? 'revise' : 'info';
  return `<section class="gate-outcome ${cls}">
<h2>Gate Outcome</h2>
<p class="gate-badge ${cls}">${esc(outcome)}</p>
${review.reasons?.length ? `<ul>${review.reasons.map(r => `<li>${esc(r)}</li>`).join('\n')}</ul>` : ''}
</section>`;
}

// --- HTML assembly -----------------------------------------------------------

function generateHtml({ runRecord, contract, source, review, knownLimitations, warnings }) {
  const title = runRecord?.runId
    ? `Review Cockpit — ${runRecord.runId}`
    : 'Review Cockpit';
  const stage = runRecord?.stage || 'unknown';
  const surfaceId = runRecord?.surfaceId || '';
  const updatedAt = runRecord?.updatedAt || runRecord?.createdAt || '';

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>${esc(title)}</title>
<style>
*, *::before, *::after { box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  line-height: 1.6;
  color: #1a1a2e;
  background: #f8f9fa;
  margin: 0;
  padding: 0;
}
header {
  background: #1a1a2e;
  color: #e0e0e0;
  padding: 1.5rem 2rem;
}
header h1 { margin: 0; font-size: 1.4rem; font-weight: 600; }
header .meta { font-size: 0.85rem; color: #a0a0b0; margin-top: 0.3rem; }
main { max-width: 960px; margin: 0 auto; padding: 1.5rem; }
section {
  background: #fff;
  border: 1px solid #dee2e6;
  border-radius: 6px;
  padding: 1.25rem 1.5rem;
  margin-bottom: 1.25rem;
}
section h2 {
  font-size: 1.1rem;
  margin: 0 0 0.75rem 0;
  padding-bottom: 0.5rem;
  border-bottom: 2px solid #e9ecef;
  color: #1a1a2e;
}
section h3 { font-size: 0.95rem; margin: 1rem 0 0.5rem; color: #495057; }
dl { margin: 0; }
dt { font-weight: 600; font-size: 0.85rem; color: #6c757d; margin-top: 0.5rem; }
dd { margin: 0.15rem 0 0 0; }
table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
thead th {
  text-align: left;
  padding: 0.5rem;
  background: #f1f3f5;
  border-bottom: 2px solid #dee2e6;
  font-weight: 600;
  color: #495057;
}
tbody td { padding: 0.4rem 0.5rem; border-bottom: 1px solid #e9ecef; vertical-align: top; }
tbody tr:hover { background: #f8f9fa; }
.sev { display: inline-block; padding: 0.1rem 0.4rem; border-radius: 3px; font-size: 0.75rem; font-weight: 700; }
.sev.S1 { background: #ffc9c9; color: #c92a2a; }
.sev.S2 { background: #ffec99; color: #e67700; }
.sev.S3 { background: #d0ebff; color: #1971c2; }
.sev.S4 { background: #e9ecef; color: #495057; }
.disposition { display: inline-block; padding: 0.1rem 0.4rem; border-radius: 3px; font-size: 0.75rem; font-weight: 600; }
.disposition.accept { background: #b2f2bb; color: #2b8a3e; }
.disposition.reject { background: #ffc9c9; color: #c92a2a; }
.disposition.defer { background: #ffec99; color: #e67700; }
.disposition.pending { background: #e9ecef; color: #868e96; }
.freshness-badge { display: inline-block; padding: 0.2rem 0.6rem; border-radius: 4px; font-weight: 700; font-size: 0.85rem; }
.freshness-badge.fresh { background: #b2f2bb; color: #2b8a3e; }
.freshness-badge.stale { background: #ffc9c9; color: #c92a2a; }
.stale-warning { color: #c92a2a; font-weight: 600; margin-top: 0.5rem; }
.freshness.stale { border-left: 4px solid #c92a2a; }
.freshness.fresh { border-left: 4px solid #2b8a3e; }
.gate-badge { display: inline-block; padding: 0.3rem 0.8rem; border-radius: 4px; font-weight: 700; font-size: 1rem; }
.gate-badge.ship { background: #b2f2bb; color: #2b8a3e; }
.gate-badge.revise { background: #ffc9c9; color: #c92a2a; }
.gate-badge.info { background: #d0ebff; color: #1971c2; }
.finding-summary { font-size: 0.9rem; color: #495057; margin-bottom: 0.75rem; }
.finding-row.accept { opacity: 0.6; }
.warnings { border-left: 4px solid #e67700; background: #fff9db; }
.warnings h2 { color: #e67700; }
.empty { opacity: 0.7; }
code { background: #f1f3f5; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.85em; }
a { color: #1971c2; }
.footer { text-align: center; font-size: 0.8rem; color: #868e96; padding: 1rem; }
</style>
</head>
<body>
<header>
<h1>${esc(title)}</h1>
<div class="meta">
${surfaceId ? `Surface: ${esc(surfaceId)}` : ''}${surfaceId && stage ? ' · ' : ''}Stage: ${esc(stage)}${updatedAt ? ` · Updated: ${esc(updatedAt)}` : ''}
</div>
</header>
<main>
${renderWarnings(warnings)}
${renderGateOutcome(review)}
${renderDecision(runRecord)}
${renderRoll(runRecord)}
${renderQuestionTraceability(runRecord)}
${renderContractSourceDiff(contract, source)}
${renderFreshness(runRecord)}
${renderEvidence(runRecord)}
${renderLimitations(knownLimitations)}
${renderFindings(contract, source, review)}
</main>
<div class="footer">Generated by considered cockpit. Static output — no JavaScript, no external dependencies.</div>
</body>
</html>`;
}

// --- Main --------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.log('Usage: node cockpit.mjs <result-directory> [--output <path>]\n\nReads JSON artifacts (run-record.json, contract.json, source.json,\nreview.json, known-limitations.json — all optional) from a result\ndirectory and writes a self-contained static HTML review cockpit. No\nJavaScript, no external dependencies, no live functionality.');
    process.exit(0);
  }
  const outputIndex = args.indexOf('--output');
  let outputPath = null;
  if (outputIndex !== -1) {
    outputPath = args[outputIndex + 1];
    if (!outputPath || outputPath.startsWith('--')) {
      console.error('Usage: node cockpit.mjs <result-directory> [--output <path>]');
      process.exit(2);
    }
  }
  const positional = args.filter((arg, i) => {
    if (arg === '--output') return false;
    if (outputIndex !== -1 && i === outputIndex + 1) return false;
    return true;
  });

  if (positional.length !== 1) {
    console.error('Usage: node cockpit.mjs <result-directory> [--output <path>]');
    process.exit(2);
  }

  return { resultDir: positional[0], outputPath };
}

async function main() {
  const { resultDir, outputPath } = parseArgs();
  const artifacts = await loadArtifacts(resultDir);
  const html = generateHtml(artifacts);
  const outPath = outputPath || join(resultDir, 'cockpit.html');
  await writeFile(outPath, html, 'utf8');
  console.log(`Cockpit written to ${outPath}`);
  if (artifacts.warnings.length) {
    console.error(`Warnings (${artifacts.warnings.length}):`);
    for (const w of artifacts.warnings) console.error(`  - ${w}`);
  }
}

// Allow importing for tests without executing main
export { esc, sortFindings, severityRank, generateHtml, loadArtifacts, readJson };

const isMain = process.argv[1] && (
  process.argv[1].endsWith('cockpit.mjs') || process.argv[1].endsWith('cockpit')
);
if (isMain) {
  main().catch(error => {
    console.error(`cockpit: ${error.message}`);
    process.exit(1);
  });
}
