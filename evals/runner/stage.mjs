#!/usr/bin/env node
/** Stage one condition run: fresh starter copy, frozen prompt, environment skeleton. */
import { cp, mkdir, readFile, writeFile, lstat, rm } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { promisify } from 'node:util';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..', '..');
const STARTER = join(ROOT, 'evals', 'starter');

// The one line that separates the conditions. RUNNER.md quotes it; keep the two in step.
const SKILL_POINTER = 'An Agent Skill is installed at ./skill. Read ./skill/SKILL.md and follow its workflow for this task.';

const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node stage.mjs --scenario <id> --condition baseline|considered --evaluation-id <id> --workdir <path> [--run-id <id>]');
  process.exit(0);
}
const scenarioId = value('--scenario');
const condition = value('--condition');
const evaluationId = value('--evaluation-id');
const workdirArg = value('--workdir');
if (!scenarioId || !evaluationId || !workdirArg) { console.error('stage requires --scenario, --evaluation-id, and --workdir.'); process.exit(2); }
if (!['baseline', 'considered'].includes(condition)) { console.error('--condition must be baseline or considered.'); process.exit(2); }
const runId = value('--run-id') || `${scenarioId}-${condition}-001`;
const workdir = resolve(workdirArg);
let occupied = false;
try { await lstat(workdir); occupied = true; } catch (error) { if (error.code !== 'ENOENT') throw error; }
if (occupied) { console.error(`${workdir} exists. Every run needs a fresh working directory.`); process.exit(2); }

const sha256 = text => createHash('sha256').update(text).digest('hex');
const scenarioPath = join(ROOT, 'evals', 'scenarios', `${scenarioId}.json`);
let scenarioText;
try { scenarioText = await readFile(scenarioPath, 'utf8'); }
catch { console.error(`No scenario file at ${scenarioPath}.`); process.exit(2); }
const scenario = JSON.parse(scenarioText);

// The envelope is not retyped here. It is lifted from protocol.md so the two cannot drift.
const protocol = await readFile(join(ROOT, 'evals', 'protocol.md'), 'utf8');
const envelope = protocol.match(/verbatim[^\n]*\n+```text\n([\s\S]*?)```/)?.[1];
if (!envelope || !envelope.includes('<scenario-json>')) { console.error('Cannot lift the neutral envelope from evals/protocol.md.'); process.exit(2); }
const body = envelope.trimEnd().replace('<scenario-json>', scenarioText.trim());
const prompt = condition === 'considered' ? `${SKILL_POINTER}\n\n${body}\n` : `${body}\n`;

await cp(STARTER, workdir, { recursive: true });
await writeFile(join(workdir, 'prompt.txt'), prompt);
if (condition === 'considered') {
  await promisify(execFile)(process.execPath, [join(ROOT, 'scripts', 'install.mjs'), '--dest', join(workdir, 'skill')]);
  // Treatment isolation: the shipped package carries the evaluation harness, and the
  // scenario catalog inside it contains the very brief this run is building. A builder
  // that finds its own brief in an eval catalog knows it is being measured. Prune eval
  // material from the installed copy; the treatment is the skill, not the measurement.
  for (const dir of ['evals', 'docs', 'tests']) await rm(join(workdir, 'skill', dir), { recursive: true, force: true });
}

const git = async argv => { try { return (await promisify(execFile)('git', argv, { cwd: ROOT })).stdout.trim() || null; } catch { return null; } };
const environment = {
  runner: 'evals/runner (Claude Code executor subagents)',
  model: null,
  model_revision: null,
  starter_project_revision: await git(['rev-parse', 'HEAD']),
  dependency_lock_sha256: sha256(''),
  node_version: process.version,
  browser_version: null,
  operating_system: `${process.platform} ${process.arch}`,
  network_policy: 'disabled',
  time_budget_seconds: null,
  token_budget: null,
  temperature: null,
  seed: null,
  randomization_seed: null,
  common_prompt_sha256: sha256(body)
};
await mkdir(workdir, { recursive: true });
// Staging metadata never enters the workdir: a builder that reads its own
// condition or evaluation id is no longer an uninformed sample.
await writeFile(`${workdir}.environment.json`, `${JSON.stringify(environment, null, 2)}\n`);

const stage = {
  evaluation_id: evaluationId,
  run_id: runId,
  scenario_id: scenarioId,
  condition,
  status: 'incomplete',
  started_at: new Date().toISOString(),
  prompt_sha256: sha256(prompt),
  scenario_sha256: sha256(scenarioText),
  condition_context_sha256: condition === 'considered' ? sha256(SKILL_POINTER) : sha256(''),
  workdir,
  route: scenario.build?.route ?? '/',
  viewport: scenario.build?.viewport ?? null
};
await writeFile(`${workdir}.stage.json`, `${JSON.stringify(stage, null, 2)}\n`);
console.log(JSON.stringify(stage, null, 2));
