#!/usr/bin/env node
/** Serve one build directory and capture the two required screenshots. */
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile, readdir, mkdir, writeFile, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { homedir } from 'node:os';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';

const MOBILE_WIDTH = 390;
const TYPES = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.json': 'application/json; charset=utf-8', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp', '.gif': 'image/gif', '.ico': 'image/x-icon', '.woff2': 'font/woff2', '.woff': 'font/woff' };

const args = process.argv.slice(2);
const value = flag => { const index = args.indexOf(flag); return index === -1 ? null : args[index + 1] || null; };
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node capture.mjs --run <workdir> --scenario <scenario-json-path> --out <run-dir> [--prep <script.mjs>]');
  process.exit(0);
}
const runDir = value('--run');
const scenarioPath = value('--scenario');
const outDir = value('--out');
const prepPath = value('--prep');
if (!runDir || !scenarioPath || !outDir) { console.error('capture requires --run, --scenario, and --out.'); process.exit(2); }
const root = resolve(runDir);
const out = resolve(outDir);
const scenario = JSON.parse(await readFile(resolve(scenarioPath), 'utf8'));
const route = scenario.build?.route ?? '/';
const viewport = scenario.build?.viewport ?? { width: 1440, height: 900 };

/* ---------- browser resolution ----------
   No repo-local node_modules: this resolves an already-installed Playwright rather than
   installing one, and pairs it with an already-downloaded Chromium build when the exact
   revision Playwright expects is absent. Both are recorded in capture.json so the
   environment freeze reports what actually ran. */

const require = createRequire(import.meta.url);
async function resolvePlaywright() {
  const candidates = [];
  if (process.env.CONSIDERED_PLAYWRIGHT) candidates.push(process.env.CONSIDERED_PLAYWRIGHT);
  candidates.push('playwright');
  const npx = join(homedir(), '.npm', '_npx');
  try {
    for (const entry of await readdir(npx)) {
      const path = join(npx, entry, 'node_modules', 'playwright');
      try { if ((await stat(path)).isDirectory()) candidates.push(path); } catch { /* not this cache entry */ }
    }
  } catch { /* no npx cache */ }
  for (const candidate of candidates) {
    try { return { playwright: require(candidate), source: candidate, version: require(`${candidate}/package.json`).version }; }
    catch { /* try the next candidate */ }
  }
  console.error('No installed Playwright found. Set CONSIDERED_PLAYWRIGHT to a playwright package directory.');
  process.exit(2);
}

async function chromiumExecutable() {
  if (process.env.CONSIDERED_CHROMIUM) return process.env.CONSIDERED_CHROMIUM;
  const base = process.env.PLAYWRIGHT_BROWSERS_PATH || join(homedir(), 'Library', 'Caches', 'ms-playwright');
  const relative = [
    join('chrome-mac-arm64', 'Google Chrome for Testing.app', 'Contents', 'MacOS', 'Google Chrome for Testing'),
    join('chrome-mac', 'Google Chrome for Testing.app', 'Contents', 'MacOS', 'Google Chrome for Testing'),
    join('chrome-mac-arm64', 'Chromium.app', 'Contents', 'MacOS', 'Chromium'),
    join('chrome-linux', 'chrome'),
    join('chrome-win', 'chrome.exe')
  ];
  let builds = [];
  try { builds = (await readdir(base)).filter(name => /^chromium-\d+$/.test(name)).sort((a, b) => Number(b.split('-')[1]) - Number(a.split('-')[1])); } catch { return null; }
  for (const build of builds) {
    for (const tail of relative) {
      const path = join(base, build, tail);
      try { await stat(path); return path; } catch { /* try the next layout */ }
    }
  }
  return null;
}

const { playwright, source, version } = await resolvePlaywright();
let browser;
let executablePath = null;
try { browser = await playwright.chromium.launch(); }
catch (cause) {
  executablePath = await chromiumExecutable();
  if (!executablePath) { console.error(`Cannot launch Chromium: ${cause.message.split('\n')[0]}`); process.exit(2); }
  browser = await playwright.chromium.launch({ executablePath });
}

/* ---------- static server ---------- */

const server = createServer(async (request, response) => {
  const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
  const relative = normalize(pathname).replace(/^([/\\])+/, '');
  if (relative.split(/[\\/]/).includes('..')) { response.writeHead(403).end(); return; }
  let file = join(root, relative);
  // A directory served without its trailing slash makes the browser resolve
  // relative asset URLs against the parent, so styles/scripts 404 and the page
  // renders unstyled. Redirect to the canonical trailing-slash form instead.
  try {
    if ((await stat(file)).isDirectory() && relative !== '' && !pathname.endsWith('/')) {
      response.writeHead(301, { location: `${pathname}/` }).end();
      return;
    }
  } catch { /* fall through to the miss handling below */ }
  try { if ((await stat(file)).isDirectory()) file = join(file, 'index.html'); }
  catch {
    const ext = extname(relative).toLowerCase();
    // Routes fall back to the shell. "No extension" is not enough: a route like
    // /changelog/3.4.0 has extname ".0", so anything that is not a known asset
    // type is treated as a navigation. Known asset types retry by basename at the
    // root, because builds reference styles.css/app.js relative to a nested route.
    if (!TYPES[ext]) file = join(root, 'index.html');
    else file = join(root, relative.split(/[\\/]/).pop());
  }
  try {
    const body = await readFile(file);
    response.writeHead(200, { 'content-type': TYPES[extname(file).toLowerCase()] || 'application/octet-stream' }).end(body);
  } catch { response.writeHead(404, { 'content-type': 'text/plain' }).end('Not found'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = new URL(route, `http://127.0.0.1:${server.address().port}`).href;

/* ---------- capture ---------- */

// State-matched capture: gold references are usually shot mid-session (a tab
// open, a step reached, a form mid-fill), while an idle load shows none of
// that. --prep names a module whose default export receives the Playwright
// page after load and drives it into the state the golds show. A failing prep
// exits loudly — a silent fallback to idle would masquerade as state-matched.
let prep = null;
if (prepPath) {
  const mod = await import(pathToFileURL(resolve(prepPath)).href);
  if (typeof mod.default !== 'function') { console.error(`--prep module must default-export an async (page) function: ${prepPath}`); process.exit(2); }
  prep = mod.default;
}

await mkdir(out, { recursive: true });
const shots = [];
for (const [name, width, height] of [['desktop.png', viewport.width, viewport.height], ['mobile.png', MOBILE_WIDTH, 844]]) {
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  await page.goto(url, { waitUntil: 'networkidle' });
  // networkidle is blind to setTimeout-driven loading skeletons: a simulated
  // fetch makes no network requests, so the page "settles" mid-skeleton. Give
  // scripted transitions a uniform window to finish before shooting.
  await page.waitForTimeout(1800);
  if (prep) {
    try { await prep(page); }
    catch (cause) { console.error(`prep failed on ${name}: ${cause.message.split('\n')[0]}`); process.exit(2); }
    await page.waitForTimeout(600);
  }
  const path = join(out, name);
  await page.screenshot({ path, fullPage: true });
  const box = await page.evaluate(() => [document.documentElement.clientWidth, Math.max(document.documentElement.scrollHeight, document.documentElement.clientHeight)]);
  shots.push({ file: name, width: box[0], height: box[1], viewport: { width, height } });
  await context.close();
}
const record = {
  captured_at: new Date().toISOString(),
  scenario_id: scenario.id,
  route,
  url,
  playwright_version: version,
  playwright_source: source,
  chromium_executable: executablePath,
  browser_version: browser.version(),
  prep: prepPath
    ? { path: resolve(prepPath), sha256: createHash('sha256').update(await readFile(resolve(prepPath))).digest('hex') }
    : null,
  screenshots: shots
};
await writeFile(join(out, 'capture.json'), `${JSON.stringify(record, null, 2)}\n`);
await browser.close();
server.close();
console.log(JSON.stringify(record, null, 2));
