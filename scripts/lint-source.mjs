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
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node lint-source.mjs [directory|file] [--json] [--enforce-heuristics]\n\nRuns static heuristic source checks against one directory or file and\nreports review leads. It does not prove semantic, visual, or per-viewport\ndesign requirements — pair it with the independent review gate.');
  process.exit(0);
}
const json = args.includes('--json');
const enforce = args.includes('--enforce-heuristics');
const positional = args.filter(arg => !['--json', '--enforce-heuristics'].includes(arg));
if (positional.length > 1) {
  console.error(`lint-source accepts a single directory or file, not multiple paths (got ${positional.length}: ${positional.join(', ')}). Pass the shared parent directory instead, e.g. "." to lint the whole project.`);
  process.exit(2);
}
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

// HON-03 (craft.md / guidance.json): sample data instantiates, never asserts.
// Detect instance-data patterns (emails, card numbers, invoice/period labels,
// recency phrases, name+email pairs, current-value settings) that are not
// marked illustrative anywhere nearby. These are leads for a reviewer against
// the brief's fact list, not proof — the checker cannot know what the brief
// supplied, only that the text reads as a specific real-world instance.
const MONTHS = 'January|February|March|April|May|June|July|August|September|October|November|December';
const EMAIL_PATTERN = /\b[\w.+-]+@[\w-]+\.[\w.-]+\b/g;
// The final alternative catches the raw-data-field shape a card's last four
// digits usually take before a template renders it into "Card ending in
// 4242" text (e.g. `paymentMethodLast4: "4242"`), which the phrase patterns
// above never see because the phrase is assembled at render time. The key
// name is anchored to card/last-four vocabulary (never a bare "card" or
// "wildcard" substring) and the value must be a quoted 4-digit string, so
// `cardWidth = 1280` and `wildcardTimeout = 3000` never match.
const CARD_KEY_PATTERN = String.raw`\b(?:\w*last_?4|lastFour|card_?number)\b`;
const CARD_PATTERN = new RegExp(
  String.raw`(?:Card\s+)?ending in \d{4}|•{4}\s?\d{4}|\*{4}\s?\d{4}|last 4 \d{4}|${CARD_KEY_PATTERN}\s*[:=]\s*["']\d{4}["']`,
  'gi'
);
const INVOICE_PATTERN = new RegExp(`(?:Invoice\\s*[\\u2014\\u2013-]?\\s*)?(?:${MONTHS})\\s+\\d{4}`, 'gi');
const RECENCY_PATTERN = /last used \d+ (?:minutes?|hours?|days?|weeks?|months?|years?) ago|\d+ (?:days?|hours?|months?) ago/gi;
// Timezone matches are anchored to actual IANA area names so import
// specifiers like "Components/Button" never qualify. The bare locale label
// form ("Time (ET)") was dropped: it fired on ordinary prose too often to
// carry as a lead.
const IANA_AREAS = 'America|Europe|Asia|Africa|Australia|Pacific|Atlantic|Indian|Antarctic|Etc|US';
const CURRENT_VALUE_PATTERN = new RegExp(
  String.raw`\b(?:${IANA_AREAS})\/[A-Z][A-Za-z_]+\b|\b\d{2}:\d{2}\s*[–-]\s*\d{2}:\d{2}\b|\bMM\/DD\/YYYY\b|\bDD\/MM\/YYYY\b|\bYYYY-MM-DD\b`,
  'g'
);
const NAME_PAIR_PATTERN = /\b[A-Z][a-zA-Z]+ [A-Z][a-zA-Z]+\b/g;
const SUPPRESSION_MARKERS = /illustrative|sample|example|placeholder|data-illustrative|aria-label="illustrative/i;

// Suppression windows are measured in Unicode code points (not UTF-16 code
// units, and never bytes), so a run of multi-byte characters (em dashes,
// curly quotes, etc.) can never shift the window relative to the Rust port,
// which counts the same way over `char`s.
function codePointsBefore(text, index, count) {
  const prefix = Array.from(text.slice(0, index));
  return prefix.slice(Math.max(0, prefix.length - count)).join('');
}
function codePointsPrefix(text, count) {
  return Array.from(text).slice(0, count).join('');
}

function isFileMarkedIllustrative(text) { return /illustrative data/i.test(codePointsPrefix(text, 400)); }
function isSuppressed(text, index, fileIllustrative) {
  if (fileIllustrative) return true;
  return SUPPRESSION_MARKERS.test(codePointsBefore(text, index, 200));
}
function isPlaceholderEmail(email) {
  const domain = (email.split('@')[1] || '').toLowerCase();
  return domain.includes('example.');
}

function collectEmails(text, fileIllustrative) {
  const out = [];
  for (const match of text.matchAll(EMAIL_PATTERN)) {
    if (isPlaceholderEmail(match[0])) continue;
    if (isSuppressed(text, match.index, fileIllustrative)) continue;
    out.push({ index: match.index, value: match[0] });
  }
  return out;
}
function collectCards(text, fileIllustrative) {
  const out = [];
  for (const match of text.matchAll(CARD_PATTERN)) {
    if (isSuppressed(text, match.index, fileIllustrative)) continue;
    out.push(match[0]);
  }
  return out;
}
function collectInvoicePeriods(text, fileIllustrative) {
  const out = [];
  for (const match of text.matchAll(INVOICE_PATTERN)) {
    if (!/^invoice/i.test(match[0])) {
      const before = codePointsBefore(text, match.index, 40);
      if (!/invoice|receipt|billed|statement/i.test(before)) continue;
    }
    if (isSuppressed(text, match.index, fileIllustrative)) continue;
    out.push(match[0]);
  }
  return out;
}
function collectRecency(text, fileIllustrative) {
  const out = [];
  for (const match of text.matchAll(RECENCY_PATTERN)) {
    if (isSuppressed(text, match.index, fileIllustrative)) continue;
    out.push(match[0]);
  }
  return out;
}
function collectPersons(text, fileIllustrative, emails) {
  const out = [];
  for (const { index, value } of emails) {
    const lineStart = text.lastIndexOf('\n', index - 1) + 1;
    const windowStart = Math.max(lineStart, index - 80);
    const before = text.slice(windowStart, index);
    let name = null;
    for (const match of before.matchAll(NAME_PAIR_PATTERN)) name = match[0];
    if (!name) continue;
    if (isSuppressed(text, index, fileIllustrative)) continue;
    out.push(`${name} ${value}`);
  }
  return out;
}
function collectCurrentValues(text, fileIllustrative) {
  const out = [];
  for (const match of text.matchAll(CURRENT_VALUE_PATTERN)) {
    if (isSuppressed(text, match.index, fileIllustrative)) continue;
    out.push(match[0]);
  }
  return out;
}

function checkInstanceData(text, file) {
  const fileIllustrative = isFileMarkedIllustrative(text);
  const emails = collectEmails(text, fileIllustrative);
  const classes = [
    ['email', emails.map(entry => entry.value)],
    ['card', collectCards(text, fileIllustrative)],
    ['invoice_or_period', collectInvoicePeriods(text, fileIllustrative)],
    ['recency', collectRecency(text, fileIllustrative)],
    ['person', collectPersons(text, fileIllustrative, emails)],
    ['current_value', collectCurrentValues(text, fileIllustrative)],
  ];
  const parts = [];
  for (const [name, values] of classes) {
    if (!values.length) continue;
    const distinct = [...new Set(values)];
    const shown = distinct.slice(0, 3);
    const display = values.length > shown.length ? [...shown, '…'] : shown;
    parts.push(`${name} ×${values.length} (${display.join(', ')})`);
  }
  if (!parts.length) return;
  add('HON-03', file, 0, `Instance data not marked illustrative: ${parts.join(', ')}.`, 'Confirm each value is supplied by the brief or label it illustrative / render unset state.');
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
  checkInstanceData(text, rel);
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
