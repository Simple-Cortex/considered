---
name: "considered"
description: "Use when consequential UI work is being designed, changed, or audited — a page, flow, dashboard, component, or whole surface — and the outcome changes what a person can decide or do: a decision, task, information structure, or visual priority. Also use when a design brief is thin, vague, or unscored and needs questioning before any build. Skip isolated copy, spacing, or token fixes with no structural impact."
---

# Considered

Work from decision to structure to composition, then review the result independently. Do not design from a component inventory.

## Start and route

1. Resolve the workspace root and the surface id. Use the host-neutral paths in `references/workflow.md`. `<skill-dir>` is the directory that contains this `SKILL.md`; every path below is relative to it or to the workspace root.
2. Select one mode: `persuade`, `operate`, `analyze`, `read`, or `experience`.
3. Select the surface route: `page`, `flow`, `component`, or `existing`.
4. Read only the command route plus its mode and surface branches. Do not load rule catalogs while building.

Use `/considered <command>` when slash commands exist. Otherwise treat the command word as an explicit instruction and follow the same files. Hosts that discover the per-command subskills in `skills/<command>/` may invoke one directly; each is equivalent to its command word here and routes to the same normative files.

## Commands

| Command | Required outcome | Read next |
| --- | --- | --- |
| `init` | Capture reusable product context | `references/workflow.md`, `assets/templates/PRODUCT.md` |
| `frame` | State the decision, object model, ranked questions, assumptions | `references/workflow.md`, `assets/templates/FRAME.md` |
| `structure` | Record the assigned hand, zones, hierarchy, actions, contract | `references/workflow.md`, `assets/templates/STRUCTURE.md`, `assets/templates/contract-block.md` |
| `compose` | Build or specify the routed surface | `references/workflow.md` plus only the applicable mode and surface references |
| `critique` | Run an adversarial review in a fresh context | `references/workflow.md`, `assets/templates/REVIEW-PACKET.md`, `references/critique.md` |
| `subtract` | Make a removal-only pass | `references/workflow.md` |
| `harden` | Add real states, extremes, and recovery behavior | `references/workflow.md`, `references/states.md` |
| `document` | Emit the portable system record | `references/workflow.md` |

The complete input, reads, outputs, and blocker protocol for every command is normative in `references/workflow.md`.

## Artifact contract

All durable artifacts are workspace-relative. The canonical paths are:

```text
.considered/PRODUCT.md
.considered/<surface-id>/FRAME.md
.considered/<surface-id>/STRUCTURE.md
.considered/<surface-id>/CONTRACT.md
.considered/<surface-id>/REVIEW-PACKET.md
.considered/<surface-id>/REVIEW.json
.considered/<surface-id>/DESIGN.md
```

Put the contract in a comment wrapper at the top of the built artifact when that is safe. Always retain the identical raw contract at `.considered/<surface-id>/CONTRACT.md`; it is the sidecar fallback for generated, binary, remote, or comment-hostile targets. See `assets/templates/contract-block.md`.

## Ask narrowly, assume explicitly

Ask questions only for blocking facts. A fact blocks only when a wrong assumption would make the primary decision, mode, safety treatment, legal or accessibility requirement, or requested delivery target materially wrong. Record every other gap as a labeled assumption with its impact and validation path in `FRAME.md`. Do not ask generic discovery questions before committing to a direction.

The one structured exception is the brief gate: `frame` scores the brief on the eight-dimension rubric in `assets/templates/FRAME.md`, and below 8 of 16 the deliverable is the collaborative question loop in `references/workflow.md` §4 — ranked questions in batches of at most three, re-scored until the brief is designable or the user explicitly accepts the risk. That loop is not discovery; it is the work.

## Progressive reads

| Need | Read |
| --- | --- |
| Mode unclear or collision | `references/modes.md` |
| Persuade | `references/persuade.md` |
| Operate/analyze dashboard, table, or data workspace | `references/dense.md` |
| Operate flow, form, or settings task | `references/forms.md` |
| Read | `references/read.md` |
| Experience | `references/experience.md` |
| Grouping or navigation | `references/ia.md` |
| Tiers or focal point | `references/hierarchy.md` |
| Actions | `references/actions.md` |
| Tokens, components, or visual finish | `references/craft.md` |
| Any interactive or rendered surface | `references/accessibility.md` |
| States or resilience | `references/states.md` |
| Review only | `references/critique.md` |

## Non-negotiables

- One surface has one mode, one P0, and one primary action per scope.
- Every element traces to a ranked question. P4 is chrome and does not compete for focus.
- No design below a brief score of 8 of 16 without the user's recorded acceptance in `FRAME.md`.
- `structure` uses the assigned hand from `node <skill-dir>/bin/considered.mjs roll` when available. Do not choose from a shortlist.
- The accessibility gates in `references/accessibility.md` are gates, not scores. An unverified gate is `blocked`, never `pass`.
- `critique` receives a fresh reviewer packet, not the build reasoning.
- Guidance rules surface by reading the routed catalog at its stage; no command emits them, and none of them has an executable checker.
- A host without scripts, structured questions, subagents, screenshots, or editable source uses the fallback in `references/workflow.md` and labels the resulting limitation.

## Red flags — stop and reread the route

| Rationalization | Reality |
| --- | --- |
| "The structure is obvious; I'll build now and backfill the contract" | `LOOP-01` exists because obvious structures are the ones that ship wrong. No compose before `CONTRACT.md` is on disk. |
| "The roll gave a weak hand; I'll pick a better direction" | Taste-picking is the failure the roll removes. Keep the hand or reroll chained from the same key — never select. |
| "The build is clean; a fresh review would just confirm it" | A build cannot certify itself. Confirmation is the reviewer's to give, from the frozen packet alone. |
| "I couldn't verify that gate, but it surely passes" | An unverified gate is `blocked`, never `pass`. Claiming it is inventing evidence. |
| "The brief is thin, but I can infer the rest" | Below 8 of 16 the deliverable is questions, not design. Run the brief gate loop. |
| "This change is too small for the chain" | Small structural changes move decisions. If it truly moves no decision, task, structure, or priority, this skill does not apply at all — use neither the chain nor a shortcut version of it. |

Symptoms that one of these is happening: target code is being written with no `CONTRACT.md` on disk; the hand is being described as "unsuitable" without a chained reroll; "review passed" is being written in the same context that built the artifact; a `REVIEW.json` outcome nobody read; a surface being designed while `FRAME.md` has no brief score.
