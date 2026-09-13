#!/usr/bin/env node
/**
 * Shadow compatibility suite.
 *
 * This suite is intentionally stricter than the legacy snapshot runner.  It
 * runs every `shadowParity: exact` capability through the direct Node script,
 * direct Rust binary, public dispatcher with native selected, and public
 * dispatcher fallbacks.  It compares exit status, normalized byte streams,
 * parsed JSON (including key and finding order), and filesystem effects.
 *
 * The only normalization is documented below in `normalize()`: CRLF, the
 * isolated temporary root, ISO timestamps, and UUIDs.  Hashes, rule IDs,
 * severities, wording, field names, and ordering are deliberately preserved.
 */
import { spawn } from 'node:child_process';
import { chmod, cp, lstat, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join, relative } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const BIN = join(ROOT, 'bin', 'considered.mjs');
const RUST = join(ROOT, 'target', 'debug', 'considered-rs');
const MATRIX_PATH = join(ROOT, 'assets', 'templates', 'engine-capabilities.json');
const DOC_PATH = join(ROOT, 'docs', 'engine-capabilities.md');
const matrix = JSON.parse(await readFile(MATRIX_PATH, 'utf8'));

function run(command, args, { cwd, env = {} } = {}) {
  return new Promise(resolve => {
    const child = spawn(command, args, {
      cwd: cwd ?? ROOT,
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe']
    });
    let stdout = '', stderr = '';
    child.stdout.on('data', chunk => { stdout += chunk; });
    child.stderr.on('data', chunk => { stderr += chunk; });
    child.once('error', error => resolve({ code: 2, stdout, stderr: `${stderr}${error.message}` }));
    child.once('close', code => resolve({ code: code ?? 2, stdout, stderr }));
  });
}

// Authorized normalization only: platform line endings, isolated temp paths,
// timestamps, and deliberately random UUID keys. Do not normalize hashes.
function normalize(value, workspace) {
  return value
    .replace(/\r\n/g, '\n')
    .replaceAll(workspace, '__TMP__')
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[\w.:+-]*/g, '__TIMESTAMP__')
    .replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, '__UUID__');
}

async function snapshot(root) {
  const entries = [];
  async function visit(path) {
    const stat = await lstat(path);
    const name = relative(root, path).replaceAll('\\', '/') || '.';
    if (stat.isSymbolicLink()) {
      entries.push([name, 'symlink']);
      return;
    }
    if (stat.isDirectory()) {
      entries.push([name, 'directory']);
      for (const child of (await readdir(path)).sort()) await visit(join(path, child));
      return;
    }
    if (stat.isFile()) {
      entries.push([name, 'file', createHash('sha256').update(await readFile(path)).digest('hex')]);
    }
  }
  await visit(root);
  return entries;
}

function jsonWithOrder(stdout) {
  // JSON.parse does not reorder own keys, so this checks values and object key
  // insertion order as exposed by the two serializers.
  return JSON.parse(stdout);
}

function fail(message) { throw new Error(message); }

function assertEquivalent(label, expected, actual, workspace) {
  if (expected.code !== actual.code) fail(`${label}: exit ${actual.code}, expected ${expected.code}`);
  const expectedOut = normalize(expected.stdout, workspace);
  const actualOut = normalize(actual.stdout, workspace);
  const expectedErr = normalize(expected.stderr, workspace);
  const actualErr = normalize(actual.stderr, workspace);
  if (expectedOut !== actualOut) fail(`${label}: stdout differs\n--- expected\n${expectedOut}\n--- actual\n${actualOut}`);
  if (expectedErr !== actualErr) fail(`${label}: stderr differs\n--- expected\n${expectedErr}\n--- actual\n${actualErr}`);
  if (expected.stdout.trim().startsWith('{') || expected.stdout.trim().startsWith('[')) {
    try {
      const a = JSON.stringify(jsonWithOrder(expected.stdout));
      const b = JSON.stringify(jsonWithOrder(actual.stdout));
      if (a !== b) fail(`${label}: parsed JSON values or ordering differs`);
    } catch (error) {
      // NDJSON is already covered by the byte comparison above. A single JSON
      // document that parses on only one side is still a compatibility error.
      const expectedLines = expected.stdout.trim().split('\n').filter(Boolean);
      const actualLines = actual.stdout.trim().split('\n').filter(Boolean);
      try {
        expectedLines.forEach(JSON.parse);
        actualLines.forEach(JSON.parse);
      } catch {
        throw error;
      }
    }
  }
}

function assert(condition, message) { if (!condition) fail(message); }

async function makeFixture() {
  const root = await mkdtemp(join(tmpdir(), 'considered-shadow-'));
  await cp(join(ROOT, 'assets'), join(root, 'assets'), { recursive: true });
  await cp(join(ROOT, 'references'), join(root, 'references'), { recursive: true });
  await cp(join(ROOT, 'docs'), join(root, 'docs'), { recursive: true });
  await cp(join(ROOT, 'evals'), join(root, 'evals'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'contracts', 'valid-operate.tsx'), join(root, 'contract-valid.tsx'));
  await cp(join(ROOT, 'tests', 'fixtures', 'contracts', 'invalid-operate.tsx'), join(root, 'contract-invalid.tsx'));
  await cp(join(ROOT, 'tests', 'fixtures', 'contracts', 'missing-contract.tsx'), join(root, 'contract-missing.tsx'));
  await cp(join(ROOT, 'tests', 'fixtures', 'source', 'clean'), join(root, 'clean'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'source', 'violations'), join(root, 'violations'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'source', 'spacing-edge'), join(root, 'spacing-edge'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'source', 'unsupported-only'), join(root, 'unsupported-only'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'source', 'real-admin-surface'), join(root, 'real-admin-surface'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'gate'), join(root, 'gate'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'projects', 'system'), join(root, 'system'), { recursive: true });
  await cp(join(ROOT, 'tests', 'fixtures', 'projects', 'empty'), join(root, 'empty'), { recursive: true });
  return root;
}

async function fakeBinary(root, name, body) {
  const path = join(root, name);
  await writeFile(path, `#!/bin/sh\n${body}\n`, { mode: 0o755 });
  await chmod(path, 0o755);
  return path;
}

function envelope(result) {
  assert(result.stderr === '', `engine envelope leaked diagnostics to stderr: ${result.stderr}`);
  const parsed = JSON.parse(result.stdout);
  assert(parsed.schemaVersion === 'considered-engine-envelope/v1', 'missing schemaVersion');
  assert(parsed.protocolVersion === matrix.protocolVersion, 'incorrect protocolVersion');
  assert(Object.hasOwn(parsed, 'engine') && Object.hasOwn(parsed, 'fallback') && Object.hasOwn(parsed, 'result'), 'incomplete engine envelope');
  return parsed;
}

function validateMatrix() {
  assert(matrix.$schema === 'considered-engine-capabilities/v1', 'capability matrix schema is unversioned');
  assert(matrix.protocolVersion === '1.0.0', 'capability matrix protocol mismatch');
  const required = ['contract', 'gate', 'inventory', 'lint', 'validate', 'roll', 'validate-skill', 'status', 'verify', 'context', 'render', 'test', 'eval', 'check', 'install'];
  for (const command of required) {
    const entry = matrix.commands[command];
    assert(entry, `matrix missing ${command}`);
    for (const key of ['node', 'rust', 'nativeDelegation', 'nodeFallback', 'manualFallback']) assert(typeof entry[key] === 'boolean', `${command}.${key} must be boolean`);
    assert(['exact', 'conditional-exact', 'versioned-exception', 'not-applicable'].includes(entry.shadowParity), `${command} has invalid shadowParity`);
    if (entry.nativeDelegation && entry.node) assert(['exact', 'conditional-exact'].includes(entry.shadowParity), `${command} delegates native without exact shadow parity`);
    if (['conditional-exact', 'versioned-exception'].includes(entry.shadowParity)) assert(typeof entry.exception === 'string' && entry.exception.length > 12, `${command} lacks a versioned exception rationale`);
  }
}

const cases = [
  { command: 'contract', name: 'clean-success', node: ['scripts/lint-contract.mjs', 'contract-valid.tsx', '--json'], rust: ['contract', 'contract-valid.tsx', '--json'], public: ['--json', 'contract', 'contract-valid.tsx'], exit: 0 },
  { command: 'contract', name: 'gate-findings', node: ['scripts/lint-contract.mjs', 'contract-invalid.tsx', '--gate', '--json'], rust: ['contract', 'contract-invalid.tsx', '--gate', '--json'], public: ['--json', 'contract', 'contract-invalid.tsx', '--gate'], exit: 1 },
  { command: 'contract', name: 'missing-contract', node: ['scripts/lint-contract.mjs', 'contract-missing.tsx', '--json'], rust: ['contract', 'contract-missing.tsx', '--json'], public: ['--json', 'contract', 'contract-missing.tsx'], exit: 1 },
  { command: 'lint', name: 'clean-success', node: ['scripts/lint-source.mjs', 'clean/QueueSurface.tsx', '--json'], rust: ['lint', 'clean/QueueSurface.tsx', '--json'], public: ['--json', 'lint', 'clean/QueueSurface.tsx'], exit: 0 },
  { command: 'lint', name: 'findings', node: ['scripts/lint-source.mjs', 'violations', '--json'], rust: ['lint', 'violations', '--json'], public: ['--json', 'lint', 'violations'], exit: 0 },
  { command: 'lint', name: 'spacing-edge', node: ['scripts/lint-source.mjs', 'spacing-edge', '--json'], rust: ['lint', 'spacing-edge', '--json'], public: ['--json', 'lint', 'spacing-edge'], exit: 0 },
  { command: 'lint', name: 'unsupported-only', node: ['scripts/lint-source.mjs', 'unsupported-only', '--json'], rust: ['lint', 'unsupported-only', '--json'], public: ['--json', 'lint', 'unsupported-only'], exit: 2 },
  { command: 'lint', name: 'real-admin-surface', node: ['scripts/lint-source.mjs', 'real-admin-surface', '--json'], rust: ['lint', 'real-admin-surface', '--json'], public: ['--json', 'lint', 'real-admin-surface'], exit: 0 },
  { command: 'gate', name: 'ship', node: ['scripts/gate.mjs', 'gate/contract.json', 'gate/source.json', '--review', 'gate/review-ship.json', '--json'], rust: ['gate', 'gate/contract.json', 'gate/source.json', '--review', 'gate/review-ship.json', '--json'], public: ['--json', 'gate', 'gate/contract.json', 'gate/source.json', '--review', 'gate/review-ship.json'], exit: 0 },
  { command: 'gate', name: 'revise', node: ['scripts/gate.mjs', 'gate/contract.json', '--review', 'gate/review-revise.json', '--json'], rust: ['gate', 'gate/contract.json', '--review', 'gate/review-revise.json', '--json'], public: ['--json', 'gate', 'gate/contract.json', '--review', 'gate/review-revise.json'], exit: 1 },
  { command: 'gate', name: 'missing-outcome', node: ['scripts/gate.mjs', 'gate/contract.json', '--review', 'gate/review-no-outcome.json', '--json'], rust: ['gate', 'gate/contract.json', '--review', 'gate/review-no-outcome.json', '--json'], public: ['--json', 'gate', 'gate/contract.json', '--review', 'gate/review-no-outcome.json'], exit: 1 },
  { command: 'gate', name: 'unknown-outcome', node: ['scripts/gate.mjs', 'gate/contract.json', '--review', 'gate/review-unknown-outcome.json', '--json'], rust: ['gate', 'gate/contract.json', '--review', 'gate/review-unknown-outcome.json', '--json'], public: ['--json', 'gate', 'gate/contract.json', '--review', 'gate/review-unknown-outcome.json'], exit: 1 },
  { command: 'gate', name: 'stale-review', node: ['scripts/gate.mjs', 'gate/contract.json', '--review', 'gate/review-stale.json', '--json'], rust: ['gate', 'gate/contract.json', '--review', 'gate/review-stale.json', '--json'], public: ['--json', 'gate', 'gate/contract.json', '--review', 'gate/review-stale.json'], exit: 1 },
  { command: 'inventory', name: 'system', node: ['scripts/inventory.mjs', '--json', 'system'], rust: ['inventory', 'system', '--json'], public: ['--json', 'inventory', 'system'], exit: 0 },
  { command: 'inventory', name: 'empty', node: ['scripts/inventory.mjs', '--json', 'empty'], rust: ['inventory', 'empty', '--json'], public: ['--json', 'inventory', 'empty'], exit: 0 },
  { command: 'inventory', name: 'real-admin-surface', node: ['scripts/inventory.mjs', '--json', 'real-admin-surface'], rust: ['inventory', 'real-admin-surface', '--json'], public: ['--json', 'inventory', 'real-admin-surface'], exit: 0 },
  { command: 'validate', name: 'success', node: ['scripts/validate-assets.mjs'], rust: ['validate'], public: ['validate'], exit: 0 },
  { command: 'validate', name: 'global-json', node: ['scripts/validate-assets.mjs', '--json'], rust: ['validate'], public: ['--json', 'validate'], exit: 0 },
  { command: 'roll', name: 'generation-zero', node: ['scripts/roll.mjs', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0', '--json'], rust: ['roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0', '--json'], public: ['--json', 'roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0'], exit: 0 },
  { command: 'roll', name: 'reroll', node: ['scripts/roll.mjs', '--mode', 'operate', '--key', 'cns-eval0001', '--reroll', '--json'], rust: ['roll', '--mode', 'operate', '--key', 'cns-eval0001', '--reroll', '--json'], public: ['--json', 'roll', '--mode', 'operate', '--key', 'cns-eval0001', '--reroll'], exit: 0 },
  { command: 'roll', name: 'invalid-mode', node: ['scripts/roll.mjs', '--mode', 'not-a-mode', '--json'], rust: ['roll', '--mode', 'not-a-mode', '--json'], public: ['--json', 'roll', '--mode', 'not-a-mode'], exit: 2 },
  { command: 'roll', name: 'reroll-without-key', node: ['scripts/roll.mjs', '--mode', 'operate', '--reroll', '--json'], rust: ['roll', '--mode', 'operate', '--reroll', '--json'], public: ['--json', 'roll', '--mode', 'operate', '--reroll'], exit: 2 },
  { command: 'roll', name: 'deck-exhausted', node: ['scripts/roll.mjs', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '4', '--json'], rust: ['roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '4', '--json'], public: ['--json', 'roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '4'], exit: 3 }
];

const lifecycleCases = [
  { command: 'validate-skill', direct: ['validate-skill', '--root', ROOT, '--json'], public: ['--json', 'validate-skill', '--root', ROOT] },
  { command: 'status', direct: ['status', '--json'], public: ['--json', 'status'] },
  { command: 'verify', direct: ['verify', '--json'], public: ['--json', 'verify'] },
  { command: 'context', direct: ['context', '--experimental', '--json'], public: ['--json', 'context', '--experimental'] }
];

const humanCases = [
  { command: 'contract', node: ['scripts/lint-contract.mjs', 'contract-valid.tsx'], rust: ['contract', 'contract-valid.tsx'], public: ['contract', 'contract-valid.tsx'] },
  { command: 'lint', node: ['scripts/lint-source.mjs', 'violations'], rust: ['lint', 'violations'], public: ['lint', 'violations'] },
  { command: 'gate', node: ['scripts/gate.mjs', 'gate/contract.json', '--review', 'gate/review-ship.json'], rust: ['gate', 'gate/contract.json', '--review', 'gate/review-ship.json'], public: ['gate', 'gate/contract.json', '--review', 'gate/review-ship.json'] },
  { command: 'inventory', node: ['scripts/inventory.mjs', 'system'], rust: ['inventory', 'system'], public: ['inventory', 'system'] },
  { command: 'validate', node: ['scripts/validate-assets.mjs'], rust: ['validate'], public: ['validate'] },
  { command: 'roll', node: ['scripts/roll.mjs', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0'], rust: ['roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0'], public: ['roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '0'] }
];

const nativeFeatureCases = [
  { command: 'contract', direct: ['contract', 'contract-invalid.tsx', '--format', 'compact'], public: ['contract', 'contract-invalid.tsx', '--format', 'compact'] },
  { command: 'contract', direct: ['contract', 'contract-invalid.tsx', '--format', 'ndjson', '--severity', 'S1', '--max-findings', '2'], public: ['contract', 'contract-invalid.tsx', '--format', 'ndjson', '--severity', 'S1', '--max-findings', '2'] },
  { command: 'lint', direct: ['lint', 'violations', '--format', 'compact'], public: ['lint', 'violations', '--format', 'compact'] },
  { command: 'lint', direct: ['lint', 'violations', '--format', 'ndjson', '--severity', 'S2', '--max-findings', '2'], public: ['lint', 'violations', '--format', 'ndjson', '--severity', 'S2', '--max-findings', '2'] },
  { command: 'inventory', direct: ['inventory', 'system', '--format', 'compact'], public: ['inventory', 'system', '--format', 'compact'] },
  { command: 'gate', direct: ['gate', 'gate/contract.json', '--review', 'gate/review-ship.json', '--format', 'compact'], public: ['gate', 'gate/contract.json', '--review', 'gate/review-ship.json', '--format', 'compact'] },
  { command: 'status', direct: ['status', '--format', 'compact'], public: ['status', '--format', 'compact'] }
];

async function runCase(test, workspace) {
  // Compatibility commands must match from a cold start, including file
  // effects. Persistent cache/check artifacts belong only to versioned compact
  // workflows and may not leak into the legacy Node contract.
  const before = await snapshot(workspace);
  const node = await run(process.execPath, [join(ROOT, test.node[0]), ...test.node.slice(1)], { cwd: workspace });
  const nodeEffects = await snapshot(workspace);
  const rust = await run(RUST, test.rust, { cwd: workspace });
  const rustEffects = await snapshot(workspace);
  assert(node.code === test.exit && rust.code === test.exit, `${test.command}/${test.name}: unexpected primary exit`);
  assertEquivalent(`${test.command}/${test.name} direct Node/Rust`, node, rust, workspace);
  assert(JSON.stringify(before) === JSON.stringify(nodeEffects), `${test.command}/${test.name}: Node created an undeclared file effect`);
  assert(JSON.stringify(before) === JSON.stringify(rustEffects), `${test.command}/${test.name}: Rust created an undeclared cold-start file effect`);

  const publicNative = await run(process.execPath, [BIN, ...test.public], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
  assertEquivalent(`${test.command}/${test.name} public native`, rust, publicNative, workspace);

  return { node, rust };
}

async function main() {
  validateMatrix();
  const documented = await readFile(DOC_PATH, 'utf8');
  assert(documented.includes('single, versioned authority'), 'documentation does not identify the authority');
  await lstat(RUST);
  const workspace = await makeFixture();
  try {
    let count = 0;
    for (const test of cases) {
      assert(['exact', 'conditional-exact'].includes(matrix.commands[test.command].shadowParity), `${test.command} test is not authorized for exact shadow parity`);
      await runCase(test, workspace);
      count += 1;
    }

    for (const test of lifecycleCases) {
      const direct = await run(RUST, test.direct, { cwd: workspace });
      const publicNative = await run(process.execPath, [BIN, ...test.public], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
      assertEquivalent(`${test.command} public native lifecycle`, direct, publicNative, workspace);
      const wrapped = await run(process.execPath, [BIN, '--format', 'engine-json', ...test.public.slice(1)], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
      const metadata = envelope(wrapped);
      assert(metadata.engine === 'native-rust' && metadata.fallback === null, `${test.command}: engine envelope did not identify native Rust`);
      assert(JSON.stringify(metadata.result) === JSON.stringify(JSON.parse(direct.stdout)), `${test.command}: engine envelope changed lifecycle output`);
    }

    for (const test of humanCases) {
      const node = await run(process.execPath, [join(ROOT, test.node[0]), ...test.node.slice(1)], { cwd: workspace });
      const rust = await run(RUST, test.rust, { cwd: workspace });
      const publicNative = await run(process.execPath, [BIN, ...test.public], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
      assertEquivalent(`${test.command} human Node/Rust`, node, rust, workspace);
      assertEquivalent(`${test.command} human public native`, rust, publicNative, workspace);
    }

    for (const test of nativeFeatureCases) {
      const coldFeature = await run(RUST, test.direct, { cwd: workspace });
      if (test.command === 'contract' && test.direct.includes('compact')) {
        assert(JSON.parse(coldFeature.stdout).cached === false, 'cold compact contract incorrectly reported a cache hit');
      }
      const direct = await run(RUST, test.direct, { cwd: workspace });
      const publicNative = await run(process.execPath, [BIN, ...test.public], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
      assertEquivalent(`${test.command} public native feature`, direct, publicNative, workspace);
    }
    const compactCache = await run(RUST, ['contract', 'contract-invalid.tsx', '--format', 'compact'], { cwd: workspace });
    assert(JSON.parse(compactCache.stdout).cached === true, 'compact contract cache hit is not observable');

    const deterministicCases = cases.filter(test => ['contract', 'inventory', 'lint', 'roll'].includes(test.command) && test.name !== 'gate-findings');
    const deterministicBefore = await snapshot(workspace);
    for (const test of deterministicCases) {
      const baseline = await run(RUST, test.rust, { cwd: workspace });
      for (let repetition = 1; repetition < 10; repetition += 1) {
        const repeated = await run(RUST, test.rust, { cwd: workspace });
        assertEquivalent(`${test.command}/${test.name} deterministic run ${repetition + 1}`, baseline, repeated, workspace);
      }
    }
    assert(JSON.stringify(deterministicBefore) === JSON.stringify(await snapshot(workspace)), 'ten-run determinism checks changed filesystem state');

    const nodeBaseline = await run(process.execPath, [join(ROOT, 'scripts', 'lint-contract.mjs'), 'contract-valid.tsx', '--json'], { cwd: workspace });
    const legacyFallback = await run(process.execPath, [BIN, '--json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: join(workspace, 'missing-native') } });
    assertEquivalent('legacy --json fallback compatibility', nodeBaseline, legacyFallback, workspace);
    const nodeUsage = await run(process.execPath, [join(ROOT, 'scripts', 'lint-contract.mjs'), '--json'], { cwd: workspace });
    const publicUsage = await run(process.execPath, [BIN, '--json', 'contract'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
    assertEquivalent('legacy malformed input remains on Node', nodeUsage, publicUsage, workspace);
    const usageEnvelopeResult = await run(process.execPath, [BIN, '--format', 'engine-json', 'contract'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
    const usageEnvelope = JSON.parse(usageEnvelopeResult.stdout);
    assert(usageEnvelopeResult.code === 2 && usageEnvelope.engine === 'node-fallback' && usageEnvelope.fallback === 'native_not_selected', 'malformed input did not retain Node fallback metadata');
    assert(usageEnvelopeResult.stderr.includes('Usage:'), 'malformed input diagnostic did not remain on stderr');

    const nativeEnvelope = envelope(await run(process.execPath, [BIN, '--format', 'engine-json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } }));
    assert(nativeEnvelope.engine === 'native-rust' && nativeEnvelope.fallback === null, 'compatible binary did not select native Rust');
    assert(JSON.stringify(nativeEnvelope.result) === JSON.stringify(JSON.parse(nodeBaseline.stdout)), 'native engine envelope changed the command result');
    const missingEnvelope = envelope(await run(process.execPath, [BIN, '--format', 'engine-json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: join(workspace, 'missing-native') } }));
    assert(missingEnvelope.engine === 'node-fallback' && missingEnvelope.fallback === 'native_missing', 'missing binary did not report stable fallback');
    assert(JSON.stringify(missingEnvelope.result) === JSON.stringify(JSON.parse(nodeBaseline.stdout)), 'missing-binary fallback changed the Node result');
    const incompatible = await fakeBinary(workspace, 'incompatible-native', 'if [ "$1" = "--protocol-version" ]; then printf "9.9.9\\n"; exit 0; fi\nexit 99');
    const incompatibleEnvelope = envelope(await run(process.execPath, [BIN, '--format', 'engine-json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: incompatible } }));
    assert(incompatibleEnvelope.engine === 'node-fallback' && incompatibleEnvelope.fallback === 'native_protocol_incompatible', 'incompatible binary did not report stable fallback');
    assert(JSON.stringify(incompatibleEnvelope.result) === JSON.stringify(JSON.parse(nodeBaseline.stdout)), 'incompatible-binary fallback changed the Node result');
    const probeFailure = await fakeBinary(workspace, 'probe-failure-native', 'exit 7');
    const probeEnvelope = envelope(await run(process.execPath, [BIN, '--format', 'engine-json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: probeFailure } }));
    assert(probeEnvelope.engine === 'node-fallback' && probeEnvelope.fallback === 'native_probe_failed', 'probe failure did not fall back');
    assert(JSON.stringify(probeEnvelope.result) === JSON.stringify(JSON.parse(nodeBaseline.stdout)), 'probe-failure fallback changed the Node result');
    const reviseEnvelopeResult = await run(process.execPath, [BIN, '--format', 'engine-json', 'gate', 'gate/contract.json', '--review', 'gate/review-revise.json'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
    const reviseEnvelope = JSON.parse(reviseEnvelopeResult.stdout);
    assert(reviseEnvelopeResult.code === 1 && reviseEnvelope.engine === 'native-rust' && reviseEnvelope.fallback === null && !reviseEnvelope.error, 'findings exit was mislabeled as an engine failure');
    const exhaustedEnvelopeResult = await run(process.execPath, [BIN, '--format', 'engine-json', 'roll', '--mode', 'operate', '--key', 'cns-eval0001', '--gen', '4'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
    const exhaustedEnvelope = JSON.parse(exhaustedEnvelopeResult.stdout);
    assert(exhaustedEnvelopeResult.code === 3 && exhaustedEnvelope.engine === 'native-rust' && exhaustedEnvelope.fallback === null && !exhaustedEnvelope.error, 'deck exhaustion was mislabeled as an engine failure');
    const childFailure = await fakeBinary(workspace, 'child-failure-native', 'if [ "$1" = "--protocol-version" ]; then printf "1.0.0\\n"; exit 0; fi\nprintf "simulated native failure\\n" >&2\nexit 70');
    const failedNative = await run(process.execPath, [BIN, '--format', 'engine-json', 'contract', 'contract-valid.tsx'], { cwd: workspace, env: { CONSIDERED_RS_BIN: childFailure } });
    assert(failedNative.code === 70, `native child failure did not propagate exit 70 (${failedNative.code})`);
    assert(failedNative.stderr.includes('simulated native failure'), 'native child diagnostics were not kept on stderr');
    const failedEnvelope = JSON.parse(failedNative.stdout);
    assert(failedEnvelope.engine === 'native-rust' && failedEnvelope.fallback === 'native_child_failed' && failedEnvelope.error?.code === 'native_child_failed', 'native child failure metadata is inaccurate');
    const manual = await run(process.execPath, [BIN, '--format', 'engine-json', 'validate-skill', '--root', '.'], { cwd: workspace, env: { CONSIDERED_RS_BIN: join(workspace, 'no-manual-native') } });
    const manualEnvelope = JSON.parse(manual.stdout);
    assert(manual.code === 2 && manualEnvelope.engine === 'manual-unavailable' && manualEnvelope.fallback === 'manual_unavailable', 'manual capability state is not explicit');
    for (const command of ['status', 'verify', 'context']) {
      const args = command === 'context' ? [command, '--experimental'] : [command];
      const unavailable = await run(process.execPath, [BIN, '--format', 'engine-json', ...args], { cwd: workspace, env: { CONSIDERED_RS_BIN: join(workspace, `no-${command}-native`) } });
      const unavailableEnvelope = JSON.parse(unavailable.stdout);
      assert(unavailable.code === 2 && unavailableEnvelope.engine === 'manual-unavailable' && unavailableEnvelope.fallback === 'manual_unavailable', `${command}: missing native companion was not explicit`);
    }

    const compact = await run(process.execPath, [BIN, 'contract', 'contract-valid.tsx', '--format', 'compact'], { cwd: workspace, env: { CONSIDERED_RS_BIN: RUST } });
    assert(compact.code === 0 && compact.stderr === '' && compact.stdout.length < 512, 'native compact output is not compact and clean');
    JSON.parse(compact.stdout);

    let mismatchDetected = false;
    try { assertEquivalent('deliberate mismatch control', { code: 0, stdout: 'a\n', stderr: '' }, { code: 0, stdout: 'b\n', stderr: '' }, workspace); } catch { mismatchDetected = true; }
    assert(mismatchDetected, 'deliberate mismatch control was not detected');
    const counts = Object.entries(cases.reduce((acc, test) => ({ ...acc, [test.command]: (acc[test.command] || 0) + 1 }), {}))
      .map(([command, total]) => `${command}: ${total}`)
      .join(', ');
    console.log(`shadow compatibility passed: ${count} exact JSON/error cases (${counts}), ${humanCases.length} exact human-output cases, ${nativeFeatureCases.length} public native feature cases, ${lifecycleCases.length} native lifecycle commands, ten-run determinism, dispatcher selection states, deliberate mismatch detected`);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
}

await main();
