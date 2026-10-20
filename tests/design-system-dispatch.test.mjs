import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const dispatcher = join(root, 'bin/considered.mjs');
const binary = join(root, 'target/debug/considered-rs');

test('design-system routes native help and reports a missing companion', () => {
  const help = spawnSync(process.execPath, [dispatcher, 'design-system', '--help'], {
    env: { ...process.env, CONSIDERED_RS_BIN: binary }, encoding: 'utf8',
  });
  assert.equal(help.status, 0, help.stderr);
  for (const command of ['import', 'query', 'freshness', 'snapshot']) {
    assert.match(help.stdout, new RegExp(`\\b${command}\\b`));
  }
  const unavailable = spawnSync(process.execPath, [dispatcher, '--format', 'engine-json', 'design-system', '--help'], {
    env: { ...process.env, CONSIDERED_RS_BIN: join(root, 'missing-design-system-binary') }, encoding: 'utf8',
  });
  assert.equal(unavailable.status, 2);
  const result = JSON.parse(unavailable.stdout);
  assert.equal(result.engine, 'manual-unavailable');
  assert.equal(result.error.code, 'native_unavailable');
});

test('public query preserves native JSON and its byte budget', async () => {
  const workspace = await mkdtemp(join(tmpdir(), 'considered-ds-dispatch-'));
  try {
    const index = join(workspace, 'index.json');
    await writeFile(index, JSON.stringify({
      schema_version: 1,
      identity: { name: 'example-ui', resolved_version: '1.0.0', source: 'fixture' },
      indexed_at: '', source_fingerprints: [],
      coverage: { status: 'partial', scope: 'fixture', evidence: [] },
      components: [{ name: 'Button', purpose: 'Trigger an action', detail_path: 'components/Button.json', coverage_status: 'unreviewed', freshness: { status: 'unknown', checked_at: '', source_fingerprints: [] } }],
    }));
    const args = ['design-system', 'query', index, '--query', 'Button', '--max-bytes', '2048'];
    const direct = spawnSync(binary, args, { encoding: 'utf8' });
    assert.equal(direct.status, 0, direct.stderr);
    const dispatched = spawnSync(process.execPath, [dispatcher, '--json', ...args], {
      env: { ...process.env, CONSIDERED_RS_BIN: binary }, encoding: 'utf8',
    });
    assert.equal(dispatched.status, 0, dispatched.stderr);
    assert.deepEqual(JSON.parse(dispatched.stdout), JSON.parse(direct.stdout));
    assert.ok(Buffer.byteLength(dispatched.stdout) <= 2048);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});
