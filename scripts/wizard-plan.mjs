/**
 * Pure detection and destination-planning for the interactive install wizard.
 * No IO loop, no prompts — the bin entry point owns the TTY interaction and
 * calls these functions with plain inputs so both can be fixture-tested.
 */
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';

/** Caution paragraph shown verbatim for the "another host" wizard choice. */
export const HOST_CAUTION =
  'Do not treat a suggested path as verified support until it works in your installed harness.';

/**
 * Expand a leading `~` (bare, or `~/...`) against homedir, as a shell would.
 * Only a leading tilde is special; `~` elsewhere in the path is left alone.
 * Non-tilde input passes through unchanged (relative and absolute both).
 */
export function expandHome(inputPath, homedir) {
  if (inputPath === '~') return homedir;
  if (inputPath.startsWith('~/')) return join(homedir, inputPath.slice(2));
  return inputPath;
}

/** Whether the cwd looks like a Claude Code project (.claude/ dir or CLAUDE.md present). */
export function detectProjectHost(cwd) {
  return existsSync(join(cwd, '.claude')) || existsSync(join(cwd, 'CLAUDE.md'));
}

/** Whether a global Claude Code skills directory already exists for this user. */
export function detectGlobalHost(homedir) {
  return existsSync(join(homedir, '.claude', 'skills'));
}

/**
 * Build the wizard's numbered choices for the given cwd/homedir.
 * defaultChoice is '1' only when a project host was detected; otherwise null
 * (the wizard must not silently pick between two undetected options).
 */
export function planChoices({ cwd, homedir }) {
  const projectDetected = detectProjectHost(cwd);
  const globalDetected = detectGlobalHost(homedir);
  return {
    projectDetected,
    globalDetected,
    defaultChoice: projectDetected ? '1' : null,
    choices: [
      { key: '1', label: 'Claude Code — project', dest: resolve(cwd, '.claude', 'skills', 'considered') },
      { key: '2', label: 'Claude Code — global', dest: resolve(homedir, '.claude', 'skills', 'considered') },
      { key: '3', label: "Another Agent Skills host — enter its documented skills directory" }
    ]
  };
}

/** Resolve the destination for the "another host" choice from its supplied skills directory. */
export function resolveCustomDest(skillsDir) {
  return resolve(skillsDir, 'considered');
}

/**
 * The non-interactive `install --yes` plan: proceeds only when a confident
 * default exists (a detected project host), never guesses between hosts.
 */
export function resolveYesPlan({ cwd }) {
  if (!detectProjectHost(cwd)) return null;
  return { dest: resolve(cwd, '.claude', 'skills', 'considered') };
}

/**
 * Interpret the "<dest> already exists — replace it?" prompt.
 * Default (empty answer) is No: an existing install is never touched on a
 * blank Enter. Only an explicit y/yes (case-insensitive) proceeds.
 */
export function interpretUpdateAnswer(answer) {
  const normalized = answer.trim().toLowerCase();
  return normalized === 'y' || normalized === 'yes';
}

/**
 * Interpret the "install at <dest>?" prompt for a destination that does not
 * yet exist. Default (empty answer) is Yes: only an explicit n/no declines.
 */
export function interpretConfirmAnswer(answer) {
  const normalized = answer.trim().toLowerCase();
  return normalized !== 'n' && normalized !== 'no';
}
