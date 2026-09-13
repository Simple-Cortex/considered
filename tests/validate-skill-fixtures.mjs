#!/usr/bin/env node
/** Exercise the public Rust validator against isolated release-tree mutations. */
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawn } from 'node:child_process';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const BIN = process.env.CONSIDERED_RS_BIN || join(ROOT, 'target', 'debug', 'considered-rs');
const cases = [
  ['valid-release-tree', null, 0, null],
  ['broken-routed-reference', async root => writeFile(join(root, 'skills/frame/SKILL.md'), 'See `references/missing.md`.\n'), 1, 'Referenced path does not exist'],
  ['missing-template', async root => writeFile(join(root, 'SKILL.md'), (await readFile(join(root, 'SKILL.md'), 'utf8')) + '\n`assets/templates/MISSING.md`\n'), 1, 'Referenced path does not exist'],
  ['command-drift', async root => writeFile(join(root, 'SKILL.md'), (await readFile(join(root, 'SKILL.md'), 'utf8')).replace('| `init` |', '| `absent` |')), 1, 'absent'],
  ['duplicate-command', async root => writeFile(join(root, 'SKILL.md'), (await readFile(join(root, 'SKILL.md'), 'utf8')).replace('| `init` |', '| `init` |\n| `init` |')), 1, 'Duplicate command'],
  ['version-drift', async root => writeFile(join(root, 'README.md'), (await readFile(join(root, 'README.md'), 'utf8')).replace('v0.7.0', 'v9.9.9')), 1, 'Version mismatch'],
  ['stale-documentation', async root => writeFile(join(root, 'skills/frame/SKILL.md'), (await readFile(join(root, 'skills/frame/SKILL.md'), 'utf8')) + '\nThis feature is not yet implemented.\n'), 1, 'stale'],
  ['manifest-drift', async root => writeFile(join(root, 'runtime-manifest.json'), JSON.stringify({ '$schema': 'bad', include: [] })), 1, 'runtime-manifest'],
  ['scaffold-placeholder', async root => writeFile(join(root, 'SKILL.md'), (await readFile(join(root, 'SKILL.md'), 'utf8')) + '\n<REPLACE-ME>\n'), 1, 'placeholder'],
];

function run(root) { return new Promise(resolve => { const child = spawn(BIN, ['validate-skill', '--json', '--root', root], { cwd: root }); let out=''; let err=''; child.stdout.on('data', d => out += d); child.stderr.on('data', d => err += d); child.on('close', code => resolve({ code, out, err })); }); }
for (const [name, mutate, expectedCode, expected] of cases) {
  const root = await mkdtemp(join(tmpdir(), 'considered-validator-'));
  await cp(ROOT, root, { recursive: true, filter: source => !source.includes('/.git') && !source.includes('/target') && !/[\\/]\.[^\\/]+$/.test(source) });
  if (mutate) await mutate(root);
  const result = await run(root);
  const text = result.out + result.err;
  const ok = result.code === expectedCode && (!expected || text.toLowerCase().includes(expected.toLowerCase()));
  console.log(`${ok ? 'pass' : 'FAIL'}  ${name}`);
  await rm(root, { recursive: true, force: true });
  if (!ok) process.exitCode = 1;
}
