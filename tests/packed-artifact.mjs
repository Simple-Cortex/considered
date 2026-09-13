#!/usr/bin/env node
/** Verify the publishable tarball, never the source tree, installs and runs. */
import { mkdtemp, readdir, rm, stat } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';

const exec = promisify(execFile);
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const temp = await mkdtemp(join(tmpdir(), 'considered-pack-'));
try {
  const { stdout } = await exec('npm', ['pack', ROOT, '--json', '--ignore-scripts'], { cwd: temp, env: { ...process.env, npm_config_cache: join(temp, 'cache') } });
  const pack = JSON.parse(stdout)[0];
  const paths = pack.files.map(file => file.path).sort();
  for (const forbidden of ['docs/', 'tests/', 'evals/', 'crates/']) {
    if (paths.some(path => path.startsWith(forbidden))) throw new Error(`tarball contains excluded ${forbidden}`);
  }
  const tarball = join(temp, pack.filename);
  await exec('tar', ['-xzf', tarball], { cwd: temp });
  const pkg = join(temp, 'package');
  for (const required of ['SKILL.md', 'bin/considered.mjs', 'runtime-manifest.json', 'scripts/install.mjs']) {
    await stat(join(pkg, required));
  }
  const destination = join(temp, 'installed');
  await exec(process.execPath, ['scripts/install.mjs', '--dest', destination], { cwd: pkg });
  const version = await exec(process.execPath, ['bin/considered.mjs', '--version'], { cwd: destination });
  if (version.stdout.trim() !== pack.version) throw new Error('installed Node fallback version mismatch');
  const installed = await readdir(destination);
  if (installed.some(name => ['docs', 'tests', 'evals', 'crates'].includes(name))) throw new Error('installer copied excluded maintainer content');
  console.log(`packed artifact smoke test passed (${paths.length} files)`);
} finally {
  await rm(temp, { recursive: true, force: true });
}
