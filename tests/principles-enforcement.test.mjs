import { test } from 'node:test';
import assert from 'node:assert';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = path => readFileSync(join(ROOT, path), 'utf8');
const index = JSON.parse(read('assets/principles-index.json'));
const ids = new Set(index.principles.map(p => p.id));

test('STRUCTURE.md template has a Principles Applied section for driving principles only', () => {
  const t = read('assets/templates/STRUCTURE.md');
  assert.ok(t.includes('## Principles Applied'));
  assert.ok(t.includes('Principle ID') && t.includes('Decision context') && t.includes('How it was applied'));
  assert.ok(/drove a decision/.test(t), 'template must say to record only principles that drove a decision');
});

test('no document offers build rationale as a place for principles documentation', () => {
  const workflow = read('references/workflow.md');
  const section = workflow.slice(workflow.indexOf('### `compose`'), workflow.indexOf('### `critique`'));
  assert.ok(section.length > 100, 'compose section found');
  for (const text of [read('assets/templates/STRUCTURE.md'), read('docs/principles-documentation-guide.md'), read('references/critique.md'), section]) {
    assert.ok(!/in (the )?build rationale/i.test(text), 'no "in build rationale" option');
    assert.ok(!/build rationale \(or/i.test(text), 'no "build rationale (or STRUCTURE.md)" option');
  }
});

test('compose step 4 points to STRUCTURE.md and the guide', () => {
  const w = read('references/workflow.md');
  assert.ok(w.includes('"Principles Applied" section'));
  assert.ok(w.includes('principles-documentation-guide.md'));
});

test('guide and template cite only principle IDs present in the index', () => {
  for (const file of ['docs/principles-documentation-guide.md', 'assets/templates/STRUCTURE.md']) {
    const cited = read(file).match(/\b[A-Z]{2,6}-\d{2}\b/g) || [];
    assert.ok(cited.length > 0, `${file} cites examples`);
    for (const id of cited) assert.ok(ids.has(id), `${file} cites ${id}, missing from index`);
  }
});

test('guide uses the index field names', () => {
  const g = read('docs/principles-documentation-guide.md');
  for (const f of ['`modes`', '`decision`', '`tiers`', '`surfaces`']) assert.ok(g.includes(f), f);
});

test('critique pass: independent compliance, entry severity, single S3 for a missing section', () => {
  const c = read('references/critique.md');
  const pass = c.slice(c.indexOf('## Principles Compliance Pass'), c.indexOf('## Severity model'));
  assert.ok(pass.includes('principles-index.json'));
  assert.ok(pass.includes('`severity` field'));
  assert.ok(/single S3/.test(pass));
  assert.ok(!/S2/.test(pass), 'missing section must not be S2');
  assert.ok(/never substitutes|does not substitute/i.test(pass));
});
