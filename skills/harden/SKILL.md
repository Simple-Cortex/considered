---
name: "considered-harden"
description: "Use when a surface exists but its loading, empty, partial, stale, error, permission, and extreme-data behavior is unspecified, or when review findings or the user raise edge cases, recovery, or destructive-action safety."
---

# Considered: harden

Stage `harden` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** the built or specified surface plus its state and action inventory.

**Durable output:** state and edge-case behavior in the target or specification, an updated state plan, and a stale-review marker.

**Read, in order:**

1. `../../references/workflow.md` §4 `harden`.
2. `../../references/states.md`, `../../references/actions.md`, and `../../references/forms.md` for flows.
3. `FRAME.md`, `STRUCTURE.md`, and the target.

Friction is proportional to consequence; when risk is unknown but inferable, record the assumption and implement the conservative reversible path. Route to a fresh `critique` afterwards.

**Block only when** unknown risk, a regulated requirement, or a safety treatment would make recovery behavior unsafe to infer.
