#!/usr/bin/env node
/** Release checks that do not require a completed evaluation result. */
import { readFile, stat } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');
if (process.argv.slice(2).some(arg => arg === '--help' || arg === '-h')) {
  console.log('Usage: node prepack.mjs\n\nRelease checks that do not require a completed evaluation result: required\nfiles present, package.json fields, SKILL.md frontmatter, and that\nvalidate-assets.mjs and test.mjs both pass. Takes no arguments.');
  process.exit(0);
}
const required = [
  'SKILL.md', 'README.md', 'LICENSE', 'package.json',
  'scripts/validate-assets.mjs', 'scripts/test.mjs', 'scripts/gate.mjs', 'scripts/lint-render.mjs', 'scripts/install.mjs',
  'assets/templates/REVIEW-PACKET.md', 'assets/templates/REVIEW.json', 'evals/README.md'
];
for (const relative of required) {
  try { await stat(join(ROOT, relative)); }
  catch { console.error(`Missing release file: ${relative}`); process.exit(1); }
}
const pkg = JSON.parse(await readFile(join(ROOT, 'package.json'), 'utf8'));
for (const field of ['name', 'version', 'license', 'repository', 'homepage', 'bugs', 'engines']) if (!pkg[field]) {
  console.error(`package.json lacks ${field}.`);
  process.exit(1);
}
const skill = await readFile(join(ROOT, 'SKILL.md'), 'utf8');
if (!/^---\nname: "considered"\ndescription: ".+"\n---/s.test(skill)) {
  console.error('SKILL.md frontmatter must contain only portable name and description fields.');
  process.exit(1);
}
const run = script => new Promise(resolve => {
  const child = spawn(process.execPath, [join(HERE, script)], { cwd: ROOT, stdio: 'inherit' });
  child.on('error', () => resolve(2));
  child.on('close', code => resolve(code ?? 2));
});
for (const script of ['validate-assets.mjs', 'test.mjs']) {
  const code = await run(script);
  if (code !== 0) process.exit(code);
}
console.log('Prepack checks passed. Evaluation results are intentionally not required for packaging.');
