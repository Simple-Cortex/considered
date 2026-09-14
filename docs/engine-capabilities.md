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
| `contract`, `gate`, `inventory`, `lint`, `validate`, `roll` | Compatible argument shapes select Rust after a `1.0.0` protocol probe; unavailable or incompatible binaries fall back to Node. Malformed legacy grammar stays on Node. `contract`, `gate`, `roll`, and `validate` resolve rule manifests, gate policy, and decks against the skill's own `assets/rules/`/`assets/decks/`, not the invocation cwd: the dispatcher pins this explicitly by passing `--root <skill-root>` to the native binary (a flag each of `considered-rs contract`/`gate`/`roll`/`validate` accepts alongside its own cwd-relative auto-detection for direct invocation). `contract --format compact`'s result cache is the one piece of this that stays cwd-relative on purpose: it is caller project state, so it is keyed under `<cwd>/.considered/cache`, never under the skill's own root. `inventory` and `lint` take an explicit path argument and were never root-relative. | `npm run test:compat-shadow`: 27 JSON/error cases (including a global `--json` ahead of `validate`, which is not forwarded to the native binary), 6 human-output cases, 7 native feature cases, ten-run determinism, and an installed-outside-cwd parity check (`validate`, `contract`, `gate`, `roll`) across both engines. |
| `validate skill`, `status`, `verify`, `context --experimental` | Rust-only. A missing companion returns exit 2 and the `manual-unavailable` engine state. `validate skill` and `context` also resolve skill-package data (the skill's own frontmatter/version files, or its `skills/` route manifest) rather than the invocation cwd, and accept the same `--root` pinning the dispatcher applies by default; `status` and `verify` are the opposite — they deliberately stay cwd-relative, reporting the *caller's* project `.considered/` surface state (an absent surface reports a normal "no surface state found" result, not an error, from any cwd). | Four direct/public command comparisons plus compatible, missing, incompatible, and failed-child envelope cases, plus an installed-outside-cwd check for `validate skill` (default root) and `context`. |
| `render`, `test`, `eval`, `check`, `install` | Node-only. | Frozen 34-case public compatibility suite and Node fixture tests. |

The dispatcher consumes the JSON authority rather than duplicating command
availability. It selects only validated argument shapes, never downloads a
binary, and keeps diagnostics on stderr. `engine-json` adds engine metadata;
legacy `--json` remains byte-compatible.

## Envelope error codes

An `engine-json` envelope carries an `error` field only when something went
wrong; a clean run, a findings exit (1), or a deck-exhaustion exit (3) never
has one, since those are normal domain outcomes, not engine failures. When
present, `error` always has this shape: `{ code, message, exitCode }`
(`native_child_failed` additionally carries `signal`). The codes:

| `error.code` | When it fires | Which engine(s) | Notes |
| --- | --- | --- | --- |
| `usage_error` | The child (native or Node fallback) exited 2 — the CLI's own documented usage/config error exit. | Both, identically | Engine-agnostic by design: a usage error means the same thing regardless of which engine produced it, so `code` and `exitCode` are always `usage_error`/`2` on both sides. `message` is each engine's own diagnostic text (native: the Rust binary's stderr; Node fallback: the script's own stderr) and is **not** required to match byte-for-byte between engines — the two runtimes report OS-level errors (e.g. a missing file) in their own native format. |
| `native_child_failed` | The native child exited with anything outside `0`/`1`/`2`/`3`, or was killed by a signal. | Native only | Signals the native binary crashed or behaved outside the documented exit-code contract. There is no Node-side equivalent (the Node fallback has no comparable "unexpected crash, not the child.exit's [0,1,2,3] protocol" state tracked here), so this code never appears under `node-fallback`. |
| `native_unavailable` | No compatible native binary was found or selected, and no Node fallback exists for this command (`validate skill`/`status`/`verify`/`context --experimental`, i.e. Rust-only commands). | Reported under engine `manual-unavailable` | Emitted before any child is spawned. |
| `child_start_failed` | The dispatcher could not even start the child process (e.g. `spawn` itself failed). | Either, whichever engine was selected | Distinct from `native_child_failed`: the process never ran at all. |

`fallback` mirrors why the engine was selected (`native_missing`,
`native_protocol_incompatible`, `native_probe_failed`, `native_not_selected`,
`manual_unavailable`, `native_child_failed`, or `null` when native ran
cleanly) and is documented alongside `engine` in
`assets/templates/engine-envelope.schema.json`.

Release benchmarking is available through `npm run bench:compare` after
building `considered-rs` in release mode. The runner alternates engine order,
uses identical small/medium/large trees, and rejects output drift before
reporting cold, median, and p95 timings.
