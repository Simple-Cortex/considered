# Engine capability matrix

`assets/templates/engine-capabilities.json` is the single, versioned authority
for engine capabilities. The dispatcher reads it at runtime and the shadow
suite validates it. It deliberately distinguishes a Rust implementation from a
safe public native delegation: `conditional-exact` commands are delegated only
for their tested argument grammar; malformed and advanced forms stay on Node.
`versioned-exception` entries retain the portable Node implementation until
their output contracts have a documented migration.

Use `considered --format engine-json <command> ...` to receive the versioned
engine envelope. The legacy `--json` stream is intentionally unchanged.

| Command | Public engine behavior | Evidence |
| --- | --- | --- |
| `contract`, `gate`, `inventory`, `lint`, `validate`, `roll` | Compatible argument shapes select Rust after a `1.0.0` protocol probe; unavailable or incompatible binaries fall back to Node. Malformed legacy grammar stays on Node. | `npm run test:compat-shadow`: 23 JSON/error cases, 6 human-output cases, and 7 native feature cases, including cold file effects and ten-run determinism. |
| `validate skill`, `status`, `verify`, `context --experimental` | Rust-only. A missing companion returns exit 2 and the `manual-unavailable` engine state. | Four direct/public command comparisons plus compatible, missing, incompatible, and failed-child envelope cases. |
| `render`, `test`, `eval`, `check`, `install` | Node-only. | Frozen 34-case public compatibility suite and Node fixture tests. |

The dispatcher consumes the JSON authority rather than duplicating command
availability. It selects only validated argument shapes, never downloads a
binary, and keeps diagnostics on stderr. `engine-json` adds engine metadata;
legacy `--json` remains byte-compatible.

Release benchmarking is available through `npm run bench:compare` after
building `considered-rs` in release mode. The runner alternates engine order,
uses identical small/medium/large trees, and rejects output drift before
reporting cold, median, and p95 timings.
