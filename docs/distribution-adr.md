# Distribution ADR

**Status:** accepted
**Date:** 2026-09-10
**Deciders:** Project maintainer

## Context

Considered needs a distribution strategy for the Rust accelerator engine alongside the existing Node.js skill implementation. Three options were considered.

## Options considered

### Option A: Separately installed native companion binary (recommended)

The Rust binary (`considered-rs`) is installed separately via `cargo install` or pre-built release archives. The Node.js skill detects the binary at runtime and delegates deterministic commands to it. If the binary is missing or incompatible, the skill falls back to the Node.js implementation transparently.

**Pros:**
- Clean separation of concerns — skill packaging vs native binary distribution
- Binary can be signed, versioned, and updated independently
- Fallback is maintained — no broken workflow if binary is missing
- Installation remains explicit and inspectable (never auto-downloads)

**Cons:**
- Two installation steps for users who want the accelerator
- Version compatibility must be checked at runtime

### Option B: Portable Rust/WASM module invoked by Node

Compile the Rust engine to WASM and invoke it from Node.js via a binding.

**Pros:**
- Single installation step
- No separate binary management

**Cons:**
- WASM performance may not match native for filesystem-heavy operations
- Adds complexity to the build pipeline
- WASM ecosystem for filesystem access is less mature
- Not yet implemented — would require significant additional work

### Option C: Bundled platform binaries

Ship pre-built binaries for each platform (macOS ARM/x64, Linux ARM/x64, Windows x64) inside the npm package.

**Pros:**
- Single installation step
- Binary is immediately available

**Cons:**
- Large package size (multiple platform binaries)
- Platform-specific testing burden
- Code signing complexity per platform
- npm package bloat affects all users, even those who don't want the accelerator

## Decision

**Option A** is selected. The Rust accelerator is a separately installed, signed native companion binary with a maintained Node.js fallback.

### Rationale

1. The skill's portable fallback must remain functional. Bundling binaries into the npm package would complicate the skill installation and create platform-specific failure modes.
2. WASM is not yet mature enough for the filesystem-heavy operations the engine performs. Revisit when the WASM filesystem API stabilizes.
3. Separate installation keeps the skill package lean and the binary distribution explicit. Users who want the accelerator install it deliberately; users who don't are unaffected.
4. The Node.js implementation is maintained through at least one compatibility release after native cutover. A command is delegated only after the shadow suite proves its frozen Node stream is byte-compatible; the capability matrix records non-delegated ports and their migration work.

### Platform support

| Platform | Architecture | Status |
| --- | --- | --- |
| macOS | ARM64 | supported (development environment) |
| macOS | x64 | unmeasured — no CI runner |
| Linux | ARM64 | unmeasured — no CI runner |
| Linux | x64 | unmeasured — CI available but not configured |
| Windows | x64 | unsupported — no development or CI environment |

### Engine preference

The CLI reports the selected engine in structured output. Preference order:
1. Native Rust binary (`considered-rs`) — if installed and protocol version is compatible
2. Node.js fallback (`scripts/*.mjs`) — always available

A missing or incompatible binary produces a clear fallback message, not a broken workflow.

### OpenAI metadata decision

**Decision:** `agents/openai.yaml` is deliberately not created.

**Rationale:**
- The skill already works via implicit invocation through SKILL.md
- The root SKILL.md description is clear and sufficient for agent discovery
- A separate metadata file would create a second source of truth that could drift from SKILL.md
- Implicit invocation remains enabled — no policy change
- If Codex-specific presentation metadata becomes necessary based on user feedback, create the file at that point with validation against the root skill name, description, and runtime manifest

### Binary installation

Installation of the Rust binary is always explicit and inspectable:
- `cargo install --path crates/considered-cli` from the repository, or
- Pre-built archives from GitHub Releases

The skill never fetches or executes a binary during ordinary skill execution. Installation remains a separate, user-initiated step.
