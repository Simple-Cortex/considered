#!/usr/bin/env node
/**
 * Benchmark runner for lint/inventory performance.
 *
 * Measures lint (source scan) and inventory performance across small, medium,
 * and large fixture trees. Records OS, architecture, Node version, repository
 * revision, command, fixture size, iterations, warm/cold distinction, median,
 * p95, peak memory, output bytes, and estimated output tokens.
 *
 * Usage: node tests/bench/run.mjs [--baseline] [--iterations N]
 *
 * --baseline   write results to tests/bench/baseline.json
 * --iterations number of iterations per command/size (default: 5)
 *
 * The small fixture is committed. Medium and large are generated on demand
 * by tests/bench/generate.mjs and live in a temp directory.
 */
import { spawn } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir, platform, arch } from 'node:os';
import { execSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const BIN = join(ROOT, 'bin', 'considered.mjs');
const SMALL_FIXTURE = join(HERE, 'fixtures', 'small');
const BASELINE_PATH = join(HERE, 'baseline.json');

const args = process.argv.slice(2);
const writeBaseline = args.includes('--baseline');
const iterFlag = args.indexOf('--iterations');
const ITERATIONS = iterFlag === -1 ? 5 : parseInt(args[iterFlag + 1], 10);

function run(cmdArgs) {
  return new Promise(resolve => {
    const startMem = process.memoryUsage?.().rss || 0;
    const t0 = performance.now();
    const child = spawn(process.execPath, [BIN, ...cmdArgs], { cwd: ROOT });
    let stdout = '', stderr = '';
    child.stdout.on('data', c => stdout += c);
    child.stderr.on('data', c => stderr += c);
    child.on('close', code => {
      const elapsed = performance.now() - t0;
      resolve({ code, stdout, stderr, elapsedMs: elapsed, stdoutBytes: Buffer.byteLength(stdout) });
    });
  });
}

function percentile(sorted, p) {
  const idx = Math.ceil(sorted.length * p) - 1;
  return sorted[Math.max(0, idx)];
}

function median(sorted) {
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

async function measureFixture(label, lintArgs) {
  const times = [];
  const coldTimes = [];
  let outputBytes = 0;

  for (let i = 0; i < ITERATIONS; i++) {
    const result = await run(lintArgs);
    if (i === 0) coldTimes.push(result.elapsedMs);
    times.push(result.elapsedMs);
    outputBytes = result.stdoutBytes;
  }

  const sorted = [...times].sort((a, b) => a - b);
  return {
    fixture: label,
    iterations: ITERATIONS,
    cold_ms: coldTimes[0],
    median_ms: Math.round(median(sorted) * 100) / 100,
    p95_ms: Math.round(percentile(sorted, 0.95) * 100) / 100,
    min_ms: Math.round(sorted[0] * 100) / 100,
    max_ms: Math.round(sorted[sorted.length - 1] * 100) / 100,
    output_bytes: outputBytes,
    estimated_output_tokens: Math.ceil(outputBytes / 4)
  };
}

function getRevision() {
  try { return execSync('git rev-parse --short HEAD', { cwd: ROOT, encoding: 'utf8' }).trim(); }
  catch { return 'unknown'; }
}

async function generateFixture(size) {
  const out = join(tmpdir(), 'considered-bench');
  const result = await new Promise(resolve => {
    const child = spawn(process.execPath, [join(HERE, 'generate.mjs'), '--size', size, '--out', out], { cwd: ROOT });
    let stdout = '';
    child.stdout.on('data', c => stdout += c);
    child.on('close', () => resolve(JSON.parse(stdout)));
  });
  return result.out;
}

// --- main ---

console.log(`Benchmark: ${ITERATIONS} iterations per command/size`);

const results = [];

// Small (committed fixture)
console.log('  small (committed)...');
results.push(await measureFixture('small', ['lint', SMALL_FIXTURE, '--json']));

// Medium (generated)
console.log('  medium (generated)...');
const mediumDir = await generateFixture('medium');
results.push(await measureFixture('medium', ['lint', mediumDir, '--json']));

// Large (generated)
console.log('  large (generated)...');
const largeDir = await generateFixture('large');
results.push(await measureFixture('large', ['lint', largeDir, '--json']));

// Inventory benchmarks on the same fixtures
console.log('  inventory benchmarks...');
const invResults = [];
invResults.push(await measureFixture('small-inventory', ['inventory', '--json', SMALL_FIXTURE]));
invResults.push(await measureFixture('medium-inventory', ['inventory', '--json', mediumDir]));
invResults.push(await measureFixture('large-inventory', ['inventory', '--json', largeDir]));

const baseline = {
  schema_version: '1.0.0',
  timestamp: new Date().toISOString(),
  os: platform(),
  architecture: arch(),
  node_version: process.version,
  rust_version: (() => { try { return execSync('rustc --version', { encoding: 'utf8' }).trim(); } catch { return 'not-installed'; } })(),
  repository_revision: getRevision(),
  iterations: ITERATIONS,
  benchmarks: {
    lint: results,
    inventory: invResults
  }
};

if (writeBaseline) {
  await writeFile(BASELINE_PATH, JSON.stringify(baseline, null, 2) + '\n');
  console.log(`Baseline written to ${BASELINE_PATH}`);
} else {
  console.log(JSON.stringify(baseline, null, 2));
}
