> **REVIEWER-ONLY FILE.** This file is read by the reviewer, never by the builder. The reviewer must run in a fresh context that did not see the build reasoning. A thread grading its own work rubber-stamps it: reload this rule catalog with no reference to the design rationale that produced the artifact under review.

Purpose: run the critique protocol, score the rubric, apply the ship/revise/redesign gate, and output findings, without any self-critique conflict of interest.

Contents: 8-pass protocol, severity model, scoring rubric (140 max), gate thresholds, self-critique rules (`EVAL-13` to `EVAL-16`), findings output format.

---

## The 8-pass protocol

Run passes in order. Stop and return early if a pass produces an `S1` finding that invalidates later passes. Purpose and structure failures make hierarchy/action/craft findings moot, so never critique styling before purpose and structure pass (`EVAL-01`).

### Pass 1: Purpose
- Does `FRAME.md` name a human, decision, consequence, and frequency?
- Does every ranked question in `FRAME.md` drive a decision?
- Does the view answer the top-ranked questions completely rather than all questions partially? (`IA-17`)
- Can you state, in one sentence, what the user should do after reading each zone? (`DASH-07`)

*If this pass fails, stop. No further critique is useful until purpose is fixed.*

### Pass 2: Structure
- Does every element trace to a question? (`IA-01`)
- Is grouping by question and object, not type or source? (`IA-02`, `IA-03`)
- Do zone headings pass the heading test? (`IA-09`)
- Are groups 3 to 7 elements? (`IA-06`)
- Is any element duplicated across zones? (`IA-07`)
- Is navigation depth 3 or fewer, 7 or fewer items per group? (`IA-12`, `IA-13`)
- Is default vs disclosed split correct for question frequencies? (`IA-17`, `IA-18`)

### Pass 3: Hierarchy
- Squint test: exactly one dominant region? (`HIER-01`)
- Inversion check: visual rank matches logical rank in the top three? (`HIER-02`)
- Exactly one P0, and 3 to 5 P1? (`HIER-05`, `HIER-06`)
- Encoding budget respected, only one element above 4 points? (`HIER-04`, `HIER-04a`)
- Accent color used at most twice? (`HIER-04b`)
- Proximity ratio satisfied in every zone? (`HIER-07`)
- Grayscale test passes? (`HIER-11`)

### Pass 4: Action
- Every action has a scope and a tier? (`ACT-02`)
- At most one primary per scope, at most two filled per viewport? (`ACT-01`)
- Emphasis decays monotonically as scope narrows? (`ACT-03`)
- Surplus actions in overflow? (`ACT-04`)
- All labels verb plus object, three words or fewer? (`ACT-05`)
- No destructive action as default primary? (`ACT-08`)
- Destructive actions separated from constructive ones? (`ACT-15`)
- All states defined, no silent disabling? (`ACT-16`, `ACT-17`)

### Pass 5: Content
- User vocabulary throughout, no schema leaks? (`CONT-01`)
- Every metric defined once and reachable? (`CONT-02`)
- Terminology consistent? (`CONT-03`)
- Units stated, precision appropriate to the decision? (`CONT-04`)
- Time window and freshness stated? (`CONT-05`)
- Every value carries context? (`DASH-05`)
- Significance thresholds prevent signaling noise? (`DASH-08`)
- Favorability, not direction, drives color? (`DASH-09`)
- Every material claim — policy, capability, commitment, prior condition, definition, quantity, duration, data fact — traces to a supplied fact? (`HON-01`, `HON-02`)
- Sample records stay inside the supplied schema and figures? (`HON-03`)
- Success and confirmation copy claims only what the mechanism verifiably does? (`HON-04`)
- Assistive text asserts nothing beyond the visible copy? (`HON-05`)
- No design rationale, restated builder constraint, or completeness meta-claim rendered as copy? (`HON-06`)

### Pass 6: States and edges
- Loading, empty first-use, empty filtered, partial, stale, error, no permission, too much data all specified? (`STATE-01`)
- Extremes verified: zero, one, many, huge, negative, null, long strings? (`STATE-04`)
- Freshness budget declared and designed against? (`STATE-05`)

### Pass 7: Accessibility and responsiveness
- All `A11Y-01` through `A11Y-12` gates pass (`references/accessibility.md` states each gate and its check; every failed gate is S1).
- Priority order holds across breakpoints? (`COMP-09`)
- Charts have accessible equivalents? (`A11Y-10`)

### Pass 8: Craft
- Restraint budget respected on all nine counts? (`COMP-16`)
- Alignment edges minimized, numbers right-aligned? (`COMP-17`)
- Spacing tokenized, rhythm limited to 3 values per zone? (`COMP-03`, `COMP-18`)
- Elevation and radius used semantically and consistently? (`COMP-10`, `COMP-11`)
- Subtraction pass performed? (`EVAL-06`)

---

## Severity model

Every finding carries a severity (`EVAL-02`). Sort the fix list by severity, then by effort ascending within severity.

| Severity | Name | Definition | Gate effect |
| --- | --- | --- | --- |
| **S1** | Blocker | User cannot make the decision, likely to make a wrong one, an accessibility gate fails, or a destructive action is unsafely presented | Cannot ship. Any S1 fails the gate |
| **S2** | Major | Decision possible but materially slower or error-prone: hierarchy conflicts, structural mis-grouping, missing context on key values, missing critical states | Max 2 permitted at ship |
| **S3** | Minor | Adds friction or inconsistency without threatening the decision | Max 6 permitted |
| **S4** | Polish | Craft refinement, no functional impact | Not gating |

Example findings by severity:

| Finding | Severity |
| --- | --- |
| Delete styled as filled primary in a page header | S1 |
| Focus indicator removed | S1 |
| Status conveyed by color alone | S1 |
| No element answers the top-ranked question | S1 |
| Squint test returns uniform field | S2 |
| Metrics grouped by chart type | S2 |
| P0 metric has no comparison baseline | S2 |
| Empty-filtered state identical to first-use empty | S2 |
| Four filled buttons in one viewport | S2 |
| Nine type sizes in one view | S3 |
| Intra-zone and inter-zone gaps both 24px | S3 |
| Legend placed far from its chart | S3 |
| Icon optically 1px low against its label | S4 |

---

## Scoring rubric (140 max)

Score each dimension 0 to 4, multiply by weight.

| # | Dimension | Weight | What a 4 looks like |
| --- | --- | --- | --- |
| D1 | Purpose fidelity | 5 | Every element traces to a ranked question; top question answered in under 5 seconds |
| D2 | Information architecture | 5 | Grouping by question and object; every zone heading passes the heading test; disclosure matches frequency |
| D3 | Visual hierarchy | 5 | One focal point; visual rank equals logical rank; encoding budget respected |
| D4 | Action clarity | 4 | One primary per scope; monotonic emphasis decay; verb+object labels; all states defined |
| D5 | Content and context | 4 | User vocabulary; every value carries context; definitions available; favorability-based color |
| D6 | States and resilience | 3 | Full state set specified; extremes verified |
| D7 | Accessibility | 4 | All gates pass; charts have accessible equivalents |
| D8 | Craft and consistency | 3 | Restraint budget respected; tokenized; aligned; rhythmic |
| D9 | Responsiveness | 2 | Priority order holds at every breakpoint |

Maximum weighted score: (5+5+5+4+4+3+4+3+2) x 4 = **140**.

Score anchors: 0 absent (not considered at all), 1 attempted but fundamentally wrong, 2 partially correct with significant gaps, 3 correct with minor gaps, 4 correct and defensible against every rule in the dimension.

---

## Gate thresholds (`EVAL-03`)

| Outcome | Conditions |
| --- | --- |
| **Ship** | Weighted score >= 112 (80%), zero S1, at most 2 S2, and D1, D2, D3, D7 each >= 3 |
| **Revise** | Weighted score 84 to 111, or any dimension at 2, with a concrete fix list |
| **Redesign** | Weighted score <= 83, or D1 or D2 <= 1, or more than 4 S2 findings |

D1, D2, D3, D7 are floor-gated: purpose, architecture, hierarchy, and accessibility cannot be compensated for by strength elsewhere. A beautiful, accessible dashboard that answers the wrong question is worth less than an ugly one that answers the right one.

---

## Evaluating the problem, not just the solution

The request itself is an artifact to evaluate, before scoring any proposed solution.

| Check | Question | If it fails |
| --- | --- | --- |
| Solution smuggling | Is a solution embedded in the problem statement? ("Build a dashboard" is a solution; "I can't tell which queues need staff" is a problem) | Restate as a problem, then ask whether a dashboard is the right response |
| Wrong artifact | Would an alert, report, scheduled email, or workflow change serve better? | Propose the alternative with reasoning |
| Phantom user | Can you name a specific person or role who opens this weekly? | If not, the artifact won't be used, say so |
| Decision absence | Does any action follow from the information? | If not, this is reporting not decision support, scope accordingly |
| Existing behavior | What do they do today? | Design must beat the current workaround, often a spreadsheet |

**`EVAL-07` (alternative-artifact test):** before designing a dashboard, ask whether the decision is better served by a notification. Machine-generated design almost never asks this and builds monitoring screens no one watches.

---

## Self-critique rules for AI reviewers

**`EVAL-13` (adversarial framing):** run the critique pass in an explicitly adversarial posture. Instruct yourself to "find the reasons this design fails", not "verify this design is good". Confirmation bias in self-review is why AI self-critique usually returns "looks good".

**`EVAL-14` (separate passes in time and context):** produce the design, then produce the critique as a distinct step with the rule catalog reloaded, without reference to your own rationale. Judge the artifact, not the intent. This is the reason this file is reviewer-only and must be loaded into a fresh context.

**`EVAL-15` (quota critique):** require a minimum finding count on first review: at least 5 findings, including at least 1 S2 or higher. Fewer than 5 findings on a first-pass design means it has almost certainly not been reviewed properly. If you genuinely cannot find 5, look harder at states, extremes, accessibility, content.

**`EVAL-16` (steelman the alternative):** for any structural decision, state the strongest case for the option you rejected. If you cannot make that case, you did not genuinely consider it.

**Iteration discipline while reviewing:**
- `EVAL-09` fix the earliest link: purpose, then structure, then hierarchy, then action, then content, then states, then craft.
- `EVAL-10` one structural change per iteration, so effect is attributable.
- `EVAL-11` re-run passes 2 through 8 after any structural change.
- `EVAL-12` stop condition: gate passes and remaining findings are S3/S4 whose fixes cost more than the value added.

---

## Findings output format

Keep output compact. For each finding:

```
[ID] [Severity] [Rule ID] One-line description
     Evidence: observation or measurement
     Fix: specific change
     Effort: S/M/L
```

Then:

```
GATE: Ship | Revise | Redesign
SCORE: [x] / 140
FIX LIST: ordered, S1 first ascending effort, then S2, then S3
WORKING: name what should not change (prevents regression, EVAL-08)
```
