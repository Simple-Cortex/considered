# Accessibility gates

Purpose: the twelve accessibility gates and how to satisfy them during `compose`, `harden`, and `critique`. These are gates, not scores. A surface failing any of them is not shippable regardless of its rubric result.

Contents: the gate table, per-gate guidance, data visualization specifics, evidence and honest limitations, review handling.

Accessibility is dimension `D7` in the review rubric and is floor-gated: it cannot be compensated for by strength elsewhere. All twelve gates are severity S1: course unit-4 §4.3 treats any failed accessibility gate as a blocker, with no lower-severity tier for this dimension.

## 1. The gates

| ID | Gate | Requirement | Severity |
| --- | --- | --- | --- |
| `A11Y-01` | Text contrast | 4.5:1 for text under 18px, or under 14px bold; 3:1 for larger | S1 |
| `A11Y-02` | Non-text contrast | 3:1 for UI boundaries, icons, focus rings, and chart marks carrying meaning | S1 |
| `A11Y-03` | Color independence | Meaning never conveyed by color alone; pair with text, icon, pattern, or position | S1 |
| `A11Y-04` | Focus visible | Every interactive element has a visible focus indicator at 3:1 against adjacent colors | S1 |
| `A11Y-05` | Accessible names | Every control has a name; icon-only controls require an explicit label | S1 |
| `A11Y-06` | Target size | 44x44px minimum for touch; 24x24px minimum for pointer with adequate spacing | S1 |
| `A11Y-07` | Keyboard complete | Every action reachable and operable by keyboard, in a logical order, with no traps | S1 |
| `A11Y-08` | Live regions | Async updates, loading completion, and errors announced to assistive technology | S1 |
| `A11Y-09` | Motion safety | Respect reduced-motion preference; no essential information conveyed only by motion | S1 |
| `A11Y-10` | Chart alternatives | Every chart has an accessible equivalent: a data table, a summary sentence, or both | S1 |
| `A11Y-11` | Heading structure | Logical heading levels, no skipped levels, one h1 per view | S1 |
| `A11Y-12` | Error identification | Errors identified in text, tied to their field, describing the remedy | S1 |

## 2. Applying the gates

### Contrast and color

`A11Y-01` and `A11Y-02` are measurements, not judgments. Verify against the actual rendered values, including text over images, gradients, and chart fills.

`A11Y-03` is the one most often failed by otherwise careful work. A status system carried by red, amber, and green fills alone excludes a large population. Add an icon, a label, or a position that carries the same meaning. See `hierarchy.md` `HIER-10` for how status color interacts with accent color.

Never push a text color below the contrast floor to make it look lighter. If an element is too unimportant to meet contrast, delete it. That is a hierarchy problem wearing a color costume.

### Focus and keyboard

`A11Y-04` and `A11Y-07` travel together. Removing a focus outline without replacing it is an S1 finding, and it is the single most common accessibility regression in generated code.

Verify the full keyboard path: reach every action, in an order that matches the visual order, with no trap. Overlays must return focus to the trigger on close. `states.md` governs the per-state behavior; this gate governs its reachability.

### Names and structure

`A11Y-05`: an icon-only control needs an explicit accessible name. A tooltip is not a name.

`A11Y-11`: heading levels describe the document structure, not the type scale. Choosing an `h3` because it looks right is a structure failure. Use the token from `craft.md` for appearance and the correct level for structure.

### Motion, targets, and async

`A11Y-06`: check the interactive target, not the icon. A 16px icon inside a 44px button passes; a bare 16px icon does not.

`A11Y-08`: an async update that changes a number without announcing it is invisible to a screen reader user. Loading completion and errors both need announcement. See `states.md` for the state inventory this applies to.

`A11Y-09`: respect the reduced-motion preference, and never carry essential information in motion alone. `hierarchy.md` `HIER-04d` already restricts motion to state changes requiring intervention, which keeps most of this gate satisfied by construction.

### Errors

`A11Y-12`: an error must be identified in text, associated with its field, and describe the remedy. Colour on a border is not identification. `forms.md` governs the recovery path; this gate governs whether the error is perceivable at all.

## 3. Data visualization

`A11Y-10` deserves particular emphasis for dashboards. A dashboard whose meaning exists only in pixels excludes screen reader users entirely. Provide a text summary of the key finding for each chart. This also benefits sighted users, because writing the summary forces you to confirm the chart has a finding at all.

- Do not rely on hue alone to distinguish series. Use direct labels, varied line styles, or varied markers.
- Verify categorical palettes against the common color vision deficiencies.
- Maintain 3:1 contrast between adjacent series fills where distinguishing them matters.
- Never use red and green as the only distinction between two states.
- Provide the underlying data through a table or an export.

## 4. Evidence and honest limitation

Accessibility claims follow the same evidence rules as every other claim in this system.

| Gate | What source inspection can establish | What needs a render or assistive technology |
| --- | --- | --- |
| `A11Y-01`, `A11Y-02` | declared token values and their computed ratios | text over images, gradients, and live chart fills |
| `A11Y-03` | presence of a non-color carrier | whether the carrier is perceivable in context |
| `A11Y-04` | focus styles are not removed | the indicator's rendered contrast against neighbours |
| `A11Y-05`, `A11Y-11` | names and heading levels in markup | announced order in a real screen reader |
| `A11Y-06` | declared target dimensions | rendered size after reflow |
| `A11Y-07` | tab order in source order | actual traversal, traps, and focus return |
| `A11Y-08` | live region markup exists | whether the announcement is useful |
| `A11Y-09` | reduced-motion query present | motion behavior under the preference |
| `A11Y-10` | an equivalent exists | whether it conveys the same finding |
| `A11Y-12` | error text and association | announcement on validation failure |

Source-lint findings are reviewer leads, not proof. When no render exists, mark render accessibility `blocked` in `REVIEW.json` and state the limitation. Never record a passed accessibility gate that was not actually checked. An unverified gate is `blocked`, not `pass`.

## 5. In review

The reviewer runs these gates in critique pass 7 and scores dimension `D7`. Because `D7` is floor-gated, a score below the floor fails the gate no matter how strong the rest of the review is.

Report every failed gate as a finding with its rule ID, the evidence, and the specific remedy. "Improve accessibility" is not a finding. "`A11Y-04`: focus outline removed on the primary action; restore a 2px `focus-ring` at 3:1 against `surface-raised`" is.
