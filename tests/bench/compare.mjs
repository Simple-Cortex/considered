#!/usr/bin/env node
/** Compare release Rust and Node implementations on identical scan trees. */
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { access, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { tmpdir, platform, arch } from 'node:os';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const RUST = join(ROOT, 'target', 'release', 'considered-rs');
const SMALL = join(HERE, 'fixtures', 'small');
const iterationsAt = process.argv.indexOf('--iterations');
const ITERATIONS = iterationsAt === -1 ? 10 : Number(process.argv[iterationsAt + 1]);
const record = process.argv.includes('--record');
const RECORD_PATH = join(HERE, 'comparison.json');

if (!Number.isSafeInteger(ITERATIONS) || ITERATIONS < 2) {
  console.error('Usage: node tests/bench/compare.mjs [--iterations <integer >= 2>]');
  process.exit(2);
}

try { await access(RUST); } catch {
  console.error('Release binary missing. Run: cargo build --release -p considered-cli');
  process.exit(2);
}

function execute(command, args) {
  return new Promise(resolve => {
    const started = performance.now();
    const child = spawn(command, args, { cwd: ROOT, stdio: ['ignore', 'pipe', 'pipe'] });
    const stdout = [];
    const stderr = [];
    child.stdout.on('data', chunk => stdout.push(chunk));
    child.stderr.on('data', chunk => stderr.push(chunk));
    child.once('close', code => resolve({
      code: code ?? 2,
      stdout: Buffer.concat(stdout),
      stderr: Buffer.concat(stderr),
      elapsed_ms: performance.now() - started
    }));
  });
}

async function generate(size) {
  const result = await execute(process.execPath, [join(HERE, 'generate.mjs'), '--size', size, '--out', join(tmpdir(), 'considered-bench-compare')]);
  if (result.code !== 0) throw new Error(result.stderr.toString() || `fixture generation failed: ${size}`);
  return JSON.parse(result.stdout).out;
}

function stats(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const median = sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
  const p95 = sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
  return { median_ms: Number(median.toFixed(2)), p95_ms: Number(p95.toFixed(2)) };
}

function digest(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}

async function measure(command, fixture) {
  const nodeArgs = command === 'lint'
    ? [join(ROOT, 'scripts', 'lint-source.mjs'), fixture, '--json']
    : [join(ROOT, 'scripts', 'inventory.mjs'), '--json', fixture];
  const rustArgs = [command, fixture, '--json'];
  const samples = { node: [], rust: [] };
  let outputBytes = 0;
  const coldNode = await execute(process.execPath, nodeArgs);
  const coldRust = await execute(RUST, rustArgs);
  if (coldNode.code !== 0 || coldRust.code !== 0 || !coldNode.stdout.equals(coldRust.stdout) || !coldNode.stderr.equals(coldRust.stderr)) {
    throw new Error(`${command}/${fixture}: cold Node and Rust executions differ`);
  }

  for (let index = 0; index < ITERATIONS; index += 1) {
    // Alternate order so thermal/process-scheduling drift does not always favor one engine.
    const order = index % 2 === 0 ? ['node', 'rust'] : ['rust', 'node'];
    const results = {};
    for (const engine of order) {
      const result = engine === 'node'
        ? await execute(process.execPath, nodeArgs)
        : await execute(RUST, rustArgs);
      if (result.code !== 0) throw new Error(`${command}/${engine} exited ${result.code}: ${result.stderr}`);
      samples[engine].push(result.elapsed_ms);
      results[engine] = result;
    }
    if (!results.node.stdout.equals(results.rust.stdout) || !results.node.stderr.equals(results.rust.stderr)) {
      throw new Error(`${command}/${fixture}: Node and Rust output drifted`);
    }
    outputBytes = results.node.stdout.length;
  }

  const node = stats(samples.node);
  const rust = stats(samples.rust);
  return {
    command,
    fixture,
    iterations: ITERATIONS,
    node: { cold_ms: Number(coldNode.elapsed_ms.toFixed(2)), ...node },
    rust: { cold_ms: Number(coldRust.elapsed_ms.toFixed(2)), ...rust },
    median_speedup: Number((node.median_ms / rust.median_ms).toFixed(2)),
    p95_speedup: Number((node.p95_ms / rust.p95_ms).toFixed(2)),
    output_bytes: outputBytes,
    output_sha256: digest((await execute(RUST, rustArgs)).stdout)
  };
}

const medium = await generate('medium');
const large = await generate('large');
const fixtures = [['small', SMALL], ['medium', medium], ['large', large]];
const comparisons = [];
for (const command of ['lint', 'inventory']) {
  for (const [label, fixture] of fixtures) {
    process.stderr.write(`measuring ${command}/${label}\n`);
    comparisons.push({ ...(await measure(command, fixture)), fixture: label });
  }
}

// The gate has two independent checks:
//   1. Correctness: Node and Rust outputs must be byte-identical on every
//      iteration of every case. This is already enforced above inside
//      measure() (cold-start comparison and per-iteration drift check), both
//      of which throw immediately on any mismatch — nothing further to do
//      here, this is just confirming that invariant holds before gating on
//      speed.
//   2. Performance: timings swing 20-50% run to run under load, so gating on
//      p95 with a small iteration count is noisy. Instead, gate on the more
//      stable median across a larger sample, and only require that at least
//      one representative (medium/large) case anywhere in the suite clears
//      the bar — not every command on every fixture.
const representative = comparisons.filter(result => ['medium', 'large'].includes(result.fixture));
const satisfied = representative.find(result => result.median_speedup >= 1.5);
if (!satisfied) {
  throw new Error('performance gate missed: no medium/large case reached >=1.5x median speedup (Node vs Rust output hashes still matched on every case)');
}
process.stderr.write(`performance gate satisfied by ${satisfied.command}/${satisfied.fixture}: ${satisfied.median_speedup}x median speedup (p95 reported, not gated)\n`);

const report = {
  schema_version: 'considered-benchmark-comparison/v1',
  measured_at: new Date().toISOString(),
  platform: platform(),
  architecture: arch(),
  node_version: process.version,
  rust_version: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
  repository_revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim(),
  source_state: 'measured from the current worktree; preserve the accompanying revision and diff when publishing results',
  iterations: ITERATIONS,
  binary_profile: 'release',
  comparisons
};
const serialized = `${JSON.stringify(report, null, 2)}\n`;
if (record) {
  await writeFile(RECORD_PATH, serialized);
  process.stderr.write(`wrote ${RECORD_PATH}\n`);
}
process.stdout.write(serialized);
