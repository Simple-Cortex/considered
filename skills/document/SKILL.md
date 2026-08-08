---
name: "considered-document"
description: "Use when a reviewed surface has to be handed to another person, agent, or session, or when the user asks for the design system, spec, or record of what was built and why."
---

# Considered: document

Stage `document` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** the current durable chain plus the target or specification.

**Durable output:** `.considered/<surface-id>/DESIGN.md` — decision, route, P0 to P4, zones, actions, components and tokens, states, responsive behavior, contract location, assumptions, and the current review gate.

**Read, in order:**

1. `../../references/workflow.md` §4 `document`.
2. `PRODUCT.md`, `FRAME.md`, `STRUCTURE.md`, `CONTRACT.md`, the latest `REVIEW.json`, and the final artifact or specification.

Link workspace-relative paths only; the record must be reusable without chat history or a host-specific feature. Separate verified facts from assumptions and state review limitations.

**Block only when** no target or specification exists to document.
