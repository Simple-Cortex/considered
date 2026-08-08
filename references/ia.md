# Information Architecture

Purpose: operating rules for information architecture (grouping, naming, navigation, disclosure, filters) prior to any dashboard layout work.

Contents: object model, question inventory, grouping procedure, grouping tests, naming rules, navigation layers, progressive disclosure, filter architecture, zone map.

## Core law

`IA-02`: Group by the question the human is asking and the object they act on. Never group by chart type, component type, data source, or technical domain. Machine clustering axis (rendering similarity) diverges from human clustering axis (meaning); this divergence is the primary cause of "hard to think with" dashboards.

Illustration: grouping by type ("Numbers", "Charts", "Tables") forces the user to gather scattered pieces to answer one question. Grouping by question (e.g. "Are we meeting commitments?") lets one zone answer one question completely.

## Object model

For each object record: Name (singular/plural, user's word), Attributes (ranked by importance), States (lifecycle values), Relationships (contains/belongs to/references), Actions (what and by whom), Identity (how a user recognizes an instance at a glance).

Worked example:

| Object | Attributes (ranked) | States | Relationships | Actions | Identity |
| --- | --- | --- | --- | --- | --- |
| Ticket | priority, age, requester, subject, assignee, channel | new, open, pending, resolved, closed | belongs to Queue, assigned to Agent | assign, reply, escalate, merge, archive | subject + requester |
| Queue | backlog, SLA target, staffed count, breach count | healthy, at risk, breaching | contains Tickets, staffed by Agents | reassign staff, configure SLA | name |
| Agent | status, active tickets, utilization | online, away, offline | works Queues, owns Tickets | assign, message | name + avatar |

`IA-04` (identity first): identity attribute renders first and most prominently in any list/table/card. Leading with an ID or timestamp instead of the human-recognizable name is a `CONT-01` schema leak.

`IA-05` (state visibility): every object with a lifecycle displays current state wherever it appears, with consistent treatment across the product.

## Question inventory (Artifact A2)

Write questions in the user's voice, then rank them.

- Phrase as a question a human would say out loud.
- One question per line, no compound questions.
- Include the decision that follows the answer.
- Aim for 6 to 12 per view. More than 12 means it's more than one view.

Columns: Rank, Question, Decision it drives, Frequency, Tier it implies. Rank drives tier assignment; frequency drives disclosure decisions (see below). This table is the connective tissue between IA and hierarchy.

## Grouping procedure (run explicitly, do not skip to layout)

```
STEP 1  List every candidate element (metric, chart, table, control, action).

STEP 2  Tag each element with the question ID(s) it helps answer.
        Any element with no question tag is DELETED. [IA-01]

STEP 3  Cluster elements by shared question tag.
        Elements answering the same question form a candidate zone.

STEP 4  For elements answering multiple questions, assign to the zone of the
        HIGHEST-RANKED question. Do not duplicate across zones. [IA-07]

STEP 5  Merge candidate zones that share an object AND are consumed in the
        same glance. Split any zone holding more than 7 elements. [IA-06]

STEP 6  Name each zone. The name should be the user's question, or the object,
        or the decision. Never the chart type or the data source. [IA-03]

STEP 7  Order the zones by the rank of their highest-ranked question.

STEP 8  For each zone, order elements: answer first, then explanation,
        then detail.

STEP 9  Run the grouping tests below. Fix violations. Repeat from step 3.
```

`IA-07` (no duplication): an element appears in exactly one zone. Belonging in two means the zones are wrong, or it's actually two elements at different aggregation levels.

`IA-06` (group size): 3 to 7 elements per zone. Below 3, consider merging. Above 7, subdivide. This is a working-memory constraint.

## Grouping tests

**G1. Glance test (`IA-08`).** Two elements belong together if a user would look at both within one 3-second glance to answer one question. If answering requires only one of them, they're in different groups.

**G2. Heading test (`IA-09`).** Write a heading for each zone using only the user's vocabulary, as a question or decision.
Pass: natural and specific, e.g. "Are we meeting commitments?", "Queue health", "Needs your attention".
Fail (banned headings): "Charts", "Metrics", "KPIs", "Overview", "Other", "Salesforce data", "Analytics". If you cannot name a group in the user's language, it isn't a real concept in the user's mind. "Overview" and "Other" are the clearest tells of failed grouping.

**G3. Move test (`IA-10`).** For each element, ask whether moving it to a neighboring zone would change the meaning of either zone. Pass: yes, clearly. Fail: no difference; zones are arbitrary and should be merged or redefined.

**G4. Card-sort simulation (`IA-11`).** Independently re-derive the grouping from the element list alone, twice, using two different framings (e.g. by object and by workflow stage). Pass: convergent groupings, or a clear reason to prefer one. Fail: wildly divergent groupings mean the domain has no natural structure at this level; use a different organizing axis.

**G5. Orphan test (`IA-01`).** Every element traces to a question. Zero orphans.

## Naming and labeling

`CONT-01`: use the user's vocabulary. Never expose schema names, internal codenames, team names.

| Avoid | Use |
| --- | --- |
| `usr_cnt_30d` | Active users, last 30 days |
| Avg. Sess. Dur. | Average session length |
| Entity | Customer, project, ticket |
| Utilization Coefficient | Agent load |
| MTTR | Time to resolve (define on hover) |

`CONT-02` (define once, reachable everywhere): every metric has one definition, exposed via an info affordance next to the label. Two screens must never define the same metric differently. Definition drift is the most common reason people stop trusting a dashboard.

`CONT-03` (label consistency): same concept, same word everywhere. Don't alternate "client"/"customer"/"account" for one object.

`CONT-04` (units and precision): always state units. Round to the precision that supports the decision, not the precision the database provides ($1,284,392.17 -> $1.28M on a summary; full precision only in detail/export). Excess precision slows comparison and implies false accuracy.

`CONT-05` (time framing): every time-sensitive value states its window and freshness. "Last 30 days" and "Updated 2 min ago" are content, not chrome.

## Navigation architecture

Four distinct layers. Collapsing them into one control is the source of most navigation confusion.

| Layer | Purpose | Typical control | Constraints |
| --- | --- | --- | --- |
| **Global** | Which product area am I in | Sidebar, top bar | 5 to 7 items, grouped if more |
| **Section** | Which view within this area | Tabs, segmented control | 3 to 7 parallel views sharing context |
| **Contextual** | Which slice of data | Filters, date range, segment selector | Scope must be visible |
| **Detail** | Which specific object | Drill-down, row click, breadcrumb | Always provide the path back |

`IA-12` (depth limit): max 3 levels of navigation hierarchy. A needed fourth level means the structure is wrong; use in-page tabs or filtering instead.

`IA-13` (group size in nav): max 7 items per nav group. Beyond that, introduce labeled groups, not a scrollbar.

`IA-14` (never mix layers): no filter in global nav; no product area in a tab set alongside data views. Mixing layers means the user can't predict what a click will do.

`IA-15` (path back): every drill-down provides an explicit return path, preserving prior state (filters, scroll position). Losing filter state on back navigation is severity S2.

`IA-16` (dashboard is not the app): the dashboard is one destination in global nav, not the container for the entire product. Full-workflow actions live in their own area and are linked from the dashboard, not embedded.

## Progressive disclosure

Decision: what's on the default view, what's one interaction away.

`IA-17` (default view rule): default view answers the top-ranked questions completely. It does not partially answer all questions.

Disclosure mechanism selection:

| Additional content | Mechanism | Why |
| --- | --- | --- |
| A precise value behind a visual | Tooltip on hover and focus | Zero cost, no layout change |
| Supporting detail for one item | Expand in place, accordion | Preserves context and position |
| A full record for one object | Side drawer | Keeps list visible for comparison |
| A different analytical slice | Tab or segmented control | Parallel, equal-weight views |
| A deeper level of the same data | Drill-down with breadcrumb | Explicit hierarchy traversal |
| A different task entirely | Separate page | Not disclosure; this is navigation |
| Advanced or rare options | Overflow menu, settings | Keeps default surface calm |

`IA-18` (three-level limit): if a routinely needed answer requires traversing 3+ disclosure levels, restructure.

`IA-19` (disclosure must be signposted): a collapsed region shows what it contains and, where cheap, how much. "Agent activity (24 events)" beats a bare chevron. Unlabeled disclosure is functionally invisible.

`IA-20` (disclosure is not navigation): expanding content in place is disclosure; changing context is navigation. Don't use an accordion to switch tasks, don't use a page load to reveal one extra value.

## Filter architecture

`IA-21` (scope must be visible): unambiguous which region a filter affects. Global filters sit above all content, visually attached to the whole view. Local filters sit inside the zone they affect. A floating filter with no containment is ambiguous.

`IA-22` (filter count): max 3 to 5 global filters in the default view. Additional filters go behind "More filters" with a count badge.

`IA-23` (active state is content): active filters shown as removable chips, plus a single "Clear all". A user must never wonder why a number looks wrong because a forgotten filter is still applied.

`IA-24` (defaults are a design decision): default filter state answers the top-ranked question for the most common user. Don't default to "all time"/"all segments" if that produces a meaningless first read.

`IA-25` (state in the URL): filter and view state belongs in the URL so views are shareable and restorable. This is an IA property, not an engineering detail.

## Artifact A4: Zone Map

Columns: Zone, Heading (user's words), Questions, Elements, Object, Order rationale.

| Zone | Heading (user's words) | Questions | Elements | Object | Order rationale |
| --- | --- | --- | --- | --- | --- |
| A | Needs attention now | Q1 | SLA breach count, breaching queue list, time to breach | Queue | Highest ranked question, contains the P0 |
| B | Current load | Q2, Q3 | Open tickets, backlog trend, tickets by priority | Ticket | Explains why zone A is happening |
| C | Capacity | Q3, Q5 | Agents online, utilization table, channel volume | Agent | Determines the response to zones A and B |
| D | Investigate | Q6 | Ticket search, event log | Ticket | Deferred, collapsed by default |

Validation checks:

- Every zone heading passes the heading test (`IA-09`).
- Zone element counts are between 3 and 7 (`IA-06`).
- No element appears in two zones (`IA-07`).
- Zone order matches question rank order.
- Every question in A2 is covered by at least one zone.
- Every element traces to at least one question (`IA-01`).
