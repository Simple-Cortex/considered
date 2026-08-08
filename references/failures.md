# Failures

Purpose: fast diagnostic reference for the 20 named failure modes of machine-generated interface design, plus surface-level AI tells to refuse on sight.

Contents: failure mode table (20 modes across structure, hierarchy, action, content, reality), AI tells list.

How to use this file: when reviewing a design, scan group by group (structure first, since a structural failure invalidates hierarchy and craft findings downstream). Name the failure mode explicitly in any critique output, then cite its corrective rule ID. Naming the failure is what makes it detectable and fixable, a vague complaint like "feels off" is not a finding.

---

## Failure mode diagnostic table

Use this to name a failure the moment you see its signature, then apply the corrective rule.

Group themes, for fast recall:

| Group | Theme | Stage it belongs to |
| --- | --- | --- |
| A. Structural | Wrong things are grouped together, or nothing is grouped at all | Information architecture |
| B. Hierarchy | Everything competes for attention, or the wrong thing wins | Visual hierarchy |
| C. Action | The user can't tell what to click, or what happens if they do | Action definition |
| D. Content | The number or label is present but its meaning was never computed | Content and context |
| E. Reality | The design only works for the one dataset it was built against | States and resilience |

### Group A: Structural failures

| Name | Signature | Root cause | Corrective |
| --- | --- | --- | --- |
| A1. Chart Dump | Every available metric rendered, one card each, uniform grid | Data availability treated as design input instead of constraint | `IA-01`: every element traces to a ranked question, or is deleted |
| A2. Type Grouping | KPI tiles together, then line charts together, then tables together | Sorting by rendering similarity (machine's axis) not semantic similarity (human's axis) | `IA-02`: group by question asked and object acted upon |
| A3. Source Grouping | Sections named after systems: "Salesforce", "Stripe", "Google Analytics" | Data pipeline structure leaks into the interface | `IA-03`: group by the user's mental model, not the integration list |
| A4. Grid Tyranny | 2x2/3x3/4-column grid of equal cards, chosen before content known | Layout selected first, content poured in second | `HIER-03`: content priority determines module size |
| A5. Flat Depth | Everything on one screen at one level, no drill-down or disclosure | "At a glance" misread as "all at once" | `IA-08`: default view answers top questions, rest is one interaction away |
| A6. Infinite Nav | Sidebar with 15+ ungrouped items, or nav nested 4 levels deep | Every feature gets a nav entry, no consolidation pass | `IA-06`: max 7 items per nav group, max 3 levels deep |

### Group B: Hierarchy failures

| Name | Signature | Root cause | Corrective |
| --- | --- | --- | --- |
| B1. Uniform Weight Syndrome | Squint at the design, it becomes an even gray field, nothing dominates | No priority tiers assigned, no differential encoding applied | `HIER-01`: exactly one element wins the squint test |
| B2. Emphasis Inflation | Six elements bold, large, colored, and boxed. Everything shouts | Emphasis applied per-element instead of budgeted across the view | `HIER-04`: enforce the encoding budget, emphasis is zero-sum |
| B3. Hierarchy Inversion | A decorative header, logo, or filter bar is the most dominant thing on the page | Chrome styled with the same care as content | `HIER-02`: visual rank must equal logical rank, chrome recedes |
| B4. Typographic Sprawl | Eight font sizes, four weights, five text colors on one screen | Each element styled locally to look good on its own | `COMP-02`: max 5 sizes, 3 weights, 3 text colors per view |
| B5. Uniform Spacing | Same gap between everything, grouping carries no meaning | A single spacing value applied globally | `HIER-07`: intra-group gap at most half of inter-group gap |

### Group C: Action failures

| Name | Signature | Root cause | Corrective |
| --- | --- | --- | --- |
| C1. Action Flatland | Every button rendered identically, user can't tell what to do | No action ranking pass, buttons treated as component not decision | `ACT-02`: every action gets a tier |
| C2. Primary Inflation | Four or more filled, high-emphasis buttons in one viewport | "Primary" applied to every action someone considered important | `ACT-01`: one primary per scope |
| C3. Scope Collision | A row-level "Delete" is as visually loud as the page-level "Publish" | Action tier assigned without reference to scope depth | `ACT-03`: emphasis decreases monotonically as scope narrows |
| C4. Dangerous Default | "Delete"/"Cancel subscription"/"Reset" styled as filled primary in confirm position | Frequency or prominence confused with safety | `ACT-08`: destructive is never default primary unless destruction is the scope's job |
| C5. Mystery Meat | Icon-only buttons, no label, no tooltip, no accessible name | Density optimized ahead of comprehension | `ACT-06`, `A11Y-05`: icon-only requires accessible name plus tooltip |
| C6. Ambiguous Verb | "Submit", "OK", "Go", "Continue" with no object | Label written from the system's perspective | `ACT-05`: verb plus object, three words or fewer |

### Group D: Content and comprehension failures

| Name | Signature | Root cause | Corrective |
| --- | --- | --- | --- |
| D1. Context-Free Number | "Revenue: $482,910" with no comparison, target, trend, or time frame | Value was available, its meaning was not computed | `DASH-05`: every number gets comparison, target, or trend with magnitude |
| D2. Schema Leak | Labels like `usr_cnt_30d`, "Avg. Sess. Dur.", "MRR ARR NRR" with no glossary | Database/internal vocabulary passed through to the surface | `CONT-01`: use the user's words, define every metric once |
| D3. So-What Vacuum | User can read the screen but can't say what to do differently | Design stopped at reporting instead of continuing to decision support | `DASH-07`: every zone answers "so what" and, where applicable, "now what" |
| D4. Decorative Density | Gradients, drop shadows, icons on every tile, 3D charts, animated counters | Visual interest substituted for information value | `COMP-07`: non-data ink must justify itself, delete decoration competing with data |
| D5. Color as Meaning Only | Status conveyed purely by red/amber/green fills | Color is the easiest available encoding | `A11Y-03`: color never the only carrier of meaning, pair with text/icon/position |

### Group E: Reality failures

| Name | Signature | Root cause | Corrective |
| --- | --- | --- | --- |
| E1. Happy Path Only | Design works with exactly the sample data provided and nothing else | States other than "populated and correct" never enumerated | `STATE-01`: specify empty, loading, partial, stale, error, no-permission |
| E2. Extremes Blindness | Breaks with a 60-char name, value of 0, negative number, 1M rows, or one row | Designed for the median case only | `STATE-04`: test zero, one, many, huge, negative, null, very long |
| E3. Responsive Afterthought | Fixed multi-column layout collapses into meaningless vertical stack, no re-prioritization | Hierarchy defined only in two dimensions at one width | `COMP-09`: priority order defined once, holds across breakpoints |
| E4. Consistency Drift | Three card styles, two border radii, four shadow depths in one product | Components generated per screen instead of drawn from a system | `COMP-01`: tokens first, components second, screens third |

---

## AI tells

Surface-level giveaways of machine-generated design. Refuse these on sight, before any deeper critique pass. These are visual patterns, not rule violations from the table above, but they correlate strongly with A4 Grid Tyranny and D4 Decorative Density and are usually reason enough to reject a draft outright.

| Tell | What it looks like |
| --- | --- |
| Uniform card grids | Every module the same size regardless of priority or content weight |
| Three-feature-card rows | Three equal boxes, each with an icon, a heading, and one sentence |
| Gradient orbs | Blurred colored circles or blobs floating decoratively behind content |
| Glassmorphism | Frosted-glass translucent panels layered for effect, not for function |
| Purple-to-blue tech gradients | Applied to buttons, headers, or backgrounds as a generic "tech" signal, unrelated to brand |
| Italic serif headings + sans body | Paired together as a fake "premium" or "editorial" signal |
| Decorative pulsing status dots | A pulsing dot with no actual live state behind it |
| Emoji as iconography | Emoji substituted for a real icon set |
| Fake dashboard screenshots | Invented charts showing no real data behind them |
| Invented company logos | A "trusted by" row of logos for companies with no real relationship |
| Centered everything | Text, layout, and sections centered regardless of content type or reading pattern |
| Over-rounded corners everywhere | Every element gets the same large radius with no semantic reason |

If a draft shows three or more of these together, reject it before running the 8-pass critique protocol. These are zero-cost to catch and catching them early saves a full review cycle.

---

## Common combinations

These failure modes rarely occur alone. Spotting one is a signal to check its usual companions.

| Primary failure | Frequently co-occurs with | Why |
| --- | --- | --- |
| A2 Type Grouping | B1 Uniform Weight Syndrome | Grouping by component type produces visually identical groups, so no priority tiers get assigned either |
| A4 Grid Tyranny | D1 Context-Free Number | Equal-size cards force equal-length content, which crowds out comparisons and targets |
| B2 Emphasis Inflation | C2 Primary Inflation | The same undisciplined instinct (apply emphasis everywhere) shows up in both styling and buttons |
| E1 Happy Path Only | E2 Extremes Blindness | Both stem from designing against the one sample dataset instead of the full state and value space |
| D2 Schema Leak | D1 Context-Free Number | Both stem from surfacing raw system output (field names, raw values) without translation for the user |

## Self-check before ship

- [ ] Named the failure mode explicitly for every finding, not just described the symptom
- [ ] Checked all 20 modes, not just the ones that jumped out first
- [ ] Cited the corrective rule ID for each named failure
- [ ] Scanned for AI tells before running the full critique protocol
- [ ] Checked common combinations when one failure in a pair was found
