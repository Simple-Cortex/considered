# Changelog policy

The public `CHANGELOG.md` tells users what changed in the product. It is not a
lab notebook, a research log, or a record of how the work got done.

Detailed development history — methodology, evaluation internals, tooling,
vendors, dead ends — lives in the private work repository, not here.

## What a public entry is

One line per user-visible change, grouped under Keep a Changelog headings
(`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`). Write for
someone deciding whether to upgrade.

A good entry answers: *what can I now do, or what will now behave differently?*

```
### Added
- `harden` command: checks a built surface against the accessibility and
  state-coverage rules before review.

### Changed
- Install moves to the Agent Skills CLI: `npx skills@latest add <owner>/considered`.

### Fixed
- `roll` no longer exhausts the deck when a reroll reuses a prior key.
```

## What never goes in the public changelog

Each of these belongs in the private repo instead.

- **Named third-party services and data sources.** Design-reference libraries,
  scraping and SEO tools, model bridges, analytics vendors. Naming a service
  you used to build or validate the product invites questions about licensing
  and terms of use, and tells competitors your supply chain.
- **Third-party brands and products used as comparisons or benchmarks.**
  Publishing "we beat *<company>*" is a claim about someone else's work made
  without their participation. Keep comparative results private, or publish
  only the aggregate with sources unnamed.
- **Evaluation and benchmarking internals.** Judge lineups, scoring thresholds,
  seeds, blinding procedures, panel composition, sample sizes.
- **Methodology defects and their fixes.** "The crop was wrong," "the footer
  leaked," "past results were conservative." These are essential to record
  privately and corrosive to publish — read in isolation they undermine
  confidence in results they were meant to strengthen.
- **Which models were used to build, judge, or refine the work**, unless the
  model *is* the user-facing feature.
- **Internal process detail.** Agent orchestration, subagent limits, session
  mechanics, who reviewed what, how long something took.
- **Anything about a specific person**, including contributors.

## Test before publishing an entry

Ask, in order:

1. **Does it change what a user can do?** If no, it does not belong here.
2. **Does it name a company, product, service, model, or person?** If yes,
   remove the name or move the entry to the private log.
3. **Would I be comfortable with the named party reading this line?** If no,
   it is not publishable regardless of accuracy.
4. **Does it reveal how we measure ourselves?** Results may be summarized;
   the machinery stays private.

## Where detail belongs

- **Private work repo** — full changelog, `DEVLOG.md`, evaluation records,
  methodology notes, vendor and tooling decisions.
- **Public repo** — this changelog, the README, the skill itself, and any
  evidence deliberately prepared for publication.

When in doubt, write the detailed entry privately first, then decide what
single sentence of it a user actually needs.
