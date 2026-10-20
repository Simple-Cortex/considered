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
3. Reviewer side only: `../../references/critique.md`, `../../references/accessibility.md`, and `../../references/design-system-evidence.md` only when component choice or library fit is relevant to the review. Omit it for audits where component fit does not affect diagnosis or recommendation.

The reviewer context receives the packet, artifact, and reviewer references — never the build rationale. Missing render evidence marks visual, responsive, and render accessibility checks `blocked`; an unverified gate is never a pass.

For component-related findings, use the frozen, selected library facts in `REVIEW-PACKET.md` to assess fit independently against the surface need. The packet must include resolved package/version, relevant component facts, project overrides, provenance, fingerprints, and limitations; it must snapshot facts inline or use immutable content-addressed copies, never rely on a mutable cache pointer alone. Library evidence informs but does not dictate the recommendation. Do not load the full cache or seek builder rationale. Cite source and version for consequential claims, and mark unavailable or inconclusive evidence as a limitation rather than a defect.

**Block only when** no inspectable artifact or source exists.
