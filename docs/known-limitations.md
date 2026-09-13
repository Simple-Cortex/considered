# Known limitations

Maintained evidence register. For every claimed host, model family, engine, and review mode, this records what was tested, exact evidence or `unmeasured`, user impact, fallback, and current status.

Last updated: 2026-09-13

## Host support

| Host | Tested | Evidence | Status |
| --- | --- | --- | --- |
| Claude Code (Anthropic) | unmeasured | Used during development, but no recorded host-routing/workflow trace meets the evaluation schema | unmeasured |
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
| Rust (considered-rs) | yes | 143 Rust tests; shadow suite covers 23 JSON/error cases, 6 human streams, 7 native feature shapes, 4 lifecycle commands, cold file effects, and ten-run determinism | supported for direct use and validated public delegation shapes |
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
