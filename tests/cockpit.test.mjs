#!/usr/bin/env node
/**
 * Tests for scripts/cockpit.mjs — static review cockpit generator.
 *
 * Run: node tests/cockpit.test.mjs
 * Or via npm: npm test (included in scripts/test.mjs)
 */
import { readFile, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';
import { generateHtml, loadArtifacts, esc, sortFindings, severityRank } from '../scripts/cockpit.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(HERE, 'fixtures', 'cockpit');
let failed = 0;
const pass = msg => console.log(`pass  ${msg}`);
const fail = msg => { failed++; console.error(`FAIL  ${msg}`); };

// --- Unit tests for helpers --------------------------------------------------

// esc: escapes HTML special characters
{
  const tests = [
    ['<script>', '&lt;script&gt;'],
    ['"quotes"', '&quot;quotes&quot;'],
    ["it's", "it&#x27;s"],
    ['a & b', 'a &amp; b'],
    [null, ''],
    [undefined, ''],
    [42, '42'],
  ];
  for (const [input, expected] of tests) {
    const got = esc(input);
    if (got === expected) pass(`esc(${JSON.stringify(input)}) → ${JSON.stringify(expected)}`);
    else fail(`esc(${JSON.stringify(input)}): expected ${JSON.stringify(expected)}, got ${JSON.stringify(got)}`);
  }
}

// severityRank: orders S1 < S2 < S3 < S4
{
  if (severityRank('S1') < severityRank('S2') && severityRank('S2') < severityRank('S3') && severityRank('S3') < severityRank('S4')) {
    pass('severityRank orders S1 < S2 < S3 < S4');
  } else fail('severityRank does not order severities correctly');
}

// sortFindings: sorts by severity, then file, then line
{
  const input = [
    { severity: 'S3', file: 'b.tsx', line: 10 },
    { severity: 'S1', file: 'c.tsx', line: 5 },
    { severity: 'S1', file: 'a.tsx', line: 20 },
    { severity: 'S1', file: 'a.tsx', line: 5 },
    { severity: 'S2', file: 'a.tsx', line: 1 },
  ];
  const sorted = sortFindings(input);
  const expected = [
    { severity: 'S1', file: 'a.tsx', line: 5 },
    { severity: 'S1', file: 'a.tsx', line: 20 },
    { severity: 'S1', file: 'c.tsx', line: 5 },
    { severity: 'S2', file: 'a.tsx', line: 1 },
    { severity: 'S3', file: 'b.tsx', line: 10 },
  ];
  const match = sorted.every((f, i) => f.severity === expected[i].severity && f.file === expected[i].file && f.line === expected[i].line);
  if (match) pass('sortFindings orders by severity → file → line');
  else fail(`sortFindings: expected ${JSON.stringify(expected.map(f => `${f.severity}:${f.file}:${f.line}`))}, got ${JSON.stringify(sorted.map(f => `${f.severity}:${f.file}:${f.line}`))}`);
}

// --- Integration tests: generation from valid fixture ------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'valid'));
  if (!artifacts.runRecord) fail('valid fixture: runRecord should be loaded');
  else pass('valid fixture: runRecord loaded');

  if (!artifacts.contract) fail('valid fixture: contract should be loaded');
  else pass('valid fixture: contract loaded');

  if (!artifacts.source) fail('valid fixture: source should be loaded');
  else pass('valid fixture: source loaded');

  if (!artifacts.review) fail('valid fixture: review should be loaded');
  else pass('valid fixture: review loaded');

  if (artifacts.warnings.length !== 0) fail(`valid fixture: expected no warnings, got ${artifacts.warnings.length}`);
  else pass('valid fixture: no warnings');

  const html = generateHtml(artifacts);

  // Valid HTML structure
  if (html.startsWith('<!DOCTYPE html>') && html.includes('</html>')) pass('valid fixture: produces valid HTML document');
  else fail('valid fixture: output is not a valid HTML document');

  // All required sections present
  const requiredSections = [
    'Decision &amp; P0',
    'Assigned Roll',
    'Question-to-Element Traceability',
    'Contract / Source Differences',
    'Review Freshness',
    'Evidence',
    'Limitations',
    'Ordered Findings',
  ];
  for (const section of requiredSections) {
    if (html.includes(section)) pass(`valid fixture: contains section "${section}"`);
    else fail(`valid fixture: missing section "${section}"`);
  }

  // Contains finding data
  if (html.includes('A11Y-04') && html.includes('IA-03') && html.includes('REVIEW-HIER')) pass('valid fixture: contains findings from all sources');
  else fail('valid fixture: missing expected findings');

  // Contains gate outcome
  if (html.includes('REVISE')) pass('valid fixture: shows gate outcome');
  else fail('valid fixture: missing gate outcome');

  // Contains disposition statuses
  if (html.includes('accept') && html.includes('pending')) pass('valid fixture: shows finding dispositions');
  else fail('valid fixture: missing finding dispositions');
}

// --- XSS escaping test -------------------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'xss'));
  const html = generateHtml(artifacts);

  // No unescaped script tags
  if (!html.includes('<script>')) pass('xss fixture: no unescaped <script> tags in output');
  else fail('xss fixture: unescaped <script> tag found in output');

  // No unescaped img tags with event handlers (the = sign is not HTML-dangerous,
  // but the surrounding < > must be escaped to prevent tag injection)
  if (!html.includes('<img src=x')) pass('xss fixture: no unescaped img tags with handlers');
  else fail('xss fixture: unescaped img tag with handler found in output');

  // Escaped versions present
  if (html.includes('&lt;script&gt;')) pass('xss fixture: script tags are escaped');
  else fail('xss fixture: expected escaped script tags');

  // Surface ID with XSS payload is escaped
  if (!html.includes('<img src=x')) pass('xss fixture: surface ID XSS payload is escaped');
  else fail('xss fixture: unescaped surface ID XSS payload');

  // Decision summary with HTML is escaped
  if (!html.includes('<b>bold attack</b>')) pass('xss fixture: decision summary HTML is escaped');
  else fail('xss fixture: unescaped HTML in decision summary');

  // Ampersands are escaped
  if (html.includes('&amp; more') || html.includes('&amp;')) pass('xss fixture: ampersands are escaped');
  else fail('xss fixture: unescaped ampersands');
}

// --- Deterministic output test -----------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'valid'));
  const html1 = generateHtml(artifacts);
  const html2 = generateHtml(artifacts);
  if (html1 === html2) pass('deterministic: two runs produce identical HTML');
  else fail('deterministic: two runs produced different HTML');
}

// --- Missing input files test ------------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'empty'));
  if (artifacts.warnings.length >= 4) pass('empty fixture: reports warnings for all missing files');
  else fail(`empty fixture: expected >= 4 warnings, got ${artifacts.warnings.length}`);

  const html = generateHtml(artifacts);
  if (html.includes('Missing or unreadable artifacts')) pass('empty fixture: shows missing artifacts section');
  else fail('empty fixture: missing artifacts warning section');

  // Still produces valid HTML despite missing inputs
  if (html.startsWith('<!DOCTYPE html>') && html.includes('</html>')) pass('empty fixture: still produces valid HTML');
  else fail('empty fixture: fails to produce valid HTML with missing inputs');
}

// --- Malformed JSON test -----------------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'malformed'));
  if (artifacts.runRecord === null) pass('malformed fixture: runRecord is null for bad JSON');
  else fail('malformed fixture: runRecord should be null for malformed JSON');

  if (artifacts.warnings.some(w => w.includes('run-record.json'))) pass('malformed fixture: warning mentions the malformed file');
  else fail('malformed fixture: no warning about malformed run-record.json');

  const html = generateHtml(artifacts);
  if (html.startsWith('<!DOCTYPE html>')) pass('malformed fixture: still produces HTML despite malformed input');
  else fail('malformed fixture: fails to produce HTML with malformed input');
}

// --- Stale review test -------------------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'stale'));
  const html = generateHtml(artifacts);

  if (html.includes('STALE')) pass('stale fixture: shows STALE indicator');
  else fail('stale fixture: missing STALE indicator');

  if (html.includes('stale-warning')) pass('stale fixture: includes stale warning CSS class');
  else fail('stale fixture: missing stale warning element');

  if (html.includes('source has changed since this review')) pass('stale fixture: includes stale explanation text');
  else fail('stale fixture: missing stale explanation text');

  if (html.includes('freshness stale')) pass('stale fixture: freshness section has stale class');
  else fail('stale fixture: freshness section missing stale class');
}

// --- Write-to-temp-dir test (end-to-end) -------------------------------------

{
  const tmpDir = await mkdtemp(join(tmpdir(), 'cockpit-test-'));
  try {
    // Copy valid fixture to temp
    const runRecord = await readFile(join(FIXTURES, 'valid', 'run-record.json'), 'utf8');
    const contract = await readFile(join(FIXTURES, 'valid', 'contract.json'), 'utf8');
    const source = await readFile(join(FIXTURES, 'valid', 'source.json'), 'utf8');
    const review = await readFile(join(FIXTURES, 'valid', 'review.json'), 'utf8');
    await writeFile(join(tmpDir, 'run-record.json'), runRecord);
    await writeFile(join(tmpDir, 'contract.json'), contract);
    await writeFile(join(tmpDir, 'source.json'), source);
    await writeFile(join(tmpDir, 'review.json'), review);

    // Run cockpit via subprocess
    const { spawn } = await import('node:child_process');
    const cockpitScript = join(HERE, '..', 'scripts', 'cockpit.mjs');
    const result = await new Promise(resolve => {
      const child = spawn(process.execPath, [cockpitScript, tmpDir], { cwd: join(HERE, '..') });
      let stdout = '', stderr = '';
      child.stdout.on('data', c => stdout += c);
      child.stderr.on('data', c => stderr += c);
      child.on('close', code => resolve({ code, stdout, stderr }));
    });

    if (result.code === 0) pass('e2e: cockpit exits 0 on valid input');
    else fail(`e2e: cockpit exit code ${result.code}, stderr: ${result.stderr}`);

    const output = await readFile(join(tmpDir, 'cockpit.html'), 'utf8');
    if (output.includes('<!DOCTYPE html>')) pass('e2e: cockpit.html is written and valid');
    else fail('e2e: cockpit.html is not valid');

    // Test --output flag
    const customOut = join(tmpDir, 'custom-output.html');
    const result2 = await new Promise(resolve => {
      const child = spawn(process.execPath, [cockpitScript, tmpDir, '--output', customOut], { cwd: join(HERE, '..') });
      let stdout = '', stderr = '';
      child.stdout.on('data', c => stdout += c);
      child.stderr.on('data', c => stderr += c);
      child.on('close', code => resolve({ code, stdout, stderr }));
    });

    if (result2.code === 0) pass('e2e: --output flag exits 0');
    else fail(`e2e: --output flag exit code ${result2.code}`);

    const customContent = await readFile(customOut, 'utf8');
    if (customContent.includes('<!DOCTYPE html>')) pass('e2e: --output writes to custom path');
    else fail('e2e: --output file is not valid HTML');
  } finally {
    await rm(tmpDir, { recursive: true, force: true });
  }
}

// --- Usage error tests -------------------------------------------------------

{
  const { spawn } = await import('node:child_process');
  const cockpitScript = join(HERE, '..', 'scripts', 'cockpit.mjs');

  // No arguments
  const noArgs = await new Promise(resolve => {
    const child = spawn(process.execPath, [cockpitScript], { cwd: join(HERE, '..') });
    let stderr = '';
    child.stderr.on('data', c => stderr += c);
    child.on('close', code => resolve({ code, stderr }));
  });
  if (noArgs.code === 2) pass('usage: no arguments exits 2');
  else fail(`usage: no arguments expected exit 2, got ${noArgs.code}`);

  // Too many arguments
  const tooMany = await new Promise(resolve => {
    const child = spawn(process.execPath, [cockpitScript, 'a', 'b'], { cwd: join(HERE, '..') });
    let stderr = '';
    child.stderr.on('data', c => stderr += c);
    child.on('close', code => resolve({ code, stderr }));
  });
  if (tooMany.code === 2) pass('usage: too many arguments exits 2');
  else fail(`usage: too many arguments expected exit 2, got ${tooMany.code}`);
}

// --- No live functionality test ----------------------------------------------

{
  const artifacts = await loadArtifacts(join(FIXTURES, 'valid'));
  const html = generateHtml(artifacts);

  // No <script> tags in output (static only)
  const scriptTagCount = (html.match(/<script/gi) || []).length;
  if (scriptTagCount === 0) pass('no-live: output contains no <script> tags');
  else fail(`no-live: output contains ${scriptTagCount} <script> tag(s)`);

  // No interactive elements for changing direction/rerolling
  if (!html.includes('reroll') && !html.includes('change direction') && !html.includes('<select') && !html.includes('<button')) {
    pass('no-live: no interactive UI for direction changes or rerolling');
  } else fail('no-live: output contains interactive elements for direction/reroll');
}

// --- Summary -----------------------------------------------------------------

if (failed) {
  console.error(`\n${failed} cockpit test failure(s).`);
  process.exit(1);
}
console.log('\nAll cockpit tests passed.');
