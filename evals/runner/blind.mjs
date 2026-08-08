#!/usr/bin/env node
/** Assemble blinded A/B packets and write the label map reviewers never see. */
import { cp, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { extname, join, relative, resolve } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const CONDITIONS = ['baseline', 'considered'];
const TEXT = new Set(['.html', '.htm', '.css', '.js', '.mjs', '.cjs', '.ts', '.tsx', '.jsx', '.json', '.md', '.txt', '.svg']);
// Treatment metadata, not behavior: artifact directories, lint output, and the contract copy.
const DROP_DIRS = new Set(['.considered', 'skill', 'node_modules', '.git']);
const DROP_FILES = new Set(['contract.md', 'contract.txt', 'prompt.txt', 'stage.json', 'environment.json', 'manifest.json', 'capture.json']);
const NEUTRAL = 'assessed';

const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
if (args.includes('--help') || args.includes('-h')) { console.log('Usage: node blind.mjs --results <result-root> --seed <n>'); process.exit(0); }
const resultsArg = value('--results');
const seed = value('--seed');
if (!resultsArg || !seed) { console.error('blind requires --results <result-root> and --seed <n>.'); process.exit(2); }
const root = resolve(resultsArg);

function cyrb128(str) {
  let h1 = 1779033703, h2 = 3144134277, h3 = 1013904242, h4 = 2773480762;
  for (let i = 0; i < str.length; i++) {
    const k = str.charCodeAt(i);
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067);
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233);
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213);
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179);
  }
  h1 = Math.imul(h3 ^ (h1 >>> 18), 597399067);
  h2 = Math.imul(h4 ^ (h2 >>> 22), 2869860233);
  h3 = Math.imul(h1 ^ (h3 >>> 17), 951274213);
  h4 = Math.imul(h2 ^ (h4 >>> 19), 2716044179);
  return (h1 ^ h2 ^ h3 ^ h4) >>> 0;
}
function mulberry32(seed) {
  let a = seed >>> 0;
  return function () {
    a = (a + 0x6D2B79F5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const exists = async path => { try { await stat(path); return true; } catch { return false; } };
const isComment = line => /^\s*(\/\/|\/\*|\*|<!--|#|-->|\*\/)/.test(line);

/** Remove the contract block and treatment-revealing comments; rename remaining mentions
 *  uniformly so behavior survives the redaction. */
function sanitizeText(text) {
  const lines = text.split('\n');
  const kept = [];
  let redactions = 0;
  let contractStyle = null; // 'block' redacts until the comment closer; 'line' until a non-comment line
  let pendingOpener = null;
  let lastDropped = false; // a lone closer is an orphan only when the line(s) before it were redacted
  for (const line of lines) {
    if (contractStyle === 'block') {
      redactions++;
      if (/(-->|\*\/)/.test(line)) contractStyle = null;
      lastDropped = true;
      continue;
    }
    if (contractStyle === 'line') {
      if (isComment(line) || line.trim() === '') { redactions++; lastDropped = true; continue; }
      contractStyle = null; // first real content line ends a line-comment wrapper and must survive
    }
    if (/CONSIDERED-CONTRACT\b/i.test(line)) {
      contractStyle = pendingOpener !== null || /(<!--|\/\*)/.test(line) ? 'block' : 'line';
      if (contractStyle === 'block' && /(-->|\*\/)/.test(line) && !/(<!--|\/\*).*CONSIDERED-CONTRACT/i.test(line)) contractStyle = null; // one-line wrapper
      redactions += pendingOpener === null ? 1 : 2;
      pendingOpener = null;
      lastDropped = true;
      continue;
    }
    if (pendingOpener !== null) { kept.push(pendingOpener); pendingOpener = null; lastDropped = false; }
    // A bare comment opener may be the contract wrapper's first line; hold it until
    // the next line shows whether it opens the contract or ordinary content.
    if (/^\s*(<!--|\/\*)\s*$/.test(line)) { pendingOpener = line; continue; }
    // Either condition name in a comment names the treatment: "considered" anywhere,
    // "baseline" when an evidence or build script annotates which run it belongs to.
    // Code lines keep both words (CSS `align-items: baseline` must survive).
    // Rule-code tokens (HON-04, A11Y-07, HIER-13 — always two-digit) in comments are
    // method residue that fingerprints a structured pipeline: 14 of 30 m0-002 review
    // forms flagged exactly this class. Two digits exactly spares UTF-8 and ISO-8601.
    if ((/considered|baseline/i.test(line) || /\b[A-Z][A-Z0-9]{1,7}-\d{2}\b/.test(line)) && isComment(line)) { redactions++; lastDropped = true; continue; }
    // A closer whose opener was just redacted is an orphan; a closer after kept
    // content terminates a real comment and must survive, or the file stops parsing.
    if (lastDropped && /^\s*(-->|\*\/)\s*$/.test(line)) { redactions++; lastDropped = false; continue; }
    kept.push(line);
    lastDropped = false;
  }
  if (pendingOpener !== null) kept.push(pendingOpener);
  let body = kept.join('\n');
  // Absolute filesystem paths reveal the treatment: staging workdirs end in
  // -base/-cons, and run directories name their condition outright. Reviewers
  // never need a host path, so redact them all rather than enumerate suffixes.
  const paths = body.match(/(?:\/(?:private|Users|tmp|home|var))(?:\/[\w.@-]+)+/g);
  if (paths) { redactions += paths.length; body = body.replace(/(?:\/(?:private|Users|tmp|home|var))(?:\/[\w.@-]+)+/g, '<redacted-path>'); }
  const mentions = body.match(/considered/gi);
  if (mentions) { redactions += mentions.length; body = body.replace(/considered/gi, match => (match[0] === match[0].toUpperCase() ? NEUTRAL[0].toUpperCase() + NEUTRAL.slice(1) : NEUTRAL)); }
  return { body, redactions };
}

async function sanitizeInto(from, to) {
  let redactions = 0;
  const files = [];
  async function walk(dir, prefix) {
    for (const entry of (await readdir(dir, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const lower = entry.name.toLowerCase();
      if (entry.isDirectory()) {
        if (DROP_DIRS.has(lower) || lower.includes('considered')) { redactions++; continue; }
        await walk(join(dir, entry.name), join(prefix, entry.name));
        continue;
      }
      if (DROP_FILES.has(lower) || lower.includes('considered') || /^lint-.*\.json$/.test(lower)) { redactions++; continue; }
      const target = join(to, prefix, entry.name);
      await mkdir(join(to, prefix), { recursive: true });
      if (TEXT.has(extname(lower))) {
        const clean = sanitizeText(await readFile(join(dir, entry.name), 'utf8'));
        redactions += clean.redactions;
        await writeFile(target, clean.body);
        // Tripwire: sanitization must never break a script. A packet that stops
        // parsing misrepresents the build to its reviewers.
        if (['.js', '.mjs', '.cjs'].includes(extname(lower))) {
          try { await promisify(execFile)(process.execPath, ['--check', target]); }
          catch (error) {
            console.error(`Sanitizer broke ${target}: ${error.stderr || error.message}`);
            process.exit(1);
          }
        }
        // Tripwire: no comment in a written packet may still name a condition — including
        // block-comment interiors the line dropper's model misses. Loud failure beats a leak.
        let inBlock = false;
        for (const line of clean.body.split('\n')) {
          const commentary = inBlock || isComment(line);
          if (/\/\*|<!--/.test(line) && !/\*\/|-->/.test(line)) inBlock = true;
          if (/\*\/|-->/.test(line)) inBlock = false;
          if (commentary && (/considered|baseline/i.test(line) || /\b[A-Z][A-Z0-9]{1,7}-\d{2}\b/.test(line))) {
            console.error(`Condition name or rule-code residue survived sanitization in ${target}: ${line.trim()}`);
            process.exit(1);
          }
        }
      } else await cp(join(dir, entry.name), target);
      files.push(join(prefix, entry.name).split('\\').join('/'));
    }
  }
  await walk(from, '');
  return { redactions, files };
}

const packetsDir = join(root, 'blind-packets');
await rm(packetsDir, { recursive: true, force: true });
const packets = [];
const labelMap = { seed, created_at: new Date().toISOString(), packets: {} };

const scenarios = await readdir(join(root, CONDITIONS[0])).catch(() => {
  console.error(`No ${CONDITIONS[0]}/ directory under ${root}.`);
  process.exit(2);
});
for (const scenarioId of scenarios.sort()) {
  const runs = {};
  for (const condition of CONDITIONS) {
    const scenarioDir = join(root, condition, scenarioId);
    const runDirs = (await readdir(scenarioDir).catch(() => [])).sort();
    const chosen = runDirs.at(-1);
    if (!chosen) { console.error(`${scenarioId}: no ${condition} run directory.`); process.exit(1); }
    runs[condition] = join(scenarioDir, chosen);
  }
  const aCondition = mulberry32(cyrb128(`${seed}:packet:${scenarioId}`))() < 0.5 ? 'baseline' : 'considered';
  const byLabel = { A: aCondition, B: aCondition === 'baseline' ? 'considered' : 'baseline' };
  const packetId = `packet-${scenarioId}`;
  const packetDir = join(packetsDir, packetId);
  const contents = {};
  const redactions = {};
  const runIds = {};

  for (const label of ['A', 'B']) {
    const from = runs[byLabel[label]];
    const to = join(packetDir, label);
    await mkdir(to, { recursive: true });
    const manifest = JSON.parse(await readFile(join(from, 'manifest.json'), 'utf8').catch(() => '{}'));
    runIds[label] = manifest.run_id ?? null;
    const entries = [];
    for (const shot of ['desktop.png', 'mobile.png']) {
      if (!await exists(join(from, shot))) { console.error(`${scenarioId}: missing ${shot} for one condition.`); process.exit(1); }
      await cp(join(from, shot), join(to, shot));
      entries.push(shot);
    }
    if (await exists(join(from, 'evidence'))) {
      const evidence = await sanitizeInto(join(from, 'evidence'), join(to, 'evidence'));
      redactions[label] = evidence.redactions;
      entries.push(...evidence.files.map(file => `evidence/${file}`));
    } else redactions[label] = 0;
    const source = await sanitizeInto(join(from, 'source'), join(to, 'source'));
    redactions[label] += source.redactions;
    entries.push(...source.files.map(file => `source/${file}`));
    contents[label] = entries.sort();
  }

  // The packet manifest carries no condition, no run id, and no redaction count: any of
  // the three would let a reviewer infer the treatment.
  await writeFile(join(packetDir, 'manifest.json'), `${JSON.stringify({
    packet_id: packetId,
    scenario_id: scenarioId,
    scenario_brief: relative(root, join(root, 'scenarios', `${scenarioId}.json`)).split('\\').join('/'),
    labels: ['A', 'B'],
    contents
  }, null, 2)}\n`);

  packets.push({ packet_id: packetId, scenario_id: scenarioId });
  labelMap.packets[packetId] = {
    scenario_id: scenarioId,
    condition_by_label: byLabel,
    run_id_by_label: runIds,
    redactions_by_label: redactions
  };
}

await writeFile(join(root, 'label-map.json'), `${JSON.stringify(labelMap, null, 2)}\n`);
console.log(JSON.stringify({ packets, label_map: 'label-map.json', packet_root: relative(root, packetsDir) }, null, 2));
