---
name: "considered-init"
description: "Use when starting Considered work in a workspace that has no `.considered/PRODUCT.md` yet, or when the recorded product context is stale, contested, or missing facts a later surface depends on."
---

# Considered: init

Stage `init` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** brief or product request, workspace root, optional source target.

**Durable output:** `.considered/PRODUCT.md` (evidence, constraints, assumptions, blockers).

**Read, in order:**

1. `../../references/workflow.md` §1 (paths), §2 (question and assumption protocol), §4 `init`, §5 (all-host fallback).
2. `../../assets/templates/PRODUCT.md`.
3. Any existing `.considered/PRODUCT.md` and design-system evidence before adding claims.

**Interview trigger:** when available evidence would leave `PRODUCT.md` substantially empty, ask one numbered batch of objective questions first — see §4 `init` step 4.

**Block only when** no writable artifact root exists, or the product and delivery cannot be identified even provisionally. Otherwise record `A#` assumptions and continue.

Next stage: `frame` for a named surface.
