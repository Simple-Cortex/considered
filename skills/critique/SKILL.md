---
name: "considered-critique"
description: "Use when a built or specified surface needs independent review before it can be called done, when a prior review is marked stale, or when the user asks for a design review, audit, or ship decision on an existing artifact."
---

# Considered: critique

Stage `critique` of the Considered chain. The root skill (`../../SKILL.md`) routes; this file only names the contract.

**Required input:** an inspectable artifact plus the current durable chain.

**Durable output:** frozen `REVIEW-PACKET.md`, independent `REVIEW.json`, the gate result, a fix list, and stated limitations.

**Read, in order:**

1. `../../references/workflow.md` §4 `critique` and §5.
2. `../../assets/templates/REVIEW-PACKET.md`, `../../assets/templates/REVIEW.json`.
3. Reviewer side only: `../../references/critique.md` and `../../references/accessibility.md`.

The reviewer context receives the packet, artifact, and reviewer references — never the build rationale. Missing render evidence marks visual, responsive, and render accessibility checks `blocked`; an unverified gate is never a pass.

**Block only when** no inspectable artifact or source exists.
