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
  'validate-skill': 'validate-skill.mjs',
  status: null,
  verify: null,
  context: null,
  test: 'test.mjs',
  eval: 'eval.mjs',
  check: 'check.mjs',
  install: 'install.mjs'
};

// The values are intentionally closed. They are part of the versioned
// engine-envelope schema, not free-form diagnostic prose.
const ENGINE = Object.freeze({
  NATIVE_RUST: 'native-rust',
  NODE_FALLBACK: 'node-fallback',
  MANUAL_UNAVAILABLE: 'manual-unavailable'
});

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
  validate skill                validate runtime skill drift (native companion)
  status [surface]              report current surface state (native companion)
  verify [surface]              verify artifact integrity (native companion)
  context --experimental        emit the bounded route manifest (native companion)
  test                          run deterministic fixture tests
  eval [options]                initialize one matched evaluation condition
  check <result-dir>            validate a completed evaluation record
  install --dest <skill-dir>    copy this skill to an explicit destination
                                 [--force]
  install --yes [--force]       copy to a detected default destination (CI)

Global options:
  --json       request machine-readable output where supported
  --format engine-json  versioned engine/result envelope; preserves --json compatibility
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

let envelope = false;
if (input[0] === '--format' && input[1] === 'engine-json') {
  envelope = true;
  input.splice(0, 2);
}

let globalJson = false;
if (input[0] === '--json') {
  globalJson = true;
  input.shift();
}

if (input.length === 0) fail('missing utility after --json');
const [utility, ...args] = input;
if (utility.startsWith('-')) fail(`unknown global option "${utility}"`);
if (utility === 'validate' && args[0] === 'skill') {
  input.splice(0, input.length, 'validate-skill', ...args.slice(1));
}
const [selectedUtility, ...selectedArgs] = input;
if (!Object.hasOwn(UTILITIES, selectedUtility)) fail(`unknown utility "${selectedUtility}"`);

let capabilityMatrix;
try {
  capabilityMatrix = JSON.parse(await readFile(join(ROOT, 'assets', 'templates', 'engine-capabilities.json'), 'utf8'));
} catch (error) {
  fail(`cannot read engine capability matrix: ${error.message}`);
}
const capability = capabilityMatrix.commands?.[selectedUtility];
if (!capability) fail(`engine capability matrix has no entry for "${selectedUtility}"`);

// Native selection is deliberately narrower than a command name.  The legacy
// Node diagnostics are frozen too, and clap's usage text is not byte-for-byte
// compatible for malformed argument shapes.  Unsupported shapes therefore
// remain on the authoritative Node path instead of receiving a near-match.
//
// --help/-h rides this same mechanism rather than a special case: neither
// this function nor any per-utility branch below ever recognizes "--help" or
// "-h" as a matched flag, so every one of them falls through to `return
// false` and --help is refused native delegation. For a utility that has a
// Node script (UTILITIES[selectedUtility] is non-null — lint, contract,
// gate, render, roll, inventory, validate, test, eval, check), that routes
// --help to the script's own usage text below (each now handles --help/-h
// directly) instead of to considered-rs's clap-generated help, which is
// worded differently and not part of the frozen Node contract. A utility
// with no Node script (status, verify, context) has no alternate text to
// diverge from, so its --help reaches the native binary's clap help either
// way once native is otherwise available.
function canDelegateNative() {
  if (selectedUtility === 'contract') {
    let fileCount = 0;
    const values = new Set();
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json' || arg === '--gate') continue;
      if (['--format', '--severity', '--path', '--max-findings', '--root'].includes(arg) && !values.has(arg) && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        const value = selectedArgs[index + 1];
        if (arg === '--format' && !['compatibility', 'compact', 'json', 'ndjson'].includes(value)) return false;
        if (arg === '--severity' && !/^(S[1-4])(,S[1-4])*$/.test(value)) return false;
        if (arg === '--max-findings' && !/^\d+$/.test(value)) return false;
        values.add(arg);
        index += 1;
        continue;
      }
      if (!arg.startsWith('-')) { fileCount += 1; continue; }
      return false;
    }
    return fileCount === 1;
  }
  if (selectedUtility === 'lint') {
    let paths = 0;
    const values = new Set();
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json' || arg === '--enforce-heuristics') continue;
      if (['--format', '--severity', '--path-filter', '--max-findings'].includes(arg) && !values.has(arg) && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        const value = selectedArgs[index + 1];
        if (arg === '--format' && !['compatibility', 'compact', 'json', 'ndjson'].includes(value)) return false;
        if (arg === '--severity' && !/^(S[1-4])(,S[1-4])*$/.test(value)) return false;
        if (arg === '--max-findings' && !/^\d+$/.test(value)) return false;
        values.add(arg);
        index += 1;
        continue;
      }
      if (!arg.startsWith('-')) { paths += 1; continue; }
      return false;
    }
    return paths === 1;
  }
  if (selectedUtility === 'gate') {
    let reports = 0;
    let review = false;
    let root = false;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json') continue;
      if (arg === '--format' && ['compatibility', 'compact', 'json'].includes(selectedArgs[index + 1])) { index += 1; continue; }
      if (arg === '--review' && !review && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        review = true;
        index += 1;
        continue;
      }
      if (arg === '--root' && !root && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        root = true;
        index += 1;
        continue;
      }
      if (!arg.startsWith('-')) { reports += 1; continue; }
      return false;
    }
    return reports > 0 && review;
  }
  if (selectedUtility === 'inventory') {
    let paths = 0;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json') continue;
      if (arg === '--format' && ['compatibility', 'compact', 'json'].includes(selectedArgs[index + 1])) { index += 1; continue; }
      if (!arg.startsWith('-')) { paths += 1; continue; }
      return false;
    }
    return paths <= 1;
  }
  if (selectedUtility === 'validate') {
    let root = false;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--root' && !root && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        root = true;
        index += 1;
        continue;
      }
      return false;
    }
    return true;
  }
  if (selectedUtility === 'roll') {
    let mode = null;
    let key = null;
    let generation = null;
    let reroll = false;
    let json = false;
    let root = false;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json' && !json) { json = true; continue; }
      if (arg === '--reroll' && !reroll) { reroll = true; continue; }
      if (arg === '--root' && !root && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        root = true;
        index += 1;
        continue;
      }
      if (['--mode', '--key', '--gen'].includes(arg) && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        const value = selectedArgs[index + 1];
        if (arg === '--mode' && mode === null) mode = value;
        else if (arg === '--key' && key === null) key = value;
        else if (arg === '--gen' && generation === null && /^\d+$/.test(value)) generation = value;
        else return false;
        index += 1;
        continue;
      }
      return false;
    }
    return (!mode || ['persuade', 'operate', 'analyze', 'read', 'experience'].includes(mode)) &&
      !(reroll && generation !== null) && (!reroll || key !== null);
  }
  if (selectedUtility === 'validate-skill') {
    let root = false;
    let json = false;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json' && !json) { json = true; continue; }
      if (arg === '--root' && !root && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        root = true;
        index += 1;
        continue;
      }
      return false;
    }
    return true;
  }
  if (selectedUtility === 'status') {
    let surface = 0;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--json') continue;
      if (arg === '--format' && ['compatibility', 'compact', 'json'].includes(selectedArgs[index + 1])) { index += 1; continue; }
      if (!arg.startsWith('-')) { surface += 1; continue; }
      return false;
    }
    return surface <= 1;
  }
  if (selectedUtility === 'verify') {
    const positional = selectedArgs.filter(arg => !arg.startsWith('-'));
    return positional.length <= 1 && selectedArgs.every(arg => !arg.startsWith('-') || arg === '--json');
  }
  if (selectedUtility === 'context') {
    if (selectedArgs.filter(arg => arg === '--experimental').length !== 1) return false;
    let root = false;
    for (let index = 0; index < selectedArgs.length; index += 1) {
      const arg = selectedArgs[index];
      if (arg === '--experimental' || arg === '--json') continue;
      if (arg === '--root' && !root && selectedArgs[index + 1] && !selectedArgs[index + 1].startsWith('-')) {
        root = true;
        index += 1;
        continue;
      }
      return false;
    }
    return true;
  }
  return false;
}

async function resolveNative() {
  if (!capability.nativeDelegation || !canDelegateNative()) return { binary: null, reason: capability.manualFallback ? 'manual_unavailable' : 'native_not_selected' };
  const binary = process.env.CONSIDERED_RS_BIN || 'considered-rs';
  return new Promise(resolveNative => {
    const probe = spawn(binary, ['--protocol-version'], { stdio: ['ignore', 'pipe', 'ignore'] });
    let stdout = '';
    probe.stdout.on('data', chunk => { stdout += chunk; });
    probe.once('error', error => resolveNative({ binary: null, reason: error.code === 'ENOENT' ? 'native_missing' : 'native_probe_failed' }));
    probe.once('exit', code => resolveNative(code === 0 && stdout.trim() === capabilityMatrix.protocolVersion
      ? { binary, reason: null }
      : { binary: null, reason: code === 0 ? 'native_protocol_incompatible' : 'native_probe_failed' }));
  });
}

const native = await resolveNative();
const nativeBinary = native.binary;
const machineJson = new Set(['contract', 'gate', 'inventory', 'lint', 'roll', 'validate-skill', 'status', 'verify', 'context']);
const wantsMachineJson = globalJson || (envelope && machineJson.has(selectedUtility));
// The native binary accepts --json only on the utilities in machineJson; a
// global --json ahead of `validate` must not be forwarded to it (the Node
// script tolerates the flag, the Rust subcommand rejects it with exit 2).
const nativeWantsJson = wantsMachineJson && machineJson.has(selectedUtility) && !selectedArgs.includes('--json');
const selectedChildArgs = nativeWantsJson ? [...selectedArgs, '--json'] : selectedArgs;
// These utilities operate on the skill's OWN package data (assets/rules/,
// decks, SKILL.md, the route manifest) rather than on the caller's project,
// so their root is always this skill's install location — never the
// invocation cwd. The Node fallback gets this right implicitly (each script
// locates itself via import.meta.url), but the native binary only knows its
// own cwd, so it resolves the wrong root (or none) whenever it is invoked
// from outside the skill directory. Pin it explicitly to the skill root the
// dispatcher already knows, so both engines agree from any invocation
// directory. (`status`/`verify` are deliberately excluded: they report the
// CALLER's .considered/ design-surface state, which is genuinely
// cwd-relative by design, not a bug.)
const SKILL_ROOTED_UTILITIES = new Set(['validate', 'validate-skill', 'contract', 'gate', 'roll', 'context']);
const nativeChildArgs = SKILL_ROOTED_UTILITIES.has(selectedUtility) && !selectedChildArgs.includes('--root')
  ? [...selectedChildArgs, '--root', ROOT]
  : selectedChildArgs;
const childArgs = nativeBinary
  ? [selectedUtility, ...nativeChildArgs]
  : capability.nodeFallback
    ? [join(ROOT, 'scripts', UTILITIES[selectedUtility]), ...(wantsMachineJson && !selectedArgs.includes('--json') ? ['--json'] : []), ...selectedArgs]
    : [];

function envelopeEngine() {
  if (nativeBinary) return ENGINE.NATIVE_RUST;
  return !capability.nodeFallback || native.reason === 'manual_unavailable' ? ENGINE.MANUAL_UNAVAILABLE : ENGINE.NODE_FALLBACK;
}

function engineEnvelope(result, error = undefined) {
  const value = {
    schemaVersion: 'considered-engine-envelope/v1',
    engine: envelopeEngine(),
    protocolVersion: capabilityMatrix.protocolVersion,
    fallback: error?.code === 'native_child_failed'
      ? 'native_child_failed'
      : (!nativeBinary && !capability.nodeFallback ? 'manual_unavailable' : native.reason),
    result
  };
  if (error) value.error = error;
  return value;
}

if (!nativeBinary && !capability.nodeFallback) {
  const manual = capability.manualFallback
    ? ` Install it with "cargo install --path crates/considered-cli", or run "cargo run -p considered-cli -- ${selectedUtility} ${selectedArgs.join(' ')}".`
    : '';
  const message = `native command "${selectedUtility}" is unavailable.${manual}`;
  if (envelope) {
    process.stdout.write(`${JSON.stringify(engineEnvelope(null, { code: 'native_unavailable', message }))}\n`);
  } else {
    console.error(`considered ${selectedUtility}: ${message}`);
  }
  process.exit(2);
}

if (envelope) {
  const child = spawn(nativeBinary || process.execPath, childArgs, { stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = ''; let stderr = '';
  child.stdout.on('data', chunk => { stdout += chunk; });
  child.stderr.on('data', chunk => { stderr += chunk; });
  child.once('error', error => {
    process.stdout.write(`${JSON.stringify(engineEnvelope(null, { code: 'child_start_failed', message: error.message }))}\n`);
    process.exit(2);
  });
  child.once('exit', (code, signal) => {
    if (stderr) process.stderr.write(stderr);
    let result;
    try { result = JSON.parse(stdout); } catch { result = { stdout }; }
    // Exit codes 1 (findings/failed gate) and 3 (deck exhaustion) are
    // legitimate domain outcomes, not engine failures, and must stay
    // error-free so callers do not mistake a normal result for a crash. Exit
    // 2 is documented as strictly a usage/config error (see considered-rs
    // --help and each Node script's own usage diagnostics) and never a valid
    // protocol response, so it always carries an error — and this rule is
    // engine-agnostic: a usage error means the same thing whether it came
    // from the native binary or the Node fallback, so both engines produce
    // the identical error shape (only `result`/`fallback`/`engine` may
    // differ). An unexpected exit code or signal outside 0/1/2/3, however,
    // remains a native-only failure mode: it signals a native child
    // genuinely crashing, which has no Node-side equivalent to compare.
    const domainExit = [0, 1, 3].includes(code);
    const failure = !signal && code === 2
      ? { code: 'usage_error', message: stderr.trim() || 'command exited with a usage error.', exitCode: 2 }
      : nativeBinary && (signal || !domainExit)
        ? { code: 'native_child_failed', signal: signal || null, exitCode: code ?? 1 }
        : undefined;
    process.stdout.write(`${JSON.stringify(engineEnvelope(result, failure))}\n`);
    process.exit(signal ? 1 : (code ?? 1));
  });
} else {

let settled = false;
let child;
try {
  child = spawn(nativeBinary || process.execPath, childArgs, { stdio: 'inherit' });
} catch (error) {
  console.error(`considered: failed to start ${selectedUtility}: ${error.message}`);
  process.exit(2);
}

child.once('error', error => {
  if (settled) return;
  settled = true;
  console.error(`considered: failed to start ${selectedUtility}: ${error.message}`);
  process.exit(2);
});

child.once('exit', (code, signal) => {
  if (settled) return;
  settled = true;
  if (signal) {
    console.error(`considered: ${selectedUtility} terminated by signal ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
}
