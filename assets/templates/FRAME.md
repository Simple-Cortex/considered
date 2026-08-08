# Frame

**Surface id:** `<kebab-case id>`
**Route:** `frame x <mode> x <page|flow|component|existing>`
**Delivery:** coded surface | specification | review-only
**Target:** `<workspace-relative target or none>`
**Status:** ready | blocked

## Brief score

Score the brief before designing. Each dimension is 0, 1, or 2; the total is out of 16. Below 8, run the brief gate loop in `references/workflow.md` §4 and re-score. Record `brief-score: <n>/16 (user-accepted)` here if the user chose to proceed below 8, and carry each 0-scored dimension as an assumption.

| # | Dimension | 0 | 1 | 2 | Score |
| --- | --- | --- | --- | --- | --- |
| PS1 | Human named | "users" | a role | a role with expertise level and context of use |  |
| PS2 | Decision named | none | implied | explicit decision with a time budget |  |
| PS3 | Consequence named | none | vague | specific cost of being wrong |  |
| PS4 | Frequency named | none | approximate | specific cadence and trigger |  |
| PS5 | Success measurable | none | subjective | observable behavior change or metric |  |
| PS6 | Scope bounded | open ended | loosely bounded | explicit in and out of scope |  |
| PS7 | Constraints known | none | some | data, platform, performance, compliance, brand |  |
| PS8 | Current state known | none | anecdotal | existing behavior, workaround, or prior art documented |  |

**Total:** `<n>` / 16
**Gate:** proceed | proceed with recorded risk | questions returned, not designing yet

## Decision

> `<role>` decides `<what>` within `<time budget>`, using `<inputs>`;
> being wrong costs `<consequence>`.

**Human goal:** believe | complete | understand | learn | feel
**Observable finish:**
**Current workaround or alternative artifact considered:**

## Mode and surface choice

**Mode:** persuade | operate | analyze | read | experience
**Mode evidence:**
**Surface route:** page | flow | component | existing
**Why this is one surface:**
**Mode collision rejected or split:**
**Dashboard type:** operational | analytical | strategic | embedded | not a dashboard

Required when the mode is operate or analyze and the surface is a dashboard or data workspace. The type sets density, refresh, interaction, and the module budget checked by `DASH-03`. Never mix two types in one view; build two surfaces and link them.

## Question inventory

Write questions in the user's voice. Rank first, then decide what belongs by default. P0 to P4 describe visual and interaction priority; P4 is chrome and has no claim on the focal position.

| ID | Tier | Question | Decision it drives | Frequency | Default or disclosed |
| --- | --- | --- | --- | --- | --- |
| Q1 | P0 |  |  |  | default |
| Q2 | P1 |  |  |  | default |
| Q3 | P1 |  |  |  | default |
| Q4 | P2 |  |  |  | disclosed |

## Object model

| Object | Identity | Ranked attributes | States | Relationships | Actions |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |

## Candidate elements and deletions

| Element | Answers question | Keep, disclose, or delete | Reason |
| --- | --- | --- | --- |
|  |  |  |  |

## Assumptions

Only nonblocking gaps go here. Label the impact and validation path.

| ID | Assumption | Impact | Validation path |
| --- | --- | --- | --- |
| A1 |  |  |  |

## Blocking facts

Ask only these questions. If none remain, write `None` and continue.

| ID | Missing fact | Why a wrong assumption is material | Exact question |
| --- | --- | --- | --- |
| B1 |  |  |  |

## Frame exit

- [ ] The brief is scored, and the total is 8 or above or the acceptance is recorded.
- [ ] One decision and one mode are declared.
- [ ] Dashboard type is declared, or the surface is not a dashboard.
- [ ] Q1 has a decision and a P0 candidate.
- [ ] Every kept element has a question tag.
- [ ] Mode collision is split or rejected.
- [ ] All nonblocking gaps are labeled assumptions.
- [ ] No unresolved blocking fact affects the next command.
