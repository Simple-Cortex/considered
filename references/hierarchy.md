# Hierarchy Reference

Operating rules for assigning and encoding visual/logical/interaction hierarchy in a design. Cross-reference rule IDs against the reviewer agent and scripts.

Contents: three hierarchies, priority tiers, encoding budget, instruments (position, type, space, color, charts), executable tests, Hierarchy Table artifact.

---

## 1. The three hierarchies

| Hierarchy | What it orders | Lives in | Failure if absent |
| --- | --- | --- | --- |
| Logical | Importance to the decision | Zone Map, Hierarchy Table | Shows the wrong things |
| Visual | Order the eye lands | Size, position, contrast, color, space | Right things read in wrong order |
| Interaction | Priority of actions | Action tiers, placement | User doesn't know what to do |

`HIER-02` (governing law): visual rank must equal logical rank; interaction rank must equal decision rank. Divergence causes cognitive load even on a "clean" screen.

**Diagnostic:** write logical rank order of top 8 elements independently from visual rank order (grayscale screenshot). Mismatch in top 3 = `HIER-02` fail, severity S1.

## 2. Priority tiers

Assign exactly one tier per element, derived from the Question Inventory (A2), never from intuition.

| Tier | Definition | Count per view | Reading time |
| --- | --- | --- | --- |
| P0 Focal | Single answer to "am I okay?" / object of primary job | Exactly 1 | 0-2s |
| P1 Primary | Answers top-ranked question, drives decision | 3-5 | 2-8s |
| P2 Supporting | Explains/decomposes a P1 | 5-12 | 8-30s |
| P3 Reference | Detail, raw records, config | Unlimited, deferred | On demand |
| P4 Chrome | Nav, filters, branding, utility | Minimal | Peripheral, never focal |

### Tier assignment procedure (first match wins)

```
1. Answers the #1 ranked question in a single glance? -> P0 (only one; if two compete, split view or demote one)
2. Answers a top-5 question AND changes the user's action if it changes? -> P1
3. Exists to explain/decompose/contextualize a P0 or P1? -> P2
4. Needed only once user has decided to investigate? -> P3 (deferred: drill, expand, tab, drawer, detail page)
5. Navigation, filtering, identity, utility? -> P4
6. None of the above? -> DELETE
```

`HIER-05`: P0 count is exactly 1, never 0 or 2.
`HIER-06`: if P1 count > 5, re-group, split views, or demote.

Tier is a property of the view, not the data: the same metric can be P0 on one view and P3 on another. Never assign global importance to a metric.

## 3. The encoding budget

Emphasis is zero-sum across a view. If everything is emphasized, nothing is.

### Device table

| Device | Points | Notes |
| --- | --- | --- |
| Position | 2 | Top-left / first in reading order; cheapest, strongest |
| Size / area | 2 | Doubling area ~ one tier of promotion |
| Weight | 1 | Bolder type/stroke |
| Color contrast | 2 | Most abused; use last |
| Enclosure | 1 | Card/border/fill; strong grouping side effect |
| Whitespace isolation | 1 | High quality; reads as confidence |
| Motion | 3 | Reserve for genuine alerts only |

### `HIER-04` encoding budget by tier

| Tier | Allowed points | Typical combo |
| --- | --- | --- |
| P0 | 4-6 | Position 2 + Size 2 + Whitespace 1 |
| P1 | 2-3 | Size 2 + Weight 1, or Enclosure 1 + Weight 1 |
| P2 | 1-2 | Enclosure 1, or Weight 1 |
| P3 | 0-1 | Default |
| P4 | 0 | Recede: low contrast, no enclosure, no accent |

Corollaries:
- `HIER-04a`: no more than one element in a view may exceed 4 points.
- `HIER-04b`: accent color scarce; marks at most the P0 element plus the primary action per view.
- `HIER-04c`: never stack redundant devices at low tiers (pick one, not border+tint+bold+icon).
- `HIER-04d`: motion reserved for state changes requiring intervention; decorative entrance motion is `COMP-07`.

### Worked example totals (for calibration)

P0 headline: Position 2 + Size 2 + Whitespace 1 = 5. P1 list: Enclosure 1 + Weight 1 + Color 1 = 3. P1 sparkline: Size 2 = 2. P2 items: Enclosure 1 each. P3: 0. P4: 0. One element at 5, nothing else above 3, passes squint test. Contrast: 12 equal cards each scoring 4-5 = Uniform Weight Syndrome (B1) + Emphasis Inflation (B2).

## 4. Instruments of hierarchy

### 4.1 Position
- Top-left quadrant receives first fixation; place P0 there or spanning it.
- F-pattern for text/list-dense content, Z-pattern for sparse/poster layouts.
- Vertical order outweighs horizontal once scanning starts; first row strongest, decays fast.
- Below the fold = tier demotion by default.
- `HIER-08`: define target viewport explicitly (e.g. 1440x900) in A7 Layout Spec; verify P0 and all P1 fit above the fold at that viewport.

### 4.2 Typographic scale

`COMP-02`: max 5 type sizes, 3 weights, 3 text colors per view.

| Role | Size | Weight | Color role | Use |
| --- | --- | --- | --- | --- |
| Display | 40-48px | 600-700 | text-primary | P0 value only |
| Title | 24-28px | 600 | text-primary | P1 values, page title |
| Heading | 16-18px | 600 | text-primary | Zone/card headings |
| Body | 14px | 400 | text-primary | Default content |
| Caption | 12px | 400-500 | text-secondary | Labels, units, metadata, axis ticks |

Text color roles (3 only), each min 4.5:1 contrast: text-primary `#111827`, text-secondary `#4B5563`, text-tertiary `#6B7280`. No pure black. Never push tertiary below contrast floor to fake lightness; if too unimportant for contrast, delete it.

`DASH-06` (label-value inversion): value carries information (larger size, stronger color), label carries identification (caption size, secondary color). Label rendered at same/greater weight than value is a common machine error.

`HIER-13` (adjacent tiers separate decisively): the scale sets a ceiling on variety; this sets the floor on contrast. Adjacent tiers must differ in at least two of size step, weight, or spatial treatment, and by enough to read at a glance — a 600-weight heading over 600-weight body, or a distinction carried only by Regular versus Medium, is one tier rendered twice. Blind review against shipped work names this failure as "everything reads at one weight"; when a tier cannot be distinguished blurred, merge it or move it a full step.

### 4.3 Space as grouping instrument

`HIER-07` (proximity ratio): `gap_inside_group <= 0.5 * gap_between_groups`. Eye needs roughly 2:1 ratio to segment reliably.

Spacing scale (4px base):

| Token | Value | Use |
| --- | --- | --- |
| space-1 | 4px | Icon to text |
| space-2 | 8px | Label to value |
| space-3 | 12px | Between related rows |
| space-4 | 16px | Card padding |
| space-6 | 24px | Between cards in a zone |
| space-8 | 32px | Between zones |
| space-12 | 48px | Between major page regions |

Example: cards at 24px, zones at 32px = ratio 0.75, ambiguous. Fix: zone gap to 48px, or intra-zone gap to 16px.

`HIER-09`: prefer space over borders for grouping; add border/surface fill only when space alone can't express it (e.g. dense operational tables). Every border competes with data.

### 4.4 Color as hierarchy

Roles must be semantic, never literal.

| Role | Purpose | Constraint |
| --- | --- | --- |
| `accent` | Interactive/focal emphasis | One hue per product |
| text-primary/secondary/tertiary | Reading hierarchy | Three steps |
| surface, surface-raised, surface-sunken | Grouping/elevation | Three steps |
| border-subtle/default/strong | Structure | Three steps |
| success/warning/danger/info | Status semantics | Never decorative |
| data-1 through data-6 | Categorical series | Six max, then "Other" |

`A11Y-03`: color never the sole carrier of meaning; pair with icon, label, or position.
`HIER-10`: status color outranks accent color. If danger state and primary action both visible, danger wins the eye; ensure it deserves that priority.

### 4.5 Visual weight of chart types

| Chart | Perceptual weight | Appropriate tier |
| --- | --- | --- |
| Large single value + trend | Very high | P0, P1 |
| Filled area chart | High | P1 |
| Bar chart | High | P1, P2 |
| Line chart | Medium | P1, P2 |
| Sparkline | Low | P1 accent, P2 |
| Table | Low per cell, high in aggregate | P2, P3 |
| Heatmap | High, attention-grabbing | P1, P2 |
| Pie / donut | High weight, low info value | Avoid; only 2-3 part-to-whole |

## 5. Executable hierarchy tests

| Test | Rule ID | Procedure | Pass | Fail |
| --- | --- | --- | --- | --- |
| Squint test | `HIER-01` | Gaussian blur ~8-12px, or downscale 10% and back | Exactly one dominant region, 2-3 secondary distinguishable | Uniform gray or multiple equally dominant regions |
| Grayscale test | `HIER-11` | Render in grayscale | Hierarchy and status meanings stay legible | Structure or status semantics lost |
| Five-second test | `HIER-12` | Expose view 5s, ask what's most important and its state | Correct, confident answer | Hesitation, wrong element, "a lot going on" |
| Trace test | `IA-01` | For every element, name the A2 question it answers | 100% coverage | Any orphan element |
| Remove-and-justify | `EVAL-05` | Consider each element removed; what decision becomes impossible/slower | Concrete answer per element | "Nice to have" / "stakeholder asked" / "we have the data" |
| Inversion check | `HIER-02` | Compare logical vs observed visual rank order, top 8 | Top 3 match | Any top-3 mismatch (severity S1) |
| Proximity audit | `HIER-07` | Measure intra/inter-group gaps per zone | Every zone satisfies gap_inside <= 0.5 * gap_between | Any zone exceeds 0.5 |

Fix priority for squint-test failures: subtract points from competing elements before adding points to the intended P0.

## 6. Artifact A5: Hierarchy Table

Required columns: ID, Element, Answers question, Tier, Devices (points), Position, Notes.

Validation checks against the table:
- Exactly one P0 (`HIER-05`).
- P1 count between 3 and 5 (`HIER-06`).
- No element other than P0 exceeds 4 points (`HIER-04a`).
- Every row has a non-empty "Answers question" (`IA-01`).
- Accent color used at most twice across the whole table (`HIER-04b`).
