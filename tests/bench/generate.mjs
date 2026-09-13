#!/usr/bin/env node
/**
 * Generate medium and large benchmark fixture trees.
 * These are NOT committed — they are generated at benchmark time.
 *
 * Usage: node tests/bench/generate.mjs [--size medium|large] [--out <dir>]
 *
 * medium: ~100 files (20 components, 20 styles, 60 supporting files)
 * large:  ~500 files (100 components, 100 styles, 300 supporting files)
 */
import { mkdir, writeFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const args = process.argv.slice(2);
function flag(name) { const i = args.indexOf(name); return i === -1 ? null : args[i + 1]; }

const size = flag('--size') || 'medium';
const outBase = flag('--out') || join(tmpdir(), 'considered-bench');

const SIZES = {
  medium: { components: 20, styles: 20, supporting: 60 },
  large: { components: 100, styles: 100, supporting: 300 }
};

if (!SIZES[size]) {
  console.error(`Unknown size "${size}". Use medium or large.`);
  process.exit(2);
}

const out = join(outBase, size);
await rm(out, { recursive: true, force: true });
await mkdir(join(out, 'src', 'components'), { recursive: true });
await mkdir(join(out, 'src', 'styles'), { recursive: true });
await mkdir(join(out, 'src', 'utils'), { recursive: true });

const { components, styles, supporting } = SIZES[size];

const componentTemplate = (i) => `
import React from 'react';
import './Component${i}.css';

export function Component${i}({ children, label }) {
  return (
    <div className="component-${i}" role="region" aria-label={label}>
      <h3>{label || 'Component ${i}'}</h3>
      <div className="content">{children}</div>
      <button onClick={() => {}} aria-label="Action ${i}">Act</button>
    </div>
  );
}
`;

const styleTemplate = (i) => `
.component-${i} {
  display: flex;
  flex-direction: column;
  padding: var(--spacing-md, 16px);
  border: 1px solid var(--border, #e5e7eb);
  border-radius: var(--radius-md, 8px);
  background: var(--surface, #fff);
  color: var(--text-primary, #1a1a1a);
}
.component-${i} .content { flex: 1; }
.component-${i} button {
  margin-top: 8px;
  padding: 6px 12px;
  font-size: 14px;
  cursor: pointer;
}
`;

const utilTemplate = (i) => `
// Utility module ${i}
export function util${i}(input) {
  if (!input) return null;
  return typeof input === 'string' ? input.trim() : String(input);
}
export const CONFIG_${i} = { enabled: true, priority: ${i} };
`;

const writes = [];
for (let i = 0; i < components; i++) {
  writes.push(writeFile(join(out, 'src', 'components', `Component${i}.tsx`), componentTemplate(i)));
}
for (let i = 0; i < styles; i++) {
  writes.push(writeFile(join(out, 'src', 'styles', `Component${i}.css`), styleTemplate(i)));
}
for (let i = 0; i < supporting; i++) {
  writes.push(writeFile(join(out, 'src', 'utils', `util${i}.ts`), utilTemplate(i)));
}
await Promise.all(writes);

const totalFiles = components + styles + supporting;
console.log(JSON.stringify({ size, out, totalFiles, components, styles, supporting }));
