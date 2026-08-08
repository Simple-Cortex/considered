#!/usr/bin/env node
/**
 * inventory.mjs - report the design system that already exists.
 *
 * Run before designing anything. New work should inherit tokens, components,
 * and conventions rather than overwrite them. A skill that invents a second
 * design system inside a codebase that already has one is a liability.
 *
 * Output is deliberately compact because it goes into a context window.
 */

import { readFile, readdir, stat } from 'node:fs/promises';
import { join, extname, basename, relative, dirname } from 'node:path';

const SKIP = new Set(['node_modules', 'dist', 'build', '.next', '.git', 'coverage', 'out', '.turbo', 'vendor']);
const CODE = new Set(['.css', '.scss', '.sass', '.less', '.html', '.jsx', '.tsx', '.js', '.mjs', '.cjs', '.ts', '.mts', '.cts', '.vue', '.svelte', '.astro', '.mdx']);
const CONFIGS = ['tailwind.config.js', 'tailwind.config.ts', 'tailwind.config.mjs', 'tailwind.config.cjs',
  'theme.js', 'theme.ts', 'panda.config.ts', 'uno.config.ts'];
const CONTEXT_DOCS = ['DESIGN.md', 'PRODUCT.md', 'STRUCTURE.md', 'design.md', 'product.md'];

function usage() {
  console.log(`considered inventory - reports an existing design system

Usage:
  node inventory.mjs [path] [--json]
  node inventory.mjs [--json] [path]

Arguments:
  path      source directory or individual source file (default: .)

Options:
  --json    machine-readable output
  -h, --help
            show this help`);
}

function usageError(message) {
  console.error(`considered inventory: ${message}`);
  console.error('Run "considered inventory --help" for usage.');
  process.exit(2);
}

function parseArgs(argv) {
  let target = null;
  let json = false;
  let help = false;

  for (const token of argv) {
    if (token === '--json') {
      if (json) usageError('--json may only be used once');
      json = true;
    } else if (token === '--help' || token === '-h') {
      if (help) usageError('help may only be used once');
      help = true;
    } else if (token.startsWith('-')) {
      usageError(`unknown option "${token}"`);
    } else if (target !== null) {
      usageError(`unexpected second target "${token}"`);
    } else {
      target = token;
    }
  }

  if (help && argv.length !== 1) usageError('help cannot be combined with a target or other options');
  return { target: target || '.', json, help };
}

const options = parseArgs(process.argv.slice(2));
if (options.help) {
  usage();
  process.exit(0);
}

const warnings = new Set();
const warn = (kind, path, error) => {
  const detail = error?.message ? `: ${error.message}` : '';
  const message = `Cannot read ${kind} ${path}${detail}`;
  if (warnings.has(message)) return;
  warnings.add(message);
  console.error(`warning: ${message}`);
};

async function walk(dir, acc = []) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch (error) {
    warn('directory', dir, error);
    return acc;
  }
  for (const entry of entries) {
    if (entry.name.startsWith('.') && entry.name !== '.storybook') continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!SKIP.has(entry.name)) await walk(full, acc);
    } else {
      acc.push(full);
    }
  }
  return acc;
}

async function exists(path) {
  try {
    await stat(path);
    return true;
  } catch (error) {
    if (error.code !== 'ENOENT') warn('path', path, error);
    return false;
  }
}

const textCache = new Map();
function readText(path) {
  if (!textCache.has(path)) {
    textCache.set(path, readFile(path, 'utf8').catch(error => {
      warn('file', path, error);
      return null;
    }));
  }
  return textCache.get(path);
}

const target = options.target;
let targetStat;
try {
  targetStat = await stat(target);
} catch (error) {
  console.error(`Cannot inspect ${target}: ${error.message}`);
  process.exit(2);
}

let all;
let scanRoot;
let targetKind;
if (targetStat.isDirectory()) {
  targetKind = 'directory';
  scanRoot = target;
  all = await walk(target);
} else if (targetStat.isFile()) {
  targetKind = 'file';
  scanRoot = dirname(target);
  all = [target];
} else {
  console.error(`Cannot inventory ${target}: expected a directory or regular file.`);
  process.exit(2);
}

const code = all.filter(file => CODE.has(extname(file)));

/* ---------- tokens ---------- */

const tokens = new Map();      // name -> value
const fonts = new Set();
const spacing = new Set();
const radii = new Set();

for (const file of code) {
  const text = await readText(file);
  if (text === null) continue;
  for (const match of text.matchAll(/(--[a-zA-Z0-9-]+)\s*:\s*([^;{}]+)/g)) {
    const name = match[1], value = match[2].trim();
    if (!tokens.has(name)) tokens.set(name, value);
  }
  for (const match of text.matchAll(/font-family\s*:\s*([^;{}]+)/gi)) fonts.add(match[1].trim().replace(/["']/g, ''));
  for (const match of text.matchAll(/(?:margin|padding|gap)(?:-[a-z]+)?\s*:\s*([^;{}]+)/gi)) {
    for (const value of match[1].matchAll(/(\d+(?:\.\d+)?(?:px|rem))/g)) spacing.add(value[1]);
  }
  for (const match of text.matchAll(/border-radius\s*:\s*([^;{}]+)/gi)) radii.add(match[1].trim());
}

const group = prefixes => {
  const out = {};
  for (const [name, value] of tokens) {
    const hit = prefixes.find(prefix => name.includes(prefix));
    if (hit) (out[hit] ||= []).push(`${name}: ${value}`);
  }
  return out;
};
const colorish = group(['color', 'bg', 'surface', 'text', 'border', 'accent', 'primary', 'danger', 'success', 'warning']);

/* ---------- components ---------- */

const componentDirs = new Set();
const components = new Map();  // name -> variants
for (const file of code) {
  const rel = targetKind === 'file'
    ? join(basename(dirname(file)), basename(file))
    : relative(scanRoot, file);
  if (!/components?|ui\/|design-system|primitives/i.test(rel)) continue;
  componentDirs.add(dirname(rel) === '.' ? '.' : dirname(rel).split('\\').join('/'));
  const name = basename(file, extname(file));
  if (/^(index|types|utils|constants)$/i.test(name)) continue;
  const text = await readText(file);
  if (text === null) continue;
  const variants = new Set();
  for (const match of text.matchAll(/(?:variant|appearance|kind|intent|tone)\s*[:?]?\s*["']([a-zA-Z-]+)["']/g)) variants.add(match[1]);
  for (const match of text.matchAll(/["'](primary|secondary|tertiary|ghost|subtle|outline|destructive|danger|link)["']/g)) variants.add(match[1]);
  components.set(name, [...variants]);
}

/* ---------- context docs ---------- */

const docs = [];
const configs = [];
if (targetKind === 'directory') {
  for (const doc of CONTEXT_DOCS) if (await exists(join(target, doc))) docs.push(doc);
  for (const config of CONFIGS) if (await exists(join(target, config))) configs.push(config);
} else {
  const name = basename(target);
  if (CONTEXT_DOCS.includes(name)) docs.push(name);
  if (CONFIGS.includes(name)) configs.push(name);
}

/* ---------- report ---------- */

const cap = (arr, n) => arr.length > n ? arr.slice(0, n).concat(`... +${arr.length - n} more`) : arr;

if (options.json) {
  console.log(JSON.stringify({
    target,
    targetKind,
    files: code.length,
    tokens: Object.fromEntries(tokens),
    fonts: [...fonts],
    spacing: [...spacing],
    radii: [...radii],
    components: Object.fromEntries(components),
    configs,
    docs,
    warnings: [...warnings]
  }, null, 2));
  process.exit(0);
}

const hasSystem = tokens.size > 0 || components.size > 0 || configs.length > 0;
const scope = targetKind === 'file' ? `in ${target}` : `under ${target}`;
console.log(`\n  EXISTING SYSTEM  ${code.length} source ${code.length === 1 ? 'file' : 'files'} ${scope}\n`);

if (!hasSystem) {
  console.log('  No design system detected. You are defining one.');
  console.log('  Establish tokens before components, and components before screens.\n');
} else {
  if (configs.length) console.log(`  Config      ${configs.join(', ')}`);
  if (docs.length)    console.log(`  Context     ${docs.join(', ')}  (read these before designing)`);

  if (tokens.size) {
    console.log(`\n  Tokens      ${tokens.size} custom properties`);
    for (const [key, list] of Object.entries(colorish)) {
      console.log(`    ${key.padEnd(9)} ${cap(list, 4).join('  ')}`);
    }
  }
  if (fonts.size)   console.log(`\n  Fonts       ${cap([...fonts], 4).join(' | ')}`);
  if (spacing.size) console.log(`  Spacing     ${cap([...spacing].sort((a, b) => parseFloat(a) - parseFloat(b)), 12).join(' ')}`);
  if (radii.size)   console.log(`  Radii       ${cap([...radii], 6).join(' | ')}`);

  if (components.size) {
    console.log(`\n  Components  ${components.size} in ${[...componentDirs].slice(0, 3).join(', ')}`);
    for (const [name, variants] of [...components].slice(0, 20)) {
      console.log(`    ${name.padEnd(18)} ${variants.length ? variants.join(', ') : '(no variants detected)'}`);
    }
    if (components.size > 20) console.log(`    ... +${components.size - 20} more`);
  }

  console.log('\n  Inherit this system. Do not invent a parallel one.');
  console.log('  If the assigned direction conflicts with these tokens, the tokens win');
  console.log('  unless the user explicitly asked for a redesign.\n');
}
