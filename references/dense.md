# Dense surfaces

Purpose: operating rules for composing information-dense surfaces (dashboards) once IA is settled: structure, metric qualification, charts, module anatomy, grid, states, interaction.

Contents: four-layer argument, surface types, KPI qualification gate, context rules, chart selection, module anatomy, grid and density, states, interaction model.

## A dashboard is an argument

A dashboard is a structured argument leading a human from "what is happening" to "what should I do", quickly and repeatedly:

```
ORIENT   ->   STATE   ->   EXPLAIN   ->   ACT
what am I      am I       what changed    what do
looking at     okay?      and why?        I do now?
```

`DASH-01` (four-layer structure): every dashboard implements these four layers in reading order, with decreasing visual weight.

| Layer | Content | Tier | Typical placement |
| --- | --- | --- | --- |
| **Orient** | Title, scope, date range, active filters, freshness | P4 | Top strip, low contrast |
| **State** | 3 to 5 decision-driving values, exceptions, alerts | P0, P1 | First zone, top-left, strongest weight |
| **Explain** | Trends, comparisons, breakdowns, contributions | P1, P2 | Middle zones |
| **Act** | Work queues, detail tables, drill paths, actions | P2, P3 | Lower zones and drill targets |

Missing Orient: numbers no one trusts. Missing State: it's a report. Missing Explain: it's a scoreboard. Missing Act: it's a spectator sport, the "so what" vacuum (most common and most damaging omission).

## Surface (dashboard) types

Type determines density, refresh, interaction, and what belongs on it. Choose and declare the type in A1.

| Type | User | Decision horizon | Density | Refresh | Interaction | Module count |
| --- | --- | --- | --- | --- | --- | --- |
| **Operational** | Frontline, trained | Minutes | High | Live or near-live | Act in place | 6 to 10 |
| **Analytical** | Analyst | Hours to days | Medium to high | On load | Filter, pivot, drill deeply | 8 to 12 |
| **Strategic** | Executive | Weeks to quarters | Low | Daily or weekly | Minimal, mostly read | 4 to 6 |
| **Embedded** | End customer | Varies | Low | On load | Very limited | 3 to 6 |

`DASH-02`: don't mix types in one view. An executive summary bolted onto an operational queue view serves neither user. Build two views and link them.

`DASH-03` (module budget): respect the module count for the declared type. A module is any discrete content block (metric, chart, table, list). Exceeding the budget is a structural failure, not a scrolling problem.

## Metric selection: the KPI qualification gate

`DASH-04`: a candidate metric is promoted to KPI status (may occupy P0 or P1) only if it passes at least 4 of these 5 gates.

| Gate | Question | Fails if |
| --- | --- | --- |
| **Owner** | Is there a specific person or role accountable for this number? | Nobody owns it |
| **Decision** | Does a change in this number change what someone does? | It is merely interesting |
| **Baseline** | Is there a target, threshold, or prior period to compare against? | It floats without reference |
| **Actionability** | Can the user influence it, directly or indirectly? | It is weather, not a lever |
| **Sensitivity** | Does it move meaningfully at the dashboard's refresh cadence? | It is flat between checks |

Outcomes:

- **5 of 5:** Strong KPI. Eligible for P0 or P1.
- **4 of 5:** KPI. Eligible for P1.
- **3 of 5:** Supporting metric. P2 at best.
- **2 or fewer:** Not a dashboard metric. Move to a report, or delete.

This gate typically removes 40 to 60 percent of metrics in an unfiltered request: the reduction that turns a data dump into a dashboard.

## Context rules: making a number mean something

`DASH-05`: every displayed value carries at least one context element. A bare number is not information.

Context elements, in order of usefulness:

1. **Comparison to target or threshold.** "82% of 90% target." Strongest form, directly implies action.
2. **Comparison to prior period.** "+12% vs last week." State the period.
3. **Trend with magnitude.** Sparkline plus direction and delta.
4. **Distribution position.** "P92 of the last 90 days." Good for detecting abnormality.
5. **Absolute plus relative pair.** "142 breaches (3.2% of volume)." Prevents denominators-hidden and small-numbers-inflated errors.

`DASH-06` (label-value weight): value dominates label.

`DASH-07` (so-what test): for every zone, state the sentence a user should be able to say after reading it. If you can't write that sentence, the zone has no purpose.

`DASH-08` (significance): don't signal a change that isn't significant. Color and arrows on noise train users to ignore them. Define a threshold below which a delta renders neutral.

`DASH-09` (direction is not goodness): up isn't always good. Bind color to whether the movement is favorable, not to the sign of the delta. A rising error rate is not green.

## Chart selection

`DASH-10`: choose the chart from the question, not from variety. Repeating the same chart type is a feature, not a monotony problem.

| The question | Chart | Notes |
| --- | --- | --- |
| What is the single current value? | Big number with context | Not a gauge |
| How has it changed over time? | Line | Area only if a single series and volume matters |
| How do categories compare? | Horizontal bar, sorted by value | Sorting is mandatory |
| What is the composition? | Stacked bar, or 100% stacked | Pie only for 2 to 3 slices |
| How did we get from A to B? | Waterfall | Excellent for contribution analysis |
| How are two variables related? | Scatter | Add a trend line only when justified |
| Where are the concentrations? | Heatmap | Strong preattentive pull; use for exceptions |
| How does the distribution look? | Histogram or box plot | Prevents average-hides-everything errors |
| What is the pipeline conversion? | Funnel | Only for genuinely sequential stages |
| Which specific records? | Table | The correct answer far more often than assumed |

Anti-patterns: 3D anything, dual y-axes, pies with more than 3 slices, donut charts used as KPIs, truncated y-axes on bar charts, rainbow categorical palettes, gauges that consume large area to encode one value.

`DASH-11` (the table is underrated): when the question is "which ones", a sorted table with a well-chosen default sort beats every chart. Machine-generated dashboards systematically under-use tables because tables look less impressive.

`DASH-19` (trend words require trend data): a metric supplied only as point-in-time values renders as those values — a big number with context, or endpoints shown as a delta ("68% → 41% week-over-week"). No rendered or assistive copy states a direction ("rising", "trending down") unless a series is supplied; direction narrated beyond the data is an unsupported claim (`HON-01`). When only two endpoints exist, show both endpoints; do not draw a curve through them. The chart absence is usually the tell: where a shipped product would place a chart it has data for, a build without that data must show the values it has, not narrate the shape it imagines. Chrome makes capability claims too: a time-range or window selector promises that the data varies with the selection — render one only when a supplied series backs it, and label a point-in-time view as a snapshot instead.

## Module anatomy

```
+--------------------------------------------------+
| Title (the question or the object)      [actions] |   <- heading, 16px/600
| Context line: window, comparison basis            |   <- caption, 12px, secondary
|                                                   |
| PRIMARY CONTENT                                   |   <- the value or the visual
|                                                   |
| Supporting detail or legend                       |   <- caption, secondary
+--------------------------------------------------+
```

`COMP-05` (module consistency): all modules in a view use the same padding, radius, border treatment, title position, title style. Variation in module chrome must encode a real difference, never decoration.

`DASH-12` (legend placement): place legends adjacent to the data they label, or label series directly on the chart. A distant legend forces a lookup on every read.

`DASH-13` (axis discipline): label axes with units. Start bar chart value axes at zero. Reduce gridlines to the minimum needed. Remove axis lines that carry no information.

## Layout, grid, and density

`COMP-08` (declare the grid): 12-column grid, defined gutter, max content width. Module widths expressed in columns. Arbitrary pixel widths produce misalignment that reads as amateur.

Working defaults for a desktop analytics view:

| Property | Value |
| --- | --- |
| Target viewport | 1440 x 900 |
| Max content width | 1440px, or 1600px for dense operational views |
| Columns | 12 |
| Gutter | 24px |
| Page padding | 32px |
| Intra-zone gap | 16 to 24px |
| Inter-zone gap | 40 to 48px |
| Module min height | 120px for metric blocks, 280px for charts |

`DASH-14` (module size from tier): module footprint is assigned from tier, not from what fits. A P0 module spans more columns than any P1 module. Equal-size modules across differing tiers is Grid Tyranny (A4).

Tier span map:

| Tier | Columns (of 12) | Height |
| --- | --- | --- |
| P0 | 6 to 12 | Tall |
| P1 | 4 to 6 | Standard |
| P2 | 3 to 4 | Standard or short |
| P3 | 12, collapsed | Collapsed by default |

`COMP-06` (whitespace budget): 30 to 40 percent of viewport as intentional negative space in strategic/analytical views, 20 to 30 percent in operational views. Below 20 percent, comprehension speed drops sharply. Above 50 percent in an operational view, the user scrolls for constantly-needed information.

`COMP-09` (priority holds across breakpoints): define priority order once. At narrower widths, modules reflow in tier order; P2/P3 modules may collapse behind disclosure. Never let a responsive stack reorder the argument.

## States: the non-negotiable set

`STATE-01`: every data region specifies all of these.

| State | Requirement |
| --- | --- |
| **Loading** | Skeleton matching final layout, no shift. Never a full-page spinner for a partially loadable view |
| **Empty, first use** | Explain what will appear here and give the action that produces it |
| **Empty, filtered** | Say which filter excluded everything and offer to clear it. Never the same as first-use empty |
| **Partial** | Show what loaded, mark what failed, allow retry of only the failed part |
| **Stale** | Show last-updated time and a refresh affordance when data exceeds its freshness budget |
| **Error** | State what failed, whether it's retryable, and what the user can do. Never a raw error code alone |
| **No permission** | Explain access is restricted and how to request it. Do not silently hide (causes distrust of totals) |
| **Too much data** | Define the cap, communicate truncation, offer filtering or export |

`STATE-04` (extremes): verify each region against zero, one, many, very many, negative, null, and very long strings. A 60-character customer name must not break a card.

`STATE-05` (freshness budget): declare expected freshness per data source, design the stale state against it. Operational views need this most.

## Interaction model

`DASH-15` (drill paths follow real hierarchies): Time: year, quarter, month, week, day. Geography: region, country, city. Organization: department, team, individual. Product: category, line, SKU. Don't invent drill paths absent from the user's mental model.

`DASH-16` (cross-filtering must be visible): if clicking one module filters others, the effect must be immediately obvious and reversible with one action. Invisible cross-filtering is a leading cause of misread dashboards.

`DASH-17` (preserve context on drill): a drill-down keeps parent context visible via breadcrumb or persistent header; returning restores prior state.

`DASH-18` (tooltips carry precision, not meaning): use tooltips for exact values and definitions only. Never place information required to answer a top-ranked question behind a hover (hover doesn't exist on touch, invisible to scanning).

## Layout spec pattern (abbreviated worked shape)

Use this shape when specifying a dashboard: declare view, type, viewport, grid, refresh/freshness budget, then each layer with its modules (id, tier, span), then actions, then states, then breakpoints.

```
VIEW: Support Operations
TYPE: Operational
VIEWPORT: 1440 x 900, min supported 1280
GRID: 12 col, 24px gutter, 32px page padding
REFRESH: 60s auto, manual refresh available, freshness budget 120s

LAYER 1 - ORIENT (P4, top strip, 48px)
  Title | Queue scope selector | Time window | Freshness | Global filter chips

LAYER 2 - STATE (Zone A, "Needs attention now")
  A1  SLA breach count + time to next breach   P0   span 5   tall
  A2  Breaching queue list, max 5 rows         P1   span 7   tall
      Row action: "Reassign staff" (tertiary, S4)

LAYER 3 - EXPLAIN (Zone B, "Current load")
  B1  Backlog trend, 4h line                   P1   span 6
  B2  Open tickets by priority, sorted bar     P1   span 3
  B3  First response time vs target            P1   span 3

LAYER 4 - ACT (Zone C, "Capacity" / Zone D, "Investigate")
  C1  Agents online + utilization table        P2   span 8
  C2  Volume by channel, sorted bar            P2   span 4
  D1  Ticket search + event log, collapsed     P3   span 12

ACTIONS
  Primary: "Reassign staff" filled accent, header right
  Secondary: "Export CSV" outlined, left of primary
  Overflow: Configure alerts, Manage SLAs, Notification settings

STATES
  All zones: skeleton on load matching final spans
  Zone A empty: "No queues at risk. All 12 queues within SLA." + green check
  Zone A error: inline, retry affordance, other zones unaffected
  Stale > 120s: amber freshness indicator + manual refresh emphasized

BREAKPOINTS
  <1280: C2 moves below C1; D1 stays collapsed
  <1024: single column in tier order A1, A2, B1, B2, B3, C1, C2, D1
```
