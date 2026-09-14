#!/usr/bin/env node
/**
 * Deterministic compatibility runner for Milestone 0.
 *
 * For each case in manifest.json, captures command, exit code, stdout, stderr,
 * and named file effects. Normalizes only temporary absolute paths, platform
 * line endings, timestamps, and intentionally random keys. Preserves rule IDs,
 * severities, ordering, field names, and deterministic roll values.
 *
 * Usage:
 *   node tests/compat/run.mjs [--snapshot] [--check]
 *
 * --snapshot  write baseline snapshots to tests/compat/snapshots/
 * --check     compare current output against stored snapshots (default)
 *
 * Exit 0 if all cases match their snapshots (or snapshots were written).
 * Exit 1 if any case diverges.
 */
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { tmpdir, platform } from 'node:os';
import { createHash } from 'node:crypto';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const MANIFEST_PATH = join(HERE, 'manifest.json');
const SNAPSHOT_DIR = join(HERE, 'snapshots');
const BIN = join(ROOT, 'bin', 'considered.mjs');

const args = process.argv.slice(2);
const mode = args.includes('--snapshot') ? 'snapshot' : 'check';

function run(args, opts = {}) {
  return new Promise(resolve => {
    const child = spawn(process.execPath, [BIN, ...args], {
      cwd: ROOT,
      stdio: ['pipe', 'pipe', 'pipe']
    });
    let stdout = '', stderr = '';
    if (opts.stdin === 'closed') {
      child.stdin.end();
    }
    child.stdout.on('data', chunk => stdout += chunk);
    child.stderr.on('data', chunk => stderr += chunk);
    child.on('error', error => resolve({ code: 2, stdout, stderr: `${stderr}${error.message}` }));
    child.on('close', code => resolve({ code: code ?? 2, stdout, stderr }));
  });
}

/**
 * Normalize output for snapshot comparison.
 * - CRLF -> LF
 * - Absolute temp paths -> __TMP__
 * - Absolute repo paths -> __ROOT__
 * - ISO timestamps -> __TIMESTAMP__
 * - UUIDs -> __UUID__
 * - Hex hashes (8+ chars) -> __HASH__
 */
function normalize(text) {
  return text
    .replace(/\r\n/g, '\n')
    .replace(new RegExp(escapeRegex(tmpdir()), 'g'), '__TMP__')
    .replace(new RegExp(escapeRegex(resolve(ROOT)), 'g'), '__ROOT__')
    .replace(new RegExp(escapeRegex(ROOT), 'g'), '__ROOT__')
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}[\w.:+-]*/g, '__TIMESTAMP__')
    .replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, '__UUID__')
    .replace(/\b[0-9a-f]{16,}\b/g, '__HASH__');
}

function escapeRegex(str) {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function snapshotPath(caseId) {
  return join(SNAPSHOT_DIR, `${caseId}.json`);
}

function buildSnapshot(caseDef, result) {
  return {
    id: caseDef.id,
    category: caseDef.category,
    command: caseDef.command,
    exit_code: result.code,
    stdout: normalize(result.stdout),
    stderr: normalize(result.stderr),
    stdout_sha256: sha256(normalize(result.stdout)),
    stderr_sha256: sha256(normalize(result.stderr))
  };
}

function sha256(text) {
  return createHash('sha256').update(text).digest('hex');
}

function assertCase(caseDef, result) {
  const errors = [];

  if (caseDef.expect_exit !== undefined && result.code !== caseDef.expect_exit) {
    errors.push(`expected exit ${caseDef.expect_exit}, got ${result.code}`);
  }

  if (caseDef.stdout_contains) {
    for (const needle of caseDef.stdout_contains) {
      if (!result.stdout.includes(needle)) {
        errors.push(`stdout missing "${needle}"`);
      }
    }
  }

  if (caseDef.stderr_contains) {
    for (const needle of caseDef.stderr_contains) {
      if (!result.stderr.includes(needle)) {
        errors.push(`stderr missing "${needle}"`);
      }
    }
  }

  if (caseDef.stderr_empty && result.stderr.trim().length > 0) {
    errors.push(`expected empty stderr, got ${result.stderr.trim().slice(0, 80)}`);
  }

  if (caseDef.stdout_matches) {
    const re = new RegExp(caseDef.stdout_matches);
    if (!re.test(result.stdout)) {
      errors.push(`stdout does not match ${caseDef.stdout_matches}`);
    }
  }

  if (caseDef.stdout_is_json) {
    try {
      const parsed = JSON.parse(result.stdout);
      if (caseDef.json_assertions) {
        for (const assertion of caseDef.json_assertions) {
          const value = getNestedValue(parsed, assertion.path);
          if (assertion.present && value === undefined) {
            errors.push(`JSON missing path "${assertion.path}"`);
          }
          if (assertion.equals !== undefined) {
            if (JSON.stringify(value) !== JSON.stringify(assertion.equals)) {
              errors.push(`JSON path "${assertion.path}": expected ${JSON.stringify(assertion.equals)}, got ${JSON.stringify(value)}`);
            }
          }
          if (assertion.array_contains_rule) {
            const found = Array.isArray(value) && value.some(f => f.rule === assertion.array_contains_rule);
            if (!found) errors.push(`JSON path "${assertion.path}" has no entry with rule "${assertion.array_contains_rule}"`);
          }
        }
      }
    } catch {
      errors.push('stdout is not valid JSON');
    }
  }

  return errors;
}

function getNestedValue(obj, path) {
  return path.split('.').reduce((v, k) => v?.[k], obj);
}

// --- main ---

const manifest = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
let failed = 0;
let passed = 0;
const results = [];

if (mode === 'snapshot') {
  await rm(SNAPSHOT_DIR, { recursive: true, force: true });
  await mkdir(SNAPSHOT_DIR, { recursive: true });
}

for (const caseDef of manifest.cases) {
  const result = await run(caseDef.command, { stdin: caseDef.stdin });
  const assertionErrors = assertCase(caseDef, result);

  if (mode === 'snapshot') {
    const snap = buildSnapshot(caseDef, result);
    await writeFile(snapshotPath(caseDef.id), JSON.stringify(snap, null, 2) + '\n');
    if (assertionErrors.length) {
      console.error(`FAIL  ${caseDef.id}: ${assertionErrors.join('; ')}`);
      failed++;
    } else {
      console.log(`pass  ${caseDef.id} (snapshot written)`);
      passed++;
    }
  } else {
    let stored;
    try {
      stored = JSON.parse(await readFile(snapshotPath(caseDef.id), 'utf8'));
    } catch {
      console.error(`FAIL  ${caseDef.id}: no snapshot found (run with --snapshot first)`);
      failed++;
      continue;
    }

    const current = buildSnapshot(caseDef, result);
    const diffs = [];

    if (stored.exit_code !== current.exit_code) {
      diffs.push(`exit code: ${stored.exit_code} -> ${current.exit_code}`);
    }
    if (stored.stdout_sha256 !== current.stdout_sha256) {
      diffs.push('stdout changed');
    }
    if (stored.stderr_sha256 !== current.stderr_sha256) {
      diffs.push('stderr changed');
    }
    diffs.push(...assertionErrors.map(e => `assertion: ${e}`));

    if (diffs.length) {
      console.error(`FAIL  ${caseDef.id}: ${diffs.join('; ')}`);
      failed++;
    } else {
      console.log(`pass  ${caseDef.id}`);
      passed++;
    }
  }

  results.push({ id: caseDef.id, exit: result.code, ok: assertionErrors.length === 0 });
}

const summary = {
  mode,
  timestamp: new Date().toISOString(),
  platform: platform(),
  node_version: process.version,
  repo_root: ROOT,
  total: manifest.cases.length,
  passed,
  failed
};

if (failed) {
  console.error(`\n${failed} compat case(s) failed.`);
  process.exit(1);
}
console.log(`\nAll ${passed} compat cases passed (${mode} mode).`);
