---
name: "considered-compose"
description: "Use when a surface has a committed `STRUCTURE.md` and `CONTRACT.md` and the target source or portable specification still has to be built or updated to match them."
---

# Considered: compose

Stage `compose` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** ready structure and contract; a writable target for code delivery.

**Durable output:** the changed target or portable specification, the contract wrapper where safe, and an updated structural state plan.

**Read, in order:**

1. `../../references/workflow.md` §4 `compose`.
2. The chain: `PRODUCT.md`, `FRAME.md`, `STRUCTURE.md`, `CONTRACT.md`, target source.
3. `../../assets/principles-index.json`, read once and filtered to the current mode (`modes` contains it or `*`).
4. Only the routed mode catalog — `../../references/persuade.md`, `dense.md`, `forms.md`, `read.md`, or `experience.md` — plus `../../references/craft.md`, `../../references/accessibility.md`, and `../../references/states.md` when triggered.

When selecting or implementing an installed library component, also use `../../references/design-system-evidence.md`: inspect the small cache index, then verify only the relevant component detail, resolved version, supported API, intended use, and constraints before implementation. Do not load the whole component cache. If no library component is being selected or implemented, this reference is not required.

Do not load reviewer catalogs while building. Checks run as evidence, not as mid-build design instructions.

**Block only when** requested code delivery has no identifiable writable target and specification-only delivery is not accepted.
