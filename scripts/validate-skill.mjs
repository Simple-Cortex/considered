#!/usr/bin/env node
/**
 * Transition wrapper for the Rust skill validator.
 *
 * Validation is deliberately not emulated here: a missing accelerator is a
 * clear, non-zero capability error rather than a weaker check reported as a
 * successful validation.
 */
import { spawn } from 'node:child_process';

const binary = process.env.CONSIDERED_RS_BIN || 'considered-rs';
const args = process.argv.slice(2);
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
