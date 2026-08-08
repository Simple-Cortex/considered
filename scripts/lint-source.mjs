#!/usr/bin/env node
/**
 * Heuristic source audit. It finds review leads from source text; it does not
 * claim to prove semantic, visual, or per-viewport design requirements.
 */
import { readFile, readdir, stat } from 'node:fs/promises';
import { join, extname, relative, basename } from 'node:path';
import { loadRuleCatalog, severityFor, summarizeFindings } from './lib/rules.mjs';

const CODE = new Set(['.css', '.scss', '.sass', '.less', '.jsx', '.tsx', '.js', '.ts', '.mjs', '.cjs', '.mts', '.cts', '.html', '.vue', '.svelte', '.astro', '.mdx']);
const STYLE = new Set(['.css', '.scss', '.sass', '.less']);
const SKIP = new Set(['node_modules', 'dist', 'build', '.next', '.git', 'coverage', 'out', '.turbo', 'vendor']);
const BANNED_LABELS = new Set(['submit', 'ok', 'yes', 'no', 'go', 'continue', 'manage', 'update', 'click here', 'learn more', 'read more']);
const BANNED_HEADINGS = new Set(['overview', 'metrics', 'kpis', 'charts', 'tables', 'other', 'analytics', 'data', 'stats', 'misc']);
const BUDGET = { typeSizes: 5, fontWeights: 3, textColors: 3, radiusValues: 2, shadowLevels: 2, borderStyles: 2 };
const args = process.argv.slice(2);
const json = args.includes('--json');
const enforce = args.includes('--enforce-heuristics');
const positional = args.filter(arg => !['--json', '--enforce-heuristics'].includes(arg));
if (positional.length > 1) { console.error('Usage: node lint-source.mjs [directory|file] [--json] [--enforce-heuristics]'); process.exit(2); }
const target = positional[0] || '.';

let catalog;
try { catalog = await loadRuleCatalog(); }
catch (error) { console.error(`Cannot load rule manifests: ${error.message}`); process.exit(2); }
const findings = [], skipped = [];
function add(rule, file, line, message, hint) {
  const declared = catalog.byId.get(rule);
  if (!declared) throw new Error(`Checker emitted undeclared rule ${rule}.`);
  findings.push({ rule, severity: severityFor(catalog, rule), confidence: 'heuristic', file, line, message, hint });
}
const lineOf = (text, index) => text.slice(0, index).split('\n').length;

async function walk(path, acc = []) {
  let entries;
  try { entries = await readdir(path, { withFileTypes: true }); }
  catch (error) { skipped.push({ path, reason: error.message }); return acc; }
  for (const entry of entries) {
    if (entry.name.startsWith('.') && entry.name !== '.storybook') continue;
    const full = join(path, entry.name);
    if (entry.isDirectory()) { if (!SKIP.has(entry.name)) await walk(full, acc); }
    else if (CODE.has(extname(entry.name))) acc.push(full);
  }
  return acc;
}

function blankBudget() { return Object.fromEntries(Object.keys(BUDGET).map(key => [key, new Set()])); }
function note(budget, key, value) {
  const token = value.trim().toLowerCase();
  if (token && !['inherit', 'initial', 'unset', '0', 'none'].includes(token)) budget[key].add(token);
}
function checkBudget(budget, file) {
  for (const [key, max] of Object.entries(BUDGET)) if (budget[key].size > max) {
    add('COMP-16', file, 0, `${budget[key].size} distinct ${key.replace(/([A-Z])/g, ' $1').toLowerCase()} in one file (budget ${max}).`, 'Review this view-level estimate and consolidate values that do not encode meaning.');
  }
}

function checkStyles(text, file, budget) {
  for (const match of text.matchAll(/font-size\s*:\s*([^;{}]+)/gi)) note(budget, 'typeSizes', match[1]);
  for (const match of text.matchAll(/font-weight\s*:\s*([^;{}]+)/gi)) note(budget, 'fontWeights', match[1]);
  for (const match of text.matchAll(/(?<!background-)(?<!border-)(?<!outline-)\bcolor\s*:\s*([^;{}]+)/gi)) note(budget, 'textColors', match[1]);
  for (const match of text.matchAll(/border-radius\s*:\s*([^;{}]+)/gi)) note(budget, 'radiusValues', match[1]);
  for (const match of text.matchAll(/box-shadow\s*:\s*([^;{}]+)/gi)) note(budget, 'shadowLevels', match[1]);
  for (const match of text.matchAll(/border(?:-(?:top|right|bottom|left))?-style\s*:\s*([^;{}]+)/gi)) note(budget, 'borderStyles', match[1]);
  for (const match of text.matchAll(/outline\s*:\s*(none|0)\s*[;}]/gi)) {
    const near = text.slice(Math.max(0, match.index - 600), match.index + 600);
    if (!/:focus-visible|outline-offset|box-shadow[^;]*focus|focus-within/i.test(near)) add('A11Y-04', file, lineOf(text, match.index), 'Focus outline appears to be removed without a nearby replacement.', 'Verify a visible focus indicator on the affected selector.');
  }
  const offScale = new Set();
  for (const match of text.matchAll(/(?:margin|padding|gap)(?:-[a-z]+)?\s*:\s*([^;{}]+)/gi)) for (const value of match[1].matchAll(/(\d+(?:\.\d+)?)px/g)) if (Number(value[1]) > 0 && Number(value[1]) % 4 !== 0) offScale.add(`${value[1]}px`);
  if (offScale.size) add('COMP-03', file, 0, `Off-scale spacing values: ${[...offScale].slice(0, 6).join(', ')}.`, 'Confirm these are intentional tokens or snap them to the spacing scale.');
  if (/@keyframes|animation\s*:|transition\s*:/i.test(text) && !/prefers-reduced-motion/i.test(text)) add('A11Y-09', file, 0, 'Motion appears without a reduced-motion override in this file.', 'Verify the global stylesheet or add prefers-reduced-motion handling.');
  for (const match of text.matchAll(/\b(td|th|\.cell|\[role="cell"\])[^\{]*\{[^}]*text-align\s*:\s*center/gi)) add('COMP-17', file, lineOf(text, match.index), 'Table cells are center-aligned.', 'Keep text left-aligned and numeric columns right-aligned.');
  for (const match of text.matchAll(/--(?:color-)?(red|green|blue|purple|orange|yellow|pink|teal)(?:-\d+)?\s*:/gi)) add('COMP-04', file, lineOf(text, match.index), `Token --${match[1]} is named for appearance.`, 'Use a semantic role such as --danger, --success, or --accent.');
}

function checkMarkup(text, file) {
  for (const match of text.matchAll(/>\s*([A-Za-z][A-Za-z ]{0,18})\s*</g)) {
    const label = match[1].trim().toLowerCase();
    if (BANNED_LABELS.has(label) && /<(button|a|Button|Link)\b[\s\S]{0,240}$/i.test(text.slice(Math.max(0, match.index - 240), match.index))) add('ACT-05', file, lineOf(text, match.index), `Action label "${match[1].trim()}" may not name an outcome.`, 'Use a concise verb plus object.');
  }
  for (const match of text.matchAll(/<h[1-6][^>]*>\s*([^<]{2,40})</gi)) if (BANNED_HEADINGS.has(match[1].trim().toLowerCase())) add('IA-03', file, lineOf(text, match.index), `Heading "${match[1].trim()}" is a container label.`, 'Name the user question, object, or decision instead.');
  for (const match of text.matchAll(/<button\b([^>]*)>([\s\S]{0,240}?)<\/button>/gi)) {
    const attrs = match[1], inner = match[2], plain = inner.replace(/<[^>]+>/g, ' ').trim();
    const named = /aria-label|aria-labelledby/i.test(attrs) || /aria-label/i.test(inner);
    const icon = /<(svg|Icon|[A-Z][A-Za-z]*Icon)\b/.test(inner);
    if (icon && plain.length < 2 && !named) add('ACT-06', file, lineOf(text, match.index), 'Icon-only button may have no accessible name.', 'Add aria-label and a visible tooltip, or use a text label.');
  }
  const h1s = [...text.matchAll(/<h1\b/gi)];
  if (h1s.length > 1) add('A11Y-11', file, lineOf(text, h1s[1].index), `${h1s.length} h1 elements appear in one source file.`, 'Verify each is not rendered in the same view.');
  const emoji = text.match(/[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]/gu);
  if (emoji?.length > 2) add('COMP-07', file, 0, `${emoji.length} emoji appear in UI source.`, 'Use a coherent icon system when these represent controls or status.');
  const filled = [...text.matchAll(/(?:variant|appearance)\s*=\s*["'](primary|filled|solid|cta)["']/gi)].length + [...text.matchAll(/class(?:Name)?\s*=\s*["'][^"']*\b(?:btn-primary|bg-(?:blue|indigo|violet|primary)-[5-7]00)\b/gi)].length;
  if (filled > 2) add('ACT-01', file, 0, `${filled} high-emphasis action treatments appear in one source file.`, 'Verify they do not share a viewport; demote competing actions.');
  if (/backdrop-(?:blur|filter)/i.test(text) && /bg-white\/\d{1,2}|rgba\(255,\s*255,\s*255,\s*0?\.[0-3]/i.test(text)) add('COMP-07', file, 0, 'Glass treatment detected.', 'Keep it only if the assigned direction and accessibility requirements justify it.');
  if (/bg-gradient-to-[a-z]+[^"']*(?:purple|violet|fuchsia|indigo)[^"']*(?:pink|blue|cyan)/i.test(text)) add('COMP-07', file, 0, 'Purple-blue gradient treatment detected.', 'Verify it is intentional rather than a default AI aesthetic.');
  for (const match of text.matchAll(/class(?:Name)?\s*=\s*["'][^"']*\boutline-none\b[^"']*["']/gi)) if (!/focus(?:-visible)?:/.test(match[0])) add('A11Y-04', file, lineOf(text, match.index), 'Tailwind outline-none appears without a focus utility.', 'Add a focus-visible ring or verify a component-level replacement.');
  if (/(?:animate-|transition-)/.test(text) && !/motion-reduce:|motion-safe:|prefers-reduced-motion/.test(text)) add('A11Y-09', file, 0, 'Tailwind-like motion utility appears without a motion preference utility.', 'Verify the global motion policy or add motion-reduce behavior.');
}

function checkStates(text, file) {
  const fetches = /useQuery|useSWR|fetch\(|axios\.|await\s+\w+\.(?:get|post|find|query)/i.test(text);
  if (!fetches) return;
  const missing = [];
  if (!/isLoading|isPending|loading|Skeleton|Spinner/i.test(text)) missing.push('loading');
  if (!/isError|error|catch\s*\(|ErrorState/i.test(text)) missing.push('error');
  if (!/length\s*===\s*0|isEmpty|EmptyState|\.length\s*\?|no results|nothing/i.test(text)) missing.push('empty');
  if (missing.length) add('STATE-01', file, 0, `Data fetching may lack ${missing.join(', ')} handling.`, 'Inspect the rendered region and record all eight states in REVIEW-PACKET.md.');
}

let stats;
try { stats = await stat(target); }
catch (error) { console.error(`Cannot read ${target}: ${error.message}`); process.exit(2); }
const files = stats.isDirectory() ? await walk(target) : (CODE.has(extname(target)) ? [target] : []);
if (!files.length) { console.error(`No supported source files found under ${target}.`); process.exit(2); }
for (const file of files) {
  let text;
  try { text = await readFile(file, 'utf8'); }
  catch (error) { skipped.push({ path: file, reason: error.message }); continue; }
  const rel = relative(process.cwd(), file) || basename(file);
  const budget = blankBudget();
  checkStyles(text, rel, budget);
  if (!STYLE.has(extname(file))) { checkMarkup(text, rel); checkStates(text, rel); }
  checkBudget(budget, rel);
}
findings.sort((a, b) => a.severity.localeCompare(b.severity) || a.file.localeCompare(b.file) || a.line - b.line);
const counts = summarizeFindings(findings);
const output = { checker: 'source', scope: 'heuristic-source-leads', scanned: files.length, skipped, counts, findings };
if (json) console.log(JSON.stringify(output, null, 2));
else {
  console.log(`\n  SOURCE REVIEW LEADS  ${files.length} files under ${target}`);
  console.log('  These are heuristics. Verify them in the independent review packet.\n');
  if (!findings.length) console.log('  No heuristic leads. This is not a visual or semantic pass.\n');
  else for (const finding of findings) console.log(`  ${finding.rule.padEnd(10)} ${finding.severity}  ${finding.file}${finding.line ? `:${finding.line}` : ''}\n               ${finding.message}\n               ${finding.hint}\n`);
  if (skipped.length) console.log(`  ${skipped.length} unreadable path(s) were skipped.\n`);
  console.log(`  ${counts.S1 || 0} S1, ${counts.S2 || 0} S2, ${counts.S3 || 0} S3 heuristic leads\n`);
}
process.exit(enforce && ((counts.S1 || 0) || (counts.S2 || 0) > 2 || (counts.S3 || 0) > 6) ? 1 : 0);
