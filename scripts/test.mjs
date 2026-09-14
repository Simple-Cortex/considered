#!/usr/bin/env node
/** Dependency-free fixture tests for maintained CLI behavior. */
import { readFile, mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  detectProjectHost, detectGlobalHost, planChoices, resolveCustomDest, resolveYesPlan,
  expandHome, interpretUpdateAnswer, interpretConfirmAnswer
} from './wizard-plan.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
if (process.argv.slice(2).some(arg => arg === '--help' || arg === '-h')) {
  console.log('Usage: node test.mjs\n\nRuns the dependency-free fixture test suite for maintained CLI behavior\n(wizard planning, install, and script-level regressions). Takes no arguments.');
  process.exit(0);
}
const FIXTURES = join(ROOT, 'tests', 'fixtures');
let failed = 0;
const pass = message => console.log(`pass  ${message}`);
const fail = message => { failed++; console.error(`FAIL  ${message}`); };

function run(script, args = []) {
  return new Promise(resolve => {
    const child = spawn(process.execPath, [join(HERE, script), ...args], { cwd: ROOT });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => stdout += chunk);
    child.stderr.on('data', chunk => stderr += chunk);
    child.on('error', error => resolve({ code: 2, stdout, stderr: `${stderr}${error.message}` }));
    child.on('close', code => resolve({ code: code ?? 2, stdout, stderr }));
  });
}
function nested(object, path) { return path.split('.').reduce((value, key) => value?.[key], object); }
function containsFinding(report, expected) { return report.findings?.some(f => f.rule === expected.rule && f.severity === expected.severity); }

const manifest = JSON.parse(await readFile(join(FIXTURES, 'manifest.json'), 'utf8'));
const asset = await run('validate-assets.mjs');
if (asset.code === 0) pass('asset manifests and decks validate'); else fail(`asset validation: ${asset.stderr || asset.stdout}`);

for (const test of manifest.lint_contract) {
  const before = failed;
  const result = await run('lint-contract.mjs', [join(FIXTURES, test.target), '--json']);
  if (result.code !== test.expected_exit) fail(`contract ${test.id}: expected exit ${test.expected_exit}, received ${result.code}`);
  let report;
  try { report = JSON.parse(result.stdout); } catch { fail(`contract ${test.id}: output is not JSON`); continue; }
  const required = test.expected_findings || test.required_findings || [];
  for (const finding of required) if (!containsFinding(report, finding)) fail(`contract ${test.id}: missing ${finding.rule} ${finding.severity}`);
  if (!required.length && report.findings.length) fail(`contract ${test.id}: expected no findings, received ${report.findings.length}`);
  if (failed === before) pass(`contract ${test.id}`);
}

for (const test of manifest.lint_source) {
  const before = failed;
  const result = await run('lint-source.mjs', [join(FIXTURES, test.target), '--json']);
  if (result.code !== test.expected_exit) fail(`source ${test.id}: expected exit ${test.expected_exit}, received ${result.code}`);
  let report;
  try { report = JSON.parse(result.stdout); } catch { fail(`source ${test.id}: output is not JSON`); continue; }
  const required = test.expected_findings || test.required_findings || [];
  for (const finding of required) if (!containsFinding(report, finding)) fail(`source ${test.id}: missing ${finding.rule} ${finding.severity}`);
  if (!required.length && report.findings.length) fail(`source ${test.id}: expected no findings, received ${report.findings.length}`);
  if (failed === before) pass(`source ${test.id}`);
}

for (const test of manifest.inventory) {
  const before = failed;
  const result = await run('inventory.mjs', ['--json', join(FIXTURES, test.target)]);
  if (result.code !== test.expected_exit) fail(`inventory ${test.id}: expected exit ${test.expected_exit}, received ${result.code}`);
  let report;
  try { report = JSON.parse(result.stdout); } catch { fail(`inventory ${test.id}: output is not JSON`); continue; }
  for (const path of test.required_json_properties || []) if (nested(report, path) === undefined) fail(`inventory ${test.id}: missing ${path}`);
  if (failed === before) pass(`inventory ${test.id}`);
}

const rollCases = JSON.parse(await readFile(join(FIXTURES, manifest.roll_cases), 'utf8')).cases;
const deterministic = rollCases.find(test => test.id === 'deterministic-operate');
if (deterministic) {
  const first = await run('roll.mjs', deterministic.args);
  const second = await run('roll.mjs', deterministic.args);
  if (first.code !== 0 || second.code !== 0) fail('roll deterministic-operate: expected successful draws');
  else {
    const a = JSON.parse(first.stdout), b = JSON.parse(second.stdout);
    if (JSON.stringify(a) !== JSON.stringify(b)) fail('roll deterministic-operate: repeated draw changed');
    else if (a.mode !== 'operate' || !a.direction || Object.keys(a.structure || {}).length !== 3) fail('roll deterministic-operate: malformed hand');
    else pass('roll deterministic-operate');
  }
}
const reroll = rollCases.find(test => test.id === 'reroll-excludes-prior-deal');
if (reroll && deterministic) {
  const prior = JSON.parse((await run('roll.mjs', deterministic.args)).stdout);
  const nextResult = await run('roll.mjs', reroll.args);
  if (nextResult.code !== 0) fail('roll reroll-excludes-prior-deal: expected successful draw');
  else {
    const next = JSON.parse(nextResult.stdout);
    const priorIds = new Set([...Object.values(prior.structure).map(entry => entry.id), prior.direction.id]);
    const nextIds = [...Object.values(next.structure).map(entry => entry.id), next.direction.id];
    if (nextIds.some(id => priorIds.has(id))) fail('roll reroll-excludes-prior-deal: repeated card'); else pass('roll reroll-excludes-prior-deal');
  }
}
for (const test of rollCases.filter(test => /rejected/.test(test.id))) {
  const result = await run('roll.mjs', test.args);
  if (result.code !== 2) fail(`roll ${test.id}: expected usage exit 2, received ${result.code}`); else pass(`roll ${test.id}`);
}

const render = await run('lint-render.mjs', [join(ROOT, 'assets', 'templates', 'RENDER-EVIDENCE.json'), '--json']);
if (render.code === 0) pass('render evidence template validates'); else fail(`render evidence template: ${render.stderr || render.stdout}`);

const gateDir = join(FIXTURES, 'gate');
const gateShip = await run('gate.mjs', [join(gateDir, 'contract.json'), join(gateDir, 'source.json'), '--review', join(gateDir, 'review-ship.json'), '--json']);
if (gateShip.code === 0) pass('gate accepts a reviewed clean high-confidence report'); else fail(`gate ship fixture: ${gateShip.stderr || gateShip.stdout}`);
const gateRevise = await run('gate.mjs', [join(gateDir, 'contract.json'), '--review', join(gateDir, 'review-revise.json'), '--json']);
if (gateRevise.code === 1) pass('gate rejects a revise review'); else fail(`gate revise fixture: expected exit 1, received ${gateRevise.code}`);

// A review that states no verdict, or an uninterpretable one, must never ship.
// Both cases previously passed the gate on score alone, letting a build self-certify.
for (const [fixture, label] of [
  ['review-no-outcome.json', 'gate refuses a review with no stated outcome'],
  ['review-unknown-outcome.json', 'gate refuses a review with an unrecognized outcome']
]) {
  const result = await run('gate.mjs', [join(gateDir, 'contract.json'), join(gateDir, 'source.json'), '--review', join(gateDir, fixture), '--json']);
  let parsed = null;
  try { parsed = JSON.parse(result.stdout); } catch { /* reported below */ }
  if (result.code === 1 && parsed?.outcome === 'REVIEW-REQUIRED') pass(label);
  else fail(`${label}: expected exit 1 and REVIEW-REQUIRED, received ${result.code} and ${parsed?.outcome ?? 'unparseable output'}`);
}
const gateStale = await run('gate.mjs', [join(gateDir, 'contract.json'), '--review', join(gateDir, 'review-stale.json'), '--json']);
let staleOutput = null;
try { staleOutput = JSON.parse(gateStale.stdout); } catch { /* reported below */ }
if (gateStale.code === 1 && staleOutput?.outcome === 'REVISE' && staleOutput.reasons?.some(reason => reason.includes('stale'))) {
  pass('gate rejects a stale independent review');
} else {
  fail(`gate stale fixture: expected exit 1 and a stale reason, received ${gateStale.code}`);
}

// --- install wizard: detection, planning, non-TTY behavior, --dest/--yes regression ---
async function withTempDir(run) {
  const dir = await mkdtemp(join(tmpdir(), 'considered-wizard-'));
  try { return await run(dir); } finally { await rm(dir, { recursive: true, force: true }); }
}

// Detection matrix: cwd with .claude/, CLAUDE.md-only, and neither.
await withTempDir(async claudeDirCwd => {
  await mkdir(join(claudeDirCwd, '.claude'));
  if (detectProjectHost(claudeDirCwd)) pass('wizard detects .claude/ directory as project host'); else fail('wizard detects .claude/ directory as project host');
});
await withTempDir(async claudeMdCwd => {
  await writeFile(join(claudeMdCwd, 'CLAUDE.md'), '# project\n');
  if (detectProjectHost(claudeMdCwd)) pass('wizard detects CLAUDE.md-only as project host'); else fail('wizard detects CLAUDE.md-only as project host');
});
await withTempDir(async bareCwd => {
  if (!detectProjectHost(bareCwd)) pass('wizard finds no project host in a bare directory'); else fail('wizard finds no project host in a bare directory');
});
await withTempDir(async home => {
  if (!detectGlobalHost(home)) pass('wizard finds no global host without ~/.claude/skills'); else fail('wizard finds no global host without ~/.claude/skills');
  await mkdir(join(home, '.claude', 'skills'), { recursive: true });
  if (detectGlobalHost(home)) pass('wizard detects an existing ~/.claude/skills as global host'); else fail('wizard detects an existing ~/.claude/skills as global host');
});

// Plan resolution: project/global/custom destinations and the detected default.
await withTempDir(async cwd => withTempDir(async home => {
  await mkdir(join(cwd, '.claude'));
  const plan = planChoices({ cwd, homedir: home });
  if (plan.defaultChoice === '1' && plan.choices[0].dest === join(cwd, '.claude', 'skills', 'considered')) pass('wizard plan defaults to project destination when detected');
  else fail(`wizard plan defaults to project destination when detected: got ${JSON.stringify(plan)}`);
  if (plan.choices[1].dest === join(home, '.claude', 'skills', 'considered')) pass('wizard plan resolves the global destination'); else fail('wizard plan resolves the global destination');
}));
await withTempDir(async cwd => withTempDir(async home => {
  const plan = planChoices({ cwd, homedir: home });
  if (plan.defaultChoice === null) pass('wizard plan has no default when no host is detected'); else fail('wizard plan has no default when no host is detected');
}));
{
  const custom = resolveCustomDest('/opt/some-host/skills');
  if (custom === join('/opt/some-host/skills', 'considered')) pass('wizard resolves a custom host destination'); else fail(`wizard resolves a custom host destination: got ${custom}`);
}

// Custom-host destination: tilde expansion against a supplied homedir, plus non-tilde forms left alone.
{
  const home = '/Users/example';
  const cases = [
    ['~/x', join(home, 'x')],
    ['~', home],
    ['relative/dir', 'relative/dir'],
    ['../sibling/dir', '../sibling/dir'],
    ['/abs/path', '/abs/path']
  ];
  for (const [input, expected] of cases) {
    const got = expandHome(input, home);
    if (got === expected) pass(`expandHome(${JSON.stringify(input)}) resolves against homedir`);
    else fail(`expandHome(${JSON.stringify(input)}): expected ${expected}, got ${got}`);
  }
}

// Overwrite-guard mapping: the y/N interpretation deciding whether --force is passed.
// The two prompts have opposite default polarity — verify the full mapping in both directions,
// including the empty-string default, so inverting either would fail here.
{
  const updateCases = [
    ['', false], ['n', false], ['N', false], ['no', false], ['garbage', false],
    ['y', true], ['Y', true], ['yes', true], ['YES', true], ['  y  ', true]
  ];
  for (const [answer, expected] of updateCases) {
    const got = interpretUpdateAnswer(answer);
    if (got === expected) pass(`interpretUpdateAnswer(${JSON.stringify(answer)}) === ${expected}`);
    else fail(`interpretUpdateAnswer(${JSON.stringify(answer)}): expected ${expected}, got ${got}`);
  }
  const confirmCases = [
    ['', true], ['y', true], ['Y', true], ['yes', true], ['garbage', true],
    ['n', false], ['N', false], ['no', false], ['NO', false], ['  n  ', false]
  ];
  for (const [answer, expected] of confirmCases) {
    const got = interpretConfirmAnswer(answer);
    if (got === expected) pass(`interpretConfirmAnswer(${JSON.stringify(answer)}) === ${expected}`);
    else fail(`interpretConfirmAnswer(${JSON.stringify(answer)}): expected ${expected}, got ${got}`);
  }
}

// --yes: both branches (confident default present / absent).
await withTempDir(async cwd => {
  await mkdir(join(cwd, '.claude'));
  const plan = resolveYesPlan({ cwd });
  if (plan?.dest === join(cwd, '.claude', 'skills', 'considered')) pass('resolveYesPlan proceeds when a project host is detected'); else fail('resolveYesPlan proceeds when a project host is detected');
});
await withTempDir(async cwd => {
  const plan = resolveYesPlan({ cwd });
  if (plan === null) pass('resolveYesPlan returns null with no confident default'); else fail('resolveYesPlan returns null with no confident default');
});

// Non-TTY bare invocation never hangs and exits 2 (usage error, per the repo's documented
// exit-code contract) with usage, not the wizard.
{
  const bare = await new Promise(resolveRun => {
    const child = spawn(process.execPath, [join(ROOT, 'bin', 'considered.mjs')], { cwd: ROOT, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => stdout += chunk);
    child.stderr.on('data', chunk => stderr += chunk);
    child.on('close', code => resolveRun({ code, stdout, stderr }));
  });
  if (bare.code === 2 && /no interactive terminal/.test(bare.stderr)) pass('non-TTY bare invocation exits 2 with usage, never the wizard');
  else fail(`non-TTY bare invocation: expected exit 2 with usage, got code ${bare.code}, stderr ${bare.stderr}`);
}

// install --dest regression (unchanged CI path) and the new --yes flag, via the bin entry point.
await withTempDir(async destParent => {
  const dest = join(destParent, 'considered');
  const result = await run('..' + '/bin/considered.mjs', ['install', '--dest', dest, '--dry-run']);
  if (result.code === 0 && /Would install/.test(result.stdout)) pass('install --dest regression: dry-run reports the destination');
  else fail(`install --dest regression: code ${result.code}, stdout ${result.stdout}, stderr ${result.stderr}`);
});
await withTempDir(async cwd => {
  await mkdir(join(cwd, '.claude'));
  const result = await new Promise(resolveRun => {
    const child = spawn(process.execPath, [join(ROOT, 'scripts', 'install.mjs'), '--yes', '--dry-run'], { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => stdout += chunk);
    child.stderr.on('data', chunk => stderr += chunk);
    child.on('close', code => resolveRun({ code, stdout, stderr }));
  });
  if (result.code === 0 && result.stdout.includes(join(cwd, '.claude', 'skills', 'considered'))) pass('install --yes accepts a detected project default');
  else fail(`install --yes accepts a detected project default: code ${result.code}, stdout ${result.stdout}, stderr ${result.stderr}`);
});
await withTempDir(async cwd => {
  const result = await new Promise(resolveRun => {
    const child = spawn(process.execPath, [join(ROOT, 'scripts', 'install.mjs'), '--yes'], { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => stdout += chunk);
    child.stderr.on('data', chunk => stderr += chunk);
    child.on('close', code => resolveRun({ code, stdout, stderr }));
  });
  if (result.code === 2 && /no confident default/.test(result.stderr)) pass('install --yes exits 2 with no confident default');
  else fail(`install --yes exits 2 with no confident default: code ${result.code}, stderr ${result.stderr}`);
});

if (failed) {
  console.error(`\n${failed} test failure(s).`);
  process.exit(1);
}
console.log('\nAll fixture tests passed.');
