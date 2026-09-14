# Craft: tokens, components, and finish

Purpose: operating rules for turning a ranked, grouped structure into a built surface using a system rather than per-screen invention. Read during `compose` and `harden`, after `structure` has assigned zones and tiers.

Contents: composition order, token set, component selection, restraint budget, alignment, rhythm, density, optical correction, subtraction pass.

`hierarchy.md` governs how these instruments create emphasis. This file governs how they are defined once and reused. Where both name a scale, `hierarchy.md` gives the tier mapping and this file gives the concrete token.

## 1. Composition order

Composition runs in one direction. Reversing it is the cause of consistency drift.

```text
TOKENS   ->  PRIMITIVES     ->  PATTERNS       ->  VIEWS
values       button, input      metric card,       the dashboard
             table, chart       filter bar,
                                empty state
```

`COMP-01` (system before view): never style at the view level. If a view needs a treatment that does not exist, add it to the token or component layer with a name and a rule, then use it. A one-off value in a view is both technical and visual debt.

`COMP-05` (module consistency): the same pattern renders the same way everywhere. Two metric cards with different padding, heading weight, or label placement read as two systems.

`COMP-19` (native controls wear the theme): every native form control that renders — select, input, textarea, checkbox, radio, button — carries the surface's ground, ink, border, and focus tokens rather than the user agent default; a select whose current value is not legible against a themed ground is an unset-state defect, not a cosmetic one. This render-only loss was independently named on a settings surface, matching the failure already recorded in `m0-002`.

## 2. The token set

Define this before writing any component. It is the minimum viable set for a data-dense product.

### Spacing, 4px base

`space-1: 4` `space-2: 8` `space-3: 12` `space-4: 16` `space-5: 20` `space-6: 24` `space-8: 32` `space-10: 40` `space-12: 48` `space-16: 64`

`COMP-03` (tokenized spacing): every margin, padding, and gap uses a token. No arbitrary values. This single rule produces most of what reads as rhythm and craft.

See `hierarchy.md` `HIER-07` for the proximity ratio that governs which token to pick between groups.

### Typography

Concrete values within the ranges `hierarchy.md` assigns to each tier.

| Token | Size | Line height | Weight | Use |
| --- | --- | --- | --- | --- |
| `text-display` | 44px | 1.1 | 600 | P0 value only |
| `text-title` | 26px | 1.2 | 600 | P1 values, page title |
| `text-heading` | 17px | 1.35 | 600 | Zone and module headings |
| `text-body` | 14px | 1.5 | 400 | Default content |
| `text-caption` | 12px | 1.4 | 400-500 | Labels, units, metadata, axis ticks |

`COMP-02` (type scale limits): at most 5 type sizes, 3 weights, and 3 text colors per view.

Numeric values in tables and metric blocks use tabular figures (`font-variant-numeric: tabular-nums`). Without it, digit widths shift and columns of numbers become hard to compare. Small detail, disproportionate effect on perceived quality in data products.

### Color roles

| Token | Purpose |
| --- | --- |
| `surface`, `surface-raised`, `surface-sunken` | Background layers, three steps maximum |
| `border-subtle`, `border-default`, `border-strong` | Structure |
| `text-primary`, `text-secondary`, `text-tertiary`, `text-inverse` | Reading hierarchy |
| `accent`, `accent-hover`, `accent-subtle`, `accent-text` | Interaction and focal emphasis |
| `success`, `warning`, `danger`, `info`, each with `-subtle` and `-text` | Status semantics |
| `data-1` to `data-6` | Categorical series |
| `focus-ring` | Focus indication, never removed |

`COMP-04` (semantic token names): name a token for its role, never its value. `danger`, not `red`. A literal name guarantees the value can never change and that it will be misapplied for decoration.

### Radius, elevation, grid

| Token | Value | Use |
| --- | --- | --- |
| `radius-sm` | 4px | Inputs, chips, small controls |
| `radius-md` | 8px | Cards, modules |
| `radius-lg` | 12px | Modals, drawers |
| `elevation-0` | none | Default. Most surfaces |
| `elevation-1` | subtle | Raised cards, only where separation is needed |
| `elevation-2` | medium | Dropdowns, popovers |
| `elevation-3` | strong | Modals |

`COMP-10` (semantic elevation): shadow encodes distance from the page, which encodes transience. A permanent card does not float. Decorative shadows on every module flatten the meaning and add noise.

`COMP-11` (one radius family): pick a radius scale and apply it consistently. Mixed radii in one view is among the fastest ways to make a design look unresolved.

`COMP-08` (declared grid): state the column count, gutter, and margin in the layout spec. An undeclared grid produces alignment that cannot be verified.

`COMP-09` (priority holds across breakpoints): P0 stays P0 at every width. Reflow may change position and column count; it may not reorder the tiers or push a P1 below a P3.

## 3. Component selection

`COMP-12` (component from question): component choice follows from the question and the object count, not from visual preference.

### Display container

| Situation | Component |
| --- | --- |
| One value, needs strong emphasis | Metric block, no container |
| One value plus supporting visual | Metric card |
| 2 to 6 comparable items, rich content each | Card grid |
| 7 or more homogeneous records, comparison needed | Table |
| Homogeneous records, scanning one attribute | List |
| Sequential events with timestamps | Timeline |
| Hierarchical records | Tree table |

`COMP-13` (cards versus tables): cards are for heterogeneous, browsable content where each item deserves attention. Tables are for homogeneous, comparable records. A card grid of 30 identical records is a table rendered inefficiently, and it makes comparison harder rather than easier.

### Overlay and disclosure

| Situation | Component | Notes |
| --- | --- | --- |
| Blocking decision, short | Modal | Under 5 fields |
| Detail while keeping the list visible | Side drawer | Preferred for record inspection |
| Contextual options for one element | Popover or menu | Anchored to the trigger |
| Extra detail in place | Accordion or inline expand | Preserves scroll position |
| A full task | Dedicated page | Not an overlay |
| Precise value or definition | Tooltip | Never for required information |

`COMP-14` (no nested overlays): a modal opening a modal means the task decomposition is wrong. Use a page, or a multi-step flow within one surface.

See `ia.md` for choosing between disclosure and navigation before choosing a component.

### Input

| Situation | Component |
| --- | --- |
| 2 mutually exclusive options | Toggle or segmented control |
| 3 to 5 mutually exclusive options | Radio group or segmented control |
| 6 to 15 options | Select |
| 15 or more options | Combobox with search |
| Multiple selection, few options | Checkbox group |
| Multiple selection, many options | Multi-select with chips |
| Bounded numeric range | Slider paired with a numeric input |
| Date, single | Date picker with typed input |
| Date range | Range picker, presets first |

`COMP-15` (presets before pickers): for date ranges, offer the common presets first. Most people want "last 7 days", not a calendar.

`CONT-06` (controls carry their consequences): every setting or consequential control carries a one-line description of what it does or what changing it costs, placed at the point of need — and gating that affects it (plan tier, permission, prerequisite) is cued on the control, not discovered on failure. Shipped settings surfaces annotate nearly every control; a bare label row reads clean but forces the user to test-fire the control to learn what it does. Write descriptions from supplied facts; where the consequence is not supplied, describe the action ("Applies at next sign-in" only if supplied), never an invented policy (`HON-02`).

## 4. Craft

Beauty in interface work is not decoration. It is the visible result of a small number of measurable disciplines, and each one is checkable.

### 4.1 Restraint

`COMP-16` (restraint budget), per view:

| Property | Maximum |
| --- | --- |
| Type sizes | 5 |
| Font weights | 3 |
| Text colors | 3 |
| Accent hues | 1 |
| Categorical data colors | 6 |
| Border styles | 2 |
| Radius values | 2 |
| Shadow levels | 2 |
| Icon styles | 1 family, one weight, one size scale |

Exceeding these is the most reliable predictor of a design being called busy, cluttered, or unprofessional. When a design feels wrong and you cannot say why, count these first.

`COMP-06` (whitespace budget): negative space is 30–40% for strategic and analytical surfaces, 20–30% for operational, per `dense.md`. As guidance beyond the measurable rule, concentrate that budget around the P0 and between zones rather than distributing it evenly.

`COMP-07` (non-data ink justified): every decorative element justifies its presence against the data it competes with. Gridlines, borders, and container fills are costs paid for a reason.

### 4.2 Alignment

`COMP-17` (alignment discipline): every element aligns to the grid, and every group shares an alignment edge. Minimize the number of distinct alignment edges in a view.

- Module titles align across a row.
- Numbers align right, text aligns left. Never center-align columns of data.
- Labels and values share a consistent left edge or a consistent baseline.
- Icons align optically with adjacent text, not mathematically.

### 4.3 Rhythm

`COMP-18` (rhythm limit): vertical spacing between sibling elements uses no more than 3 distinct values within a zone. When spacing is arbitrary, the eye detects the irregularity even when the viewer cannot name it.

### 4.4 Density calibration

Density follows from frequency of use and expertise, decided in `frame`, not chosen visually.

| Density | Row height | Module padding | Font | Use |
| --- | --- | --- | --- | --- |
| Comfortable | 48px | 24px | 14px | Weekly use, low expertise |
| Standard | 40px | 20px | 14px | Daily use |
| Compact | 32px | 16px | 13px | Many times daily, trained |

Offer a density toggle in operational products. Do not force one density on every user.

### 4.5 Optical correction

- Center text optically, not mathematically, when it sits beside an icon of different visual mass.
- Icons usually need to be 1 to 2px larger than the cap height of adjacent text to appear the same size.
- Fully round elements need slightly more horizontal padding than square ones to look balanced.
- Pure white on pure black, and pure black on pure white, both cause halation. Use near values.

### 4.6 The subtraction pass

`EVAL-06` (subtraction pass): before finalizing, run one pass whose only permitted operation is removal. Remove borders that space already implies, icons that labels already explain, headings that content already announces, gridlines, legends replaced by direct labels, and any color not carrying meaning.

A design that survives an aggressive subtraction pass with its meaning intact is reliably the one people call beautiful. The `subtract` command exists for this pass.

### 4.7 The supplied-fact boundary

The brief's fact set is closed. Structural discipline produces richer surfaces — more states, more disclosure, more copy — and every added surface is a new place to assert something nobody supplied. Measured review found exactly this failure concentrated in otherwise stronger outputs: invented member directories with contact details, reversibility promises on destructive actions, activation-email mechanics, "before" conditions built to frame an after, and durations stated with the confidence of a supplied fact.

`HON-01` (rendered claims trace to supplied facts): every material claim in rendered copy — visible or assistive — must trace to the brief, the target source, or user-supplied evidence. A claim that traces to nothing does not render. The gap it was covering is either a `FRAME.md` assumption (a design input, never copy) or a question under the blocking-fact test in `workflow.md`.

The boundary by family:

| Family | Never render unsupplied | Render instead |
| --- | --- | --- |
| Policy and capability (`HON-02`) | account rules, permission gates, undo and reversibility, retention, enforcement mechanics | the behavior the facts support, with conservative treatment of the unknown |
| Sample data (`HON-03`) | new totals, distributions, trends, availability statements, contact details | instances of the supplied schema, figures derived from supplied ones |
| Outcomes (`HON-04`) | "sent", "we'll follow up", "you can restore this later" without a mechanism | the verifiable event the build performs |
| Assistive text (`HON-05`) | summaries or trends stronger than the visible copy | a restatement of visible, supported content |

`STATE-05`'s freshness clamp is the proven instance of this rule: the budget is a design input recorded as an assumption, never a rendered label. Apply the same move to every unsupplied fact — record it, design against it, do not print it.

Reversibility is the recurring trap, and the boundary cuts both ways: when the facts say nothing about undo or recall, a fabricated "this cannot be undone" is the same defect as a fabricated "you can restore this later" (`HON-02`). Treat the unknown conservatively in behavior — require confirmation, never show optimistic success (`ACT-18`) — but keep the copy to the action and its supplied consequences; an unknown consequence stays unstated.

Three instantiation edges recur under measurement:

- A control that needs a current value the facts do not supply renders its unset state — "Not set", an unconfigured setup card — never an invented default and never asserted emptiness. A pre-filled "UTC" invents configuration; "No invoices yet" is as much a data claim as an invented invoice (`HON-03`). Binary controls are included: an unknown boolean renders an explicit unknown, not a definite off.
- A supplied rule does not license invented enforcement or explanation. "Requires a work email" supplies a policy, not a personal-domain blocklist; a service's name supplies a name, not a role description (`HON-01`).
- The chain never renders (`HON-06`). Design rationale ("ranked by consequence: X affects every member"), brief instructions restated as product facts ("no flags beyond these exist yet"), and completeness meta-claims belong in the artifact record, not the copy. If a sentence explains the design to the user or promises the page is exhaustive, delete it.

Real copy is still required (`states.md`); the boundary is not a license for lorem ipsum. Write concrete copy from the facts you have. Where a surface needs instance data the brief did not supply, keep it schema-faithful and figure-consistent (`HON-03`) and say so in the artifact record.

### 4.8 The polish pass

`EVAL-17` (the polish pass): after the subtraction pass, run one zoomed-in pass whose only question is whether the system survived its own construction. Check, in order:

- Every spacing and size value still sits on the scale — late edits are where 13px gaps enter.
- Same-role elements are styled identically everywhere; a card padded differently from its siblings reads as broken, not varied.
- All-caps labels carry opened tracking (0.04–0.08em); condensed all-caps at default tracking is a machine tell.
- Display copy carries no widowed last word.
- The squint test resolves: blurred, the page still shows one primary element, its seconds, and the groupings — this is also the enforcement moment for the proximity ratio (`HIER-07`), the rhythm limit (`COMP-18`), and decisive tier separation (`HIER-13`). Equal spacing everywhere satisfies a value count and still reads as a form stack; the squint is what catches it.

The pass is cheap and its absence is visible: blind review against shipped work reliably names unrefined spacing and one-weight hierarchy as what separates a generated surface from a designed one.

## 5. Compose checklist

Before marking `compose` complete:

- Every spacing, color, type, and radius value resolves to a token (`COMP-03`, `COMP-04`).
- The restraint budget holds on every count (`COMP-16`).
- Each component choice traces to a question and an object count (`COMP-12`).
- No nested overlays exist (`COMP-14`).
- Tier order survives every declared breakpoint (`COMP-09`).
- A removal-only pass has been run (`EVAL-06`), then a polish pass (`EVAL-17`).
- No rendered claim outruns the supplied facts, assistive text included (`HON-01`, `HON-02`, `HON-03`, `HON-04`, `HON-05`).
- The accessibility gates in `accessibility.md` pass. They are gates, not scores.
