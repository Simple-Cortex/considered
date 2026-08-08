#!/usr/bin/env node
/**
 * considered - CLI entry point.
 * Thin dispatcher. All utility logic lives in scripts/.
 */
import { spawn } from 'node:child_process';
import { readFile, lstat } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve as resolvePath } from 'node:path';
import { homedir } from 'node:os';
import { createInterface } from 'node:readline/promises';
import {
  planChoices, resolveCustomDest, expandHome, interpretUpdateAnswer, interpretConfirmAnswer, HOST_CAUTION
} from '../scripts/wizard-plan.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..');

const UTILITIES = {
  roll: 'roll.mjs',
  inventory: 'inventory.mjs',
  lint: 'lint-source.mjs',
  contract: 'lint-contract.mjs',
  render: 'lint-render.mjs',
  gate: 'gate.mjs',
  validate: 'validate-assets.mjs',
  test: 'test.mjs',
  eval: 'eval.mjs',
  check: 'check.mjs',
  install: 'install.mjs'
};

function usage(stream = process.stdout) {
  stream.write(`considered - design reasoning for coding agents

Usage:
  considered                   interactive install wizard (TTY only)
  considered [--json] <utility> [arguments]
  considered --help
  considered --version

Agent skill phases (not CLI utilities):
  init, frame, structure, compose, critique, subtract, harden, document

CLI utilities:
  roll [options]               assign a structure and direction
  inventory [path] [--json]   report an existing design system
  lint <path>                  run static source checks
  contract <file>              validate a contract block
  render <evidence.json>       validate attested screenshot evidence
  gate <report...> --review <file>
                               combine high-confidence checks with independent review
  validate                      validate packaged manifests and decks
  test                          run deterministic fixture tests
  eval [options]                initialize one matched evaluation condition
  check <result-dir>            validate a completed evaluation record
  install --dest <skill-dir>    copy this skill to an explicit destination
                                 [--force]
  install --yes [--force]       copy to a detected default destination (CI)

Global options:
  --json       request machine-readable output where supported
  -h, --help   show this help
  -v, --version
               show the package version

Modes: persuade, operate, analyze, read, experience

Exit codes: 0 clean, 1 blocking findings, 2 usage error, 3 deck exhausted.
Docs: https://github.com/considered-design/considered
`);
}

function fail(message) {
  console.error(`considered: ${message}`);
  console.error('Run "considered --help" for usage.');
  process.exit(2);
}

function installUsage() {
  process.stderr.write(`considered: no arguments, and no interactive terminal detected.

Usage:
  considered install --dest <skill-directory> [--force]
  considered install --yes [--force]   (accepts a detected default; CI-safe)

Run "considered --help" for the full command list.
`);
}

async function pathExists(target) {
  try { await lstat(target); return true; } catch (error) { if (error.code === 'ENOENT') return false; throw error; }
}

async function readVersion() {
  const pkg = JSON.parse(await readFile(join(ROOT, 'package.json'), 'utf8'));
  return pkg.version;
}

function runInstall(dest, force) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(process.execPath, [join(ROOT, 'scripts', 'install.mjs'), '--dest', dest, ...(force ? ['--force'] : [])], { stdio: 'inherit' });
    child.once('error', rejectRun);
    child.once('exit', (code, signal) => signal ? rejectRun(new Error(`install terminated by signal ${signal}`)) : resolveRun(code ?? 1));
  });
}

async function runWizard() {
  const rl = createInterface({ input: process.stdin, output: process.stdout });
  try {
    const cwd = process.cwd();
    const home = homedir();
    const { choices, defaultChoice, projectDetected, globalDetected } = planChoices({ cwd, homedir: home });

    console.log('considered install wizard\n');
    for (const choice of choices) {
      const note = choice.key === '1' && projectDetected ? ' (detected)'
        : choice.key === '2' && globalDetected ? ' (detected)' : '';
      const destLabel = choice.dest ? ` — ${choice.dest}` : '';
      console.log(`  [${choice.key}] ${choice.label}${destLabel}${note}`);
    }
    console.log('');

    const promptSuffix = defaultChoice ? ` [${defaultChoice}]` : '';
    const MAX_MENU_ATTEMPTS = 3; // initial attempt + 2 retries
    let picked = null;
    for (let attempt = 0; attempt < MAX_MENU_ATTEMPTS && !picked; attempt++) {
      let answer = (await rl.question(`Select a destination${promptSuffix}: `)).trim();
      if (!answer && defaultChoice) answer = defaultChoice;
      picked = choices.find(choice => choice.key === answer) ?? null;
      if (!picked && attempt < MAX_MENU_ATTEMPTS - 1) {
        console.error(`considered: "${answer}" is not a valid choice. Enter ${choices.map(c => c.key).join(', ')}.`);
      }
    }
    if (!picked) {
      console.error('considered: no valid choice after 3 attempts.');
      return 2;
    }

    let dest;
    if (picked.key === '3') {
      console.log(`\n${HOST_CAUTION}`);
      console.log('If your host has no skill discovery, see the manual fallback in references/workflow.md.\n');
      const skillsDir = (await rl.question("Enter the host's documented skills directory: ")).trim();
      if (!skillsDir) {
        console.error('considered: no directory entered. Aborting.');
        return 2;
      }
      dest = resolveCustomDest(resolvePath(expandHome(skillsDir, home)));
    } else {
      dest = picked.dest;
    }

    let force = false;
    if (await pathExists(dest)) {
      console.log(`\n${dest}`);
      const update = await rl.question('already exists. Replace the existing install? [y/N]: ');
      if (!interpretUpdateAnswer(update)) {
        console.log('Aborted. Nothing was written.');
        return 0;
      }
      force = true;
    } else {
      console.log(`\n${dest}`);
      const confirm = await rl.question('Install considered here? [Y/n]: ');
      if (!interpretConfirmAnswer(confirm)) {
        console.log('Aborted. Nothing was written.');
        return 0;
      }
    }

    rl.close();
    const version = await readVersion();
    const code = await runInstall(dest, force);
    if (code !== 0) return code;

    console.log(`\nInstalled considered@${version} at ${dest}`);
    console.log('\nVerify:');
    console.log('  1. Restart or refresh your agent host so it re-scans skill directories.');
    console.log('  2. Ask the agent: "use the considered skill to frame <your surface>".');
    console.log('  3. Check the host\'s skill listing for "considered".');
    return 0;
  } finally {
    rl.close();
  }
}

const input = process.argv.slice(2);
if (input.length === 0) {
  if (process.stdin.isTTY && process.stdout.isTTY) {
    process.exit(await runWizard());
  }
  installUsage();
  process.exit(2);
}

if (input[0] === '--help' || input[0] === '-h') {
  if (input.length !== 1) fail(`${input[0]} does not accept arguments`);
  usage();
  process.exit(0);
}

if (input[0] === '--version' || input[0] === '-v') {
  if (input.length !== 1) fail(`${input[0]} does not accept arguments`);
  let version;
  try {
    const pkg = JSON.parse(await readFile(join(ROOT, 'package.json'), 'utf8'));
    if (typeof pkg.version !== 'string' || pkg.version.length === 0) {
      throw new Error('package.json has no version');
    }
    version = pkg.version;
  } catch (error) {
    console.error(`considered: cannot read package version: ${error.message}`);
    process.exit(2);
  }
  console.log(version);
  process.exit(0);
}

let globalJson = false;
if (input[0] === '--json') {
  globalJson = true;
  input.shift();
}

if (input.length === 0) fail('missing utility after --json');
const [utility, ...args] = input;
if (utility.startsWith('-')) fail(`unknown global option "${utility}"`);
if (!Object.hasOwn(UTILITIES, utility)) fail(`unknown utility "${utility}"`);

const childArgs = [join(ROOT, 'scripts', UTILITIES[utility])];
if (globalJson) childArgs.push('--json');
childArgs.push(...args);

let settled = false;
let child;
try {
  child = spawn(process.execPath, childArgs, { stdio: 'inherit' });
} catch (error) {
  console.error(`considered: failed to start ${utility}: ${error.message}`);
  process.exit(2);
}

child.once('error', error => {
  if (settled) return;
  settled = true;
  console.error(`considered: failed to start ${utility}: ${error.message}`);
  process.exit(2);
});

child.once('exit', (code, signal) => {
  if (settled) return;
  settled = true;
  if (signal) {
    console.error(`considered: ${utility} terminated by signal ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
