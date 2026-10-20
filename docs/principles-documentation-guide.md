# Principles Documentation Guide

How to record, in `STRUCTURE.md`'s "Principles Applied" section, the principles that drove your decisions. It exists for traceability and accountability.

The review packet includes `STRUCTURE.md` but deliberately excludes build rationale. This section is therefore the only place a reviewer sees which principles shaped the surface; do not put it anywhere else.

## Why Document Principles

**Traceability.** A reviewer (or future you) can trace a design decision back to a specific principle instead of reading the output as a black box.

**Accountability.** Writing down "I applied PERS-07 because..." shows you consulted the index rather than guessing.

Documentation never substitutes for compliance. The reviewer judges the built surface independently.

## What to Document

Document only the principles that drove a decision on this surface. Do not list every principle in the active set, and do not pad the section with principles the surface merely happens to satisfy.

For each one, record three things:

1. **Principle ID** - the `id` from `assets/principles-index.json` (e.g., `PERS-07`, `HIER-13`)
2. **Decision context** - what you were deciding when you applied it
3. **How it was applied** - the specific treatment you chose, not a restatement of the principle

The third item matters most. "I followed HIER-13" is useless. "P1 headings 24px/600 against P2 body 16px/400" is auditable.

## Finding the Right Principles

Read `assets/principles-index.json` once and keep entries whose `modes` contain the surface's mode or `*`. At each decision, narrow by `decision`, then by `tiers` and `surfaces` (absent means all). The `decision` values are: form, state, density, navigation, action-placement, emphasis, disclosure, narrative, hierarchy, proof, objection-handling, pricing.

## Good vs Bad Examples

### Bad: Restating the principle

```
### Pricing
- PERS-07: Pricing exposes the real decision.
```

### Good: Showing the specific treatment

```
### Pricing Section
- PERS-07 (pricing exposes the real decision): Led the comparison with seat limit and renewal terms, the criteria that change plan choice. Stated annual renewal price and cancellation terms beside each tier; removed the unevidenced "Most popular" badge.
- PERS-09 (the price is the P0 value): Set each tier price in display type above its label; every tier has a working "Start" action.
```

### Bad: Vague hierarchy claims

```
### Hierarchy
- HIER-13: Tiers are distinct.
```

### Good: Showing the measurement

```
### Hierarchy
- HIER-13 (adjacent tiers separate decisively): P1 section headings 24px/600 against P2 body 16px/400, a full scale step apart in size and weight.
```

## If You Did Not Consult the Index

Say so in the section, with the reason:

```
## Principles Applied

I did not consult the principles index for this surface. The composition was driven by [reason: e.g., "an existing design system", "a client requirement that overrides the index"].
```

A missing or blank section is what the reviewer flags: it hides the decisions and leaves the work without traceability.

## Summary

- Record only the principles that drove a decision on this surface
- Give ID, decision context, and the specific treatment
- Put it in `STRUCTURE.md` "Principles Applied" and nowhere else
- The reviewer verifies compliance independently; this section is for traceability
