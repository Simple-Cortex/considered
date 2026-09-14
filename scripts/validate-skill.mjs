#!/usr/bin/env node
/**
 * Transition wrapper for the Rust skill validator.
 *
 * Validation is deliberately not emulated here: a missing accelerator is a
 * clear, non-zero capability error rather than a weaker check reported as a
 * successful validation.
 */
import { spawn } from 'node:child_process';

const args = process.argv.slice(2);
if (args.includes('--help') || args.includes('-h')) {
  console.log('Usage: node validate-skill.mjs [--json] [--root <path>]\n\nValidates the packaged skill for runtime drift and correctness via the\nnative companion binary (there is no Node fallback: a missing accelerator\nis reported as a capability error, not a weaker check reported as a pass).');
  process.exit(0);
}
const binary = process.env.CONSIDERED_RS_BIN || 'considered-rs';
const child = spawn(binary, ['validate-skill', ...args], { stdio: 'inherit' });
child.once('error', error => {
  if (error.code === 'ENOENT') {
    console.error('considered validate skill: native validator is unavailable. Install the explicit companion binary or use cargo run -p considered-cli -- validate-skill.');
    process.exit(2);
  }
  console.error(`considered validate skill: failed to start native validator: ${error.message}`);
  process.exit(2);
});
child.once('exit', (code, signal) => {
  if (signal) {
    console.error(`considered validate skill: native validator terminated by signal ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
