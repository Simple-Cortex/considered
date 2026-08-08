#!/usr/bin/env node
/** Copy this portable skill package to an explicit harness skill directory. */
import { cp, lstat, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, relative, sep } from 'node:path';
import { resolveYesPlan } from './wizard-plan.mjs';

// The installed skill must match the published package surface exactly. This mirrors
// the "files" array in package.json; keep the two in step. Anything outside this set
// (repo tooling, CI config, editor and OS noise) never reaches a user's skill directory.
const SHIPPED = new Set([
  'SKILL.md', 'README.md', 'LICENSE',
  'skills', 'references', 'assets', 'scripts', 'bin', 'evals', 'docs', 'tests'
]);

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
const help = args.includes('--help') || args.includes('-h');
if (help) {
  console.log('Usage: node install.mjs --dest <skill-directory> [--force] [--dry-run]\n       node install.mjs --yes [--force] [--dry-run]\n\nCopies the complete portable skill package to an explicit destination.\nThe destination must be the final considered skill directory, not its parent.\n--yes proceeds only when a confident default destination is detected in the\ncurrent directory (a .claude/ directory or CLAUDE.md); --dest always wins.');
  process.exit(0);
}
const allowed = new Set(['--dest', '--force', '--dry-run', '--yes']);
for (const token of args.filter(token => token.startsWith('--'))) if (!allowed.has(token)) { console.error(`Unknown option: ${token}`); process.exit(2); }
const explicitDest = value('--dest');
if (explicitDest !== null && args.filter(token => token === '--dest').length !== 1) { console.error('install requires exactly one --dest <skill-directory>.'); process.exit(2); }
let dest = explicitDest;
if (!dest) {
  if (!args.includes('--yes')) { console.error('install requires --dest <skill-directory> (or --yes to accept a detected default).'); process.exit(2); }
  const plan = resolveYesPlan({ cwd: process.cwd() });
  if (!plan) { console.error('install --yes found no confident default in this directory (no .claude/ or CLAUDE.md). Pass --dest explicitly.'); process.exit(2); }
  dest = plan.dest;
}
const destTokens = explicitDest !== null ? 2 : 0;
const expectedArgCount = destTokens + Number(args.includes('--force')) + Number(args.includes('--dry-run')) + Number(args.includes('--yes'));
if (args.length !== expectedArgCount) { console.error('Malformed install options. Run with --help.'); process.exit(2); }
const target = resolve(dest);
if (target === ROOT) { console.error('Destination is already this skill directory.'); process.exit(2); }
let present = false;
try { await lstat(target); present = true; } catch (error) { if (error.code !== 'ENOENT') throw error; }
if (present && !args.includes('--force')) { console.error(`${target} exists. Use --force only when replacing this skill intentionally.`); process.exit(2); }
if (args.includes('--dry-run')) { console.log(`${present ? 'Would replace' : 'Would install'} Considered at ${target}`); process.exit(0); }
if (present) await rm(target, { recursive: true, force: true });
await cp(ROOT, target, {
  recursive: true,
  filter: source => {
    const rel = relative(ROOT, source);
    if (rel === '') return true;
    const segments = rel.split(sep);
    if (!SHIPPED.has(segments[0])) return false;
    if (segments.includes('node_modules') || segments.includes('.DS_Store')) return false;
    // Recorded evaluation runs are per-user output, not part of the skill.
    return !(segments[0] === 'evals' && segments[1] === 'results');
  }
});
console.log(`Installed Considered at ${target}`);
