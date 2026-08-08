---
name: "considered-subtract"
description: "Use when a built or specified surface is crowded, when review findings point at competing or untraceable elements, or when the user asks to simplify, cut, or reduce an existing surface."
---

# Considered: subtract

Stage `subtract` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** the built or specified surface, the current contract, and `REVIEW.json` if one exists.

**Durable output:** a removal-only change to the target or specification, an updated `DELETED` list, and a stale-review marker.

**Read, in order:**

1. `../../references/workflow.md` §4 `subtract`.
2. `FRAME.md`, `STRUCTURE.md`, `CONTRACT.md`, the target, and `REVIEW.json` when present.

Removal only: no new components, copy, controls, or visual treatment. Deletions are recorded, never silent (`LOOP-04`). Return to `critique` when a removal changes hierarchy or action scope.

**Block only when** no artifact, specification, or candidate element exists to remove.
