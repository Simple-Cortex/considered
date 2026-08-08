# Structure

**Surface id:** `<kebab-case id>`
**Route:** `structure x <mode> x <page|flow|component|existing>`
**Frame:** `.considered/<surface-id>/FRAME.md`
**Target:** `<workspace-relative target or specification-only>`
**Status:** ready | blocked

## Assigned hand

Use `scripts/roll.mjs` when available. Record the assigned hand exactly. On the all-host fallback, record the single unbiased selection method and do not reselect by preference.

| Layer | Assigned id | Name | Law carried into the surface |
| --- | --- | --- | --- |
| Organizing axis |  |  |  |
| Depth strategy |  |  |  |
| Framing |  |  |  |
| Direction |  |  |  |

**Reproduction key and generation:** `cns-________/0`
**Roll method:** script | host-fallback
**Rejected reroll history:**

## Zone map

| Zone | Heading in the user's words | Questions | Elements | Object | Order rationale |
| --- | --- | --- | --- | --- | --- |
| A |  | Q1 |  |  |  |

## Hierarchy table

P0 is the only focal answer. P1 drives the main decision. P2 explains P0 or P1. P3 is deferred reference. P4 is chrome, filters, navigation, and utility. P4 is not a competing content element.

| ID | Element | Answers question | Tier | Devices and points | Position or disclosure | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| E1 |  | Q1 | P0 |  |  | why it beats rivals |
| E2 |  | Q2 | P1 |  |  |  |
| E3 |  | Q3 | P1 |  |  |  |
| E4 |  | Q4 | P2 |  |  |  |
| C1 |  | chrome | P4 | 0 | peripheral |  |

## Action table

| ID | Action | Scope | Tier | Treatment | Risk | Label | Disabled rule | Loading | Placement |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A1 |  | S1 | primary | filled accent | safe |  |  |  |  |

## Layout and state plan

**Target viewport or medium:**
**Reading order and P0 placement:**
**Default disclosure:**
**Responsive priority behavior:**
**Data regions and freshness budgets:**
**Required empty, loading, partial, stale, error, no-permission, and overflow behavior:**

## Deleted or deferred

| Item | Deleted or deferred | Why |
| --- | --- | --- |
|  |  |  |

## Canonical contract

The same raw contract must be in `.considered/<surface-id>/CONTRACT.md` and, when safe, in a comment wrapper at the top of the built artifact. Use `assets/templates/contract-block.md` without changing field names.

```text
CONSIDERED-CONTRACT v1
THESIS:     <one organizing idea and the default it refuses>
DECISION:   <role> decides <what> within <time>; being wrong costs <consequence>
MODE:       <one mode>
QUESTIONS:
  Q1 [P0] <question> -> <ZoneId>
ZONES:
  <ZoneId> "<user-language heading>" :: Q1 :: E1,E2,E3
HIERARCHY:
  P0 <element> - <why it beats every rival>
  P1 <element>, <element>, <element>
  P2 <element>, <element>
  P3 <deferred element>
  P4 chrome: <nav>, <filters>, <utility>
ACTIONS:
  S1 primary "<Verb object>" safe
DELETED:    <item> - <reason>
ROLL:       <structure ids>, <direction id>, cns-________/0
```

## Structure exit

- [ ] Every zone answers ranked questions and has 3 to 7 distinct elements.
- [ ] Exactly one P0 and three to five P1 elements are declared.
- [ ] P4 chrome is explicitly demoted.
- [ ] Every action has scope, tier, risk, and state behavior.
- [ ] Contract is valid in its wrapper or sidecar form.
