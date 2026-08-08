---
name: "considered-structure"
description: "Use when a surface has a ready `FRAME.md` and no committed zone, tier, action, and contract record yet — including before writing any code for that surface, and when a structural change invalidates the existing `STRUCTURE.md`."
---

# Considered: structure

Stage `structure` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** ready `FRAME.md`, product context, target or specification delivery.

**Durable output:** `.considered/<surface-id>/STRUCTURE.md` and the raw canonical `CONTRACT.md`, including the assigned hand and its reproduction record.

**Read, in order:**

1. `../../references/workflow.md` §4 `structure`, plus §5 for the roll fallback.
2. `../../references/ia.md`, `../../references/hierarchy.md`, `../../references/actions.md`.
3. `../../assets/templates/STRUCTURE.md`, `../../assets/templates/contract-block.md`.

**Assignment, not taste:** run `node <skill-dir>/scripts/roll.mjs --mode <mode>` when available and record ids, key, and generation (`ROLL-01`). Never shortlist or reselect.

**Block only when** an unresolved FRAME blocker affects decision, mode, P0, action risk, or target ownership.
