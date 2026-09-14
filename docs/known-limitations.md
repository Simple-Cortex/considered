# Known limitations

Maintained evidence register. For every claimed host, model family, engine, and review mode, this records what was tested, exact evidence or `unmeasured`, user impact, fallback, and current status.

Last updated: 2026-09-13

"Observed" means a recorded, informal internal evaluation record exists (workdir, prompt hash, tool calls, CLI calls, artifacts). It is not "supported": these are single builds recorded informally, not the published `m0-*` evaluations.

## Host support

| Host | Tested | Evidence | Status |
| --- | --- | --- | --- |
| Claude Code (Anthropic) | observed — 4 builds, 2026-09-13 | internal evaluation record, 2026-09-13. All four builds read `SKILL.md` and produced the full artifact chain; all four ran `scripts/roll.mjs` and the linters directly and none executed `bin/considered.mjs` utilities (one opened its `--help`). Three of four self-reviewed; one spawned two fresh reviewers. | observed on one host — the skill's workflow is followed; the dispatcher is not on the builder's path (skill text names `scripts/`) |
| Qwen Code | observed — 2 headless builds of `read-docs-home`, 2026-09-13 | internal evaluation record, 2026-09-13. Skill discovered by name from `~/.qwen/skills`. Both runs read `SKILL.md`, `workflow.md` and 6–7 reference files, ran `scripts/roll.mjs` and both linters, wrote PRODUCT/FRAME/STRUCTURE/CONTRACT; one run also wrote REVIEW-PACKET + REVIEW.json, the other skipped critique (run variance, not engine). Neither executed `bin/considered.mjs`. `CONTRACT.md` sidecar landed under `.considered/<surface>/` (same as Claude Code builds). | observed on a second host — workflow followed end to end; friction noted was cosmetic/doc-level, none blocking |
| Codex (OpenAI) | unmeasured | No formal evaluation run | unmeasured — implicit invocation enabled |
| Cursor | unmeasured | No formal evaluation run | unmeasured |
| Generic LLM with file access | unmeasured | No formal evaluation run | unmeasured — manual fallback path available |

## Model families

| Model family | Tested | Evidence | Status |
| --- | --- | --- | --- |
| Claude 3.5/4 Sonnet | unmeasured | No recorded operational trace records the required model revision, prompt hash, and tool calls | unmeasured for operational behavior |
| Claude 3.5/4 Opus | unmeasured | No formal evaluation run | unmeasured |
| GPT-4o / o-series | unmeasured | No formal evaluation run | unmeasured |
| Gemini 2.x | unmeasured | No formal evaluation run | unmeasured |

## Engine

| Engine | Tested | Evidence | Status |
| --- | --- | --- | --- |
| Node.js (scripts/) | yes | 58 fixture tests and the frozen 34-case public compatibility suite pass | supported — authoritative fallback |
| Rust (considered-rs) | yes | 143 Rust tests; shadow suite covers 27 JSON/error cases, 6 human streams, 7 native feature shapes, 4 lifecycle commands, cold file effects, ten-run determinism, and (2026-09-13) an installed-skill outside-cwd parity block for validate/contract/gate/roll/context/validate-skill | supported for direct use and validated public delegation shapes |
| Rust — engine used inside builds (binary placed on PATH for real builds) | observed — builds on two hosts with the binary present and absent | internal evaluation record, 2026-09-13. No builder invoked `bin/considered.mjs` directly, so the engine never ran inside a build; every script the builders used (`scripts/roll.mjs`, `lint-source.mjs`, `lint-contract.mjs`) is Node-only and the skill text names only `scripts/`. A separate check confirmed native `validate`/`contract`/`gate`/`roll`/`context`/`validate-skill` now resolve the project root correctly from any installed-skill workdir (previously failed with exit 2, "Cannot find project root"). | observed: engine correct after a fix, but unexercised by builders — a doc/routing change is needed before the engine can affect a build |
| WASM | unmeasured | Not yet implemented | not available |

## Review modes

| Mode | Tested | Evidence | Status |
| --- | --- | --- | --- |
| Independent review (REVIEW.json) | yes | The published evaluation records (m0-001, m0-002) | supported |
| Model-reviewed | yes | Evaluation harness includes a model-review pipeline | supported — not equivalent to human review |
| Production | unmeasured | No production deployment evaluated | unmeasured |

## Feature limitations

### Contract validation
- Validates internal consistency only. Visual rank, responsive behavior, and state behavior are verified through REVIEW-PACKET.md and the independent reviewer, never inferred from a text contract.

### Source lint
- All findings are heuristic leads for a reviewer, not proof. A clean scan is never a design or accessibility pass.
- Regex-based checks may produce false positives on unconventional code patterns.
- Does not analyze Tailwind AST, framework-specific component APIs, or rendered output.
- Instance-data leads are pattern-based (emails, card fragments, invoice/period labels, recency phrases, name+email pairs, current-value settings) and cannot know what the brief actually supplied — they flag text that reads as a specific real-world instance, not a confirmed violation.

### Gate
- Gate thresholds are calibrated against one evaluation (the published m0-001 evaluation record). Provisional — recalibration needed with more data.
- The gate does not evaluate design quality, only structural compliance and review completeness.

### Roll
- Assignment is deterministic from a reproduction key. Different keys produce different assignments.
- The deck is finite. Exhaustion (all generations used) requires starting a fresh key.

### Inventory
- Detection is regex-based. It may miss tokens defined in non-standard formats or components using unconventional export patterns.

### Performance
- On the measured macOS ARM64 development machine, the release binary was about 3x faster than Node for medium/large lint scans and 6–14x faster for inventory while producing byte-identical output. Evidence: `tests/bench/comparison.json`. Run `cargo build --release -p considered-cli && npm run bench:compare` on a target platform before making platform-specific latency claims.
- Small-scan p95 is sensitive to cold process startup and OS scheduling; the benchmark records cold and warm measurements separately.

### Compatibility
- The Node.js implementation is the authoritative fallback. The dispatcher delegates only validated Rust argument shapes; malformed legacy grammar remains on Node to preserve exact diagnostics. See `docs/engine-capabilities.md`.
- Hosts without the Rust binary degrade to Node for portable commands. Rust-only lifecycle commands report a machine-readable `manual-unavailable` state instead of pretending a fallback exists.

## What is NOT claimed

- Considered does not prove semantic accessibility compliance.
- Considered does not prove rendered visual hierarchy correctness.
- Considered does not prove design quality.
- Considered does not replace independent human review for public effectiveness claims.
- Model panels are acceptable for iteration; independent human review is required before broad public claims.
