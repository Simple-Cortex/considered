# States Reference

Purpose: specify every state a data region or an action must define, plus concrete copy patterns, so nothing ships in "happy path only" form.

Contents: copy patterns, the 8 required data states, extremes, freshness budget, action states, disable rule.

---

## Copy patterns (write real copy, not placeholders)

Generic state copy is the most common failure. Every state message must name the specific thing involved: the filter, the failed part, the permission, the cap. Use these patterns as defaults.

| State | Bad (generic) | Good (specific) |
| --- | --- | --- |
| Empty, first use | "No data" | "No campaigns yet. Create your first campaign to see performance here." + action |
| Empty, filtered | "No results" | "No tickets match Priority: Urgent + Channel: Email. Clear filters to see all 42 open tickets." + clear action |
| Partial | "Error loading" | "Backlog trend loaded. Agent utilization failed to load. Retry utilization." |
| Stale | "Data may be outdated" | "Updated 6 min ago (budget: 2 min). Refresh now." |
| Error | "Something went wrong" | "Couldn't load SLA breaches (timeout). Retry, or view last known values from 10:42 AM." |
| No permission | "Access denied" | "You don't have access to Finance queues. Request access from your workspace admin." |
| Too much data | "Too many results" | "Showing first 500 of 12,400 rows. Filter to narrow, or export the full set." |

Rule of thumb: if the message would read identically for any two different data regions in the product, rewrite it. State copy that survives a global find-replace of the noun is generic and fails.

---

## `STATE-01`: the required set

Every data region specifies all 8. A region with only a populated/correct state is a specification defect.

| State | Requirement |
| --- | --- |
| **Loading** | Skeleton matching final layout, no shift. Never a full-page spinner for a partially loadable view |
| **Empty, first use** | Explain what will appear here, give the action that produces it |
| **Empty, filtered** | Name the filter(s) that excluded everything, offer to clear it. Never identical to first-use empty |
| **Partial** | Show what loaded, mark what failed, allow retry of only the failed part |
| **Stale** | Show last-updated time and a refresh affordance when data exceeds its freshness budget |
| **Error** | State what failed, whether it's retryable, what the user can do. Never a raw error code alone |
| **No permission** | Explain access is restricted and how to request it. Never silently hide, which causes distrust of totals |
| **Too much data** | Define the cap, communicate truncation, offer filtering or export |

Two states are the most frequently confused or skipped, watch these specifically:
- Empty-filtered vs empty-first-use: these must read as visibly different states. Reusing first-use copy when a filter is the actual cause is an `S2` defect (see critique.md severity model).
- No-permission vs empty: silently omitting restricted rows so a region just looks empty is worse than showing the restriction, because it produces totals the user cannot trust.

## `STATE-04`: extremes

Verify every region against all seven:

| Extreme | What to check |
| --- | --- |
| Zero | Renders as the empty state, not a broken "0" or blank chart axis |
| One | Singular grammar, layout doesn't assume multiple rows/series |
| Many | Normal working range, still readable |
| Very many | Triggers too-much-data handling, not silent truncation |
| Negative | Correct sign, correct color mapping (see `DASH-09`, direction is not goodness) |
| Null | Explicit "not available" treatment, never rendered as 0 or blank |
| Very long strings | A 60-character name must not break a card, table row, or label |

## `STATE-05`: freshness budget

Declare an expected freshness per data source (e.g. "budget: 2 min" for live queue data, "budget: 24h" for a daily batch metric). Design the stale state against that specific budget, not a generic threshold. Operational views need this most because staleness there is itself a decision-relevant fact.

The budget is a design input, not display copy. When the brief supplies no freshness fact, record the chosen budget as an assumption in `FRAME.md` and design the stale behavior against it — but never render an invented budget value or refresh promise in the UI. A fabricated "budget: 2m" label is an unsupported product claim, exactly like an invented metric or customer. The values in this section are examples for the artifact record, not content. This is the general rule `HON-02` applied to freshness; the full supplied-fact boundary is in `craft.md` §4.7.

---

## Action states (`ACT-16`, `ACT-17`, `ACT-18`)

**`ACT-16`:** every action must define all of these. An action with only a default state is a specification defect.

| State | Requirement |
| --- | --- |
| Default | Meets contrast requirements, at rest |
| Hover | Visible change in background or border. Pointer devices only |
| Focus | Visible focus ring, minimum 3:1 contrast against adjacent colors, never removed |
| Active / pressed | Distinct from hover, gives immediate feedback |
| Loading | In-place, preserves button width, disables re-entry, announces to assistive tech |
| Disabled | Only when the reason is discoverable (see `ACT-17` below) |
| Success | Optional. Brief in-place confirmation for low-consequence actions |
| Error | Inline message adjacent to the action, describing cause and remedy |

**`ACT-17` (never disable silently):** a disabled control with no explanation is a dead end. Choose one:
1. Keep the control enabled and explain the failure on activation (preferred in most cases).
2. Disable it and attach a tooltip or adjacent text explaining exactly what is required to enable it.

Never disable a control whose enabling condition is not visible on the same screen.

**`ACT-18` (optimistic vs pessimistic):** for trivially reversible actions, apply optimistically with an undo affordance. For consequential actions, show a pending state and confirm on server response. Never show an optimistic success for an irreversible action.

---

## Data state and action state interact

A region's data state constrains which action states are valid on it:

| Data state | Action state it forces |
| --- | --- |
| Loading | Related submit/refresh actions show Loading or are disabled with a visible reason |
| Error | Retry action must be present and enabled, never disabled |
| No permission | Actions scoped to the hidden data are removed, not shown disabled |
| Too much data | Export/filter actions stay enabled, they are the way out |

## Quick audit checklist

- [ ] All 8 `STATE-01` states specified per data region, with copy that names the specific noun (filter, source, permission)
- [ ] Empty-filtered copy is visibly different from empty-first-use copy
- [ ] All 7 `STATE-04` extremes tested against the actual layout, not just described
- [ ] Freshness budget declared per data source, stale state references that budget's number
- [ ] All 8 `ACT-16` action states defined per action, including loading and error
- [ ] No control disabled without a visible, same-screen reason (`ACT-17`)
- [ ] Irreversible actions never show optimistic success (`ACT-18`)

Note on rule numbering: source material for this unit defines `STATE-01`, `STATE-04`, and `STATE-05` only. `STATE-02`, `STATE-03`, and `STATE-06` are not defined in the reviewed course content and are not fabricated here.
