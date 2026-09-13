# Considered

A portable design-reasoning skill for AI coding agents.

Considered turns the chain a senior product designer uses into durable artifacts:

```text
human decision -> questions -> zones -> hierarchy -> action scopes -> composition -> independent review
```

It is for consequential UI work where a wrong information structure, focal point, or action treatment changes what a person can decide or do. It intentionally skips isolated copy, spacing, and token fixes with no structural impact.

---

## Why this exists

Ask a capable coding agent for a dashboard and you get one: a grid of cards, a
chart or two, a sidebar. It renders, it is styled, and it is plausible. It is
also, very often, not *designed* — nobody decided what question the surface
answers, which value is worth the largest type, which action is dangerous
enough to need distance, or what the empty state should teach. The agent
produced a layout; design is the reasoning that should have preceded it.

That reasoning is exactly what does not survive an agent conversation. It
happens implicitly, in one pass, inside a context window that will be gone
tomorrow, and it is graded by the same model that produced it. Three failure
modes follow:

1. **Structure by accident.** Information architecture emerges from generation
   order, not from the user's decision. The most important value on the screen
   is whatever happened to be first in the data.
2. **Template gravity.** Left to its defaults, a model reaches for the same
   component set every time. The output is competent, interchangeable, and
   unconsidered — the visual equivalent of filler prose.
3. **Self-certified quality.** The agent that built the surface declares it
   done. No fresh pair of eyes, no evidence, no gate a bad build can fail.

Considered exists to make an agent do what a senior designer does instead:
frame the decision before touching layout, commit to a structure and defend
it, express a visual direction deliberately, and then hand frozen evidence to
an independent reviewer who can block the ship. Every step leaves a durable
artifact, so the reasoning outlives the chat.

## What using it looks like

You install one skill directory and ask your agent to design the way you would
brief a designer — "frame the billing settings page," "critique this
dashboard." The skill routes the request through phases, and each phase must
write its artifact before the next may start:

| Phase | What the agent does | Durable output |
| --- | --- | --- |
| `init` | Records what the product is, for whom, and what it must never do | `.considered/PRODUCT.md` |
| `frame` | Names the human decision the surface serves, inventories the questions it must answer, ranks them | `FRAME.md` |
| `structure` | Receives an assigned structural hand (see below), maps zones, hierarchy tiers, and action scopes, and signs a contract | `STRUCTURE.md`, `CONTRACT.md` |
| `compose` | Builds the surface inside that contract | the target surface or a portable spec |
| `critique` | Freezes an evidence packet; a fresh, separate reviewer scores it and issues a verdict the gate enforces | `REVIEW-PACKET.md`, `REVIEW.json` |
| `subtract` | Removal-only revision — the pass where good work usually gets better | revised surface |
| `harden` | Real states: empty, loading, error, overflowing, interrupted | state coverage |
| `document` | A portable record of what was decided and why | `DESIGN.md` |

Two properties distinguish this from prompting an agent to "design carefully":

**The direction is assigned, not chosen.** At `structure`, a deterministic
roll deals three structure laws and one visual direction from curated decks.
The agent does not get to pick its comfort direction, and it does not get to
re-roll a hand it finds difficult — it must express the dealt direction
*within* the genre's conventions and the client's constraints. This is the
mechanism that breaks template gravity: constraint forces consideration.

**The reviewer is not the builder.** The critique phase hands a frozen packet
— screenshots, sanitized source, the contract, lint reports — to a fresh
context that never saw the build rationale. The reviewer scores rubric
dimensions, hunts unsupported claims, and issues Ship / Revise / Redesign.
The gate is mechanical: without an independent `REVIEW.json`, there is no
pass. A build cannot certify itself.

## Who it is for, and how it helps

**Designers working with (or around) AI agents.** Your judgment is the
scarcest input in an agent workflow, and today it gets spent pixel-polishing
agent output after the fact. Considered moves that leverage upstream: the
artifacts are the medium. You review a `FRAME.md` that says what decision the
surface serves before a line of layout exists; you audit a `CONTRACT.md` that
commits to hierarchy tiers and action scopes; you read a `REVIEW.json` from a
reviewer that was never shown the builder's excuses. When the output is wrong,
the artifact chain shows *where the reasoning went wrong* — a mis-framed
decision, a violated tier, an action scope that never got a scope — instead of
leaving you to reverse-engineer a finished screen. And because the corpus is
plain JSON and Markdown, a design team can extend it: house rules ride
alongside the built-in ones and are enforced the same way.

**Developers who ship UI without a designer.** The skill carries the judgment
you would otherwise have to source: 190 rules distilled from design practice
and from measured failures, routed to the moment they apply — persuasion rules
when composing a pricing page, density rules in a data workspace, honesty
rules everywhere. You do not read the corpus; the workflow surfaces the
relevant slice at the relevant phase.

**Teams that need design quality to be checkable.** "Looks good to me" does
not survive an incident review. A gated Considered build ships with evidence:
what was decided, what was assigned, what the contract promised, what the
lint found, what an independent reviewer scored, and why the gate passed.
That record is diffable, auditable, and survives the departure of whoever
(or whatever) built it.

**What it will not do:** fix isolated copy, chase spacing tokens, or theme an
existing surface. If structure is not in question, this is the wrong tool —
by design.

## How the system is built

The implementation is deliberately small: zero-dependency Node ESM, plain
JSON and Markdown, no framework, no build step. Everything an agent needs to
act on is a file it can read.

```text
SKILL.md                     the router: one skill surface every host understands
skills/<command>/SKILL.md    eight trigger-only subskills for hosts that discover nested skills
references/                  the teaching layer: workflow.md (normative protocol),
                             ia.md, craft.md, dense.md, persuade.md, read.md, experience.md,
                             modes.md, states.md, actions.md, forms.md, hierarchy.md,
                             accessibility.md, failures.md, critique.md
assets/rules/                the rule corpus: contract, gate, guidance, render, source manifests
assets/decks/                the structure-law and direction decks the roll deals from
bin/ + scripts/              the CLI: roll, contract, lint, render, gate, validate, eval, install
evals/                       frozen scenarios, the matched-evaluation harness, blind-panel runner
.considered/                 (in your project) the artifact chain each surface accumulates
```

### The rule corpus

190 rules across five manifests, split by what can honestly enforce them:

- **Contract rules** validate `CONTRACT.md` mechanically — tiers, action
  scopes, roll provenance. High confidence, machine-checked.
- **Source rules** are heuristic regex leads — possible accessibility, state,
  and slop issues — explicitly labeled as reviewer leads, never verdicts.
- **Render rules** require attested evidence from a renderer or independent
  reviewer; a claim about pixels needs a witness.
- **Gate rules** define the thresholds a review must clear, in data
  (`assets/rules/gate.json`), not code.
- **Guidance rules** (110 of the 190) carry prose `check` fields, not
  executable checkers — "the price carries value-weight above its labels,"
  "sample data instantiates, never asserts." No command emits them, and that
  is deliberate: they surface by *routing*, in the reference catalog the
  workflow already sends the agent to at that phase. Turning them into a
  linter would mean faking a hundred checkers or dumping an undifferentiated
  list at build time — the "read every rule before designing" failure this
  skill exists to avoid.

Coverage is enforced in both directions by `npm run validate`: every rule in
a manifest must be taught in a reference file, and every rule id cited in a
reference must resolve to a manifest. A rule an agent cannot look up is a rule
it cannot act on, so the build fails instead.

The corpus is not static, and its growth is evidence-driven — rules are
minted from measured failures, not taste. The honesty family (`HON-01..06`)
was distilled from fabrication flags in the first matched evaluation; the
newest rules (`PERS-10`, `MODE-10`) were minted from convergent findings by
three independent blind judges and then validated by rebuilding the losing
surface under the new corpus and watching every judge move its rank up. The
full provenance of every mint is in `CHANGELOG.md`.

### The deterministic roll

`considered roll` hashes a key (surface id, mode, generation) into deck
indices and deals three structure laws plus one visual direction. Same key,
same hand — the assignment is reproducible and auditable (`ROLL-01` requires
its provenance recorded in the contract). Re-rolls under the same key exclude
previously dealt cards, so an agent cannot fish for an easy hand. Two rules
bound the roll's authority: the direction serves the genre (`MODE-09` — a
terminal aesthetic has no business on a signup form), and the direction must
survive a supplied brand register (`MODE-10` — constraint is not an excuse to
produce a template).

### The validation model

| Layer | What it can honestly establish | Gate treatment |
| --- | --- | --- |
| Contract lint | contract syntax, references, tiers, actions, roll provenance | high-confidence |
| Source lint | possible source-level accessibility, state, token, and slop issues | heuristic reviewer leads |
| Render evidence lint | measurements or observations attested by a renderer or independent reviewer | high-confidence attestation |
| Fresh reviewer | purpose, information architecture, hierarchy, visual quality, states, responsive behavior | mandatory |

Gate thresholds live in `assets/rules/gate.json`: finding limits per
severity, the minimum weighted score, the critical dimensions that cannot be
averaged away, and the recognized reviewer verdicts. A review whose verdict is
absent or unrecognized returns `REVIEW-REQUIRED` — the gate never infers a
verdict from a score, because that would let a build self-certify. A missing
screenshot never becomes a pass; the review is marked source-only with its
limitation.

## Evidence

The project holds itself to the standard it imposes: claims require matched
conditions, blind review, and published records — including the negative
findings.

### The matched evaluation: m0-001

Ten scenarios, both conditions built by the same model (`claude-sonnet-5`,
one invocation each), reviewed blind by three model reviewers per scenario
(`claude-opus`, `claude-fable-5`, `claude-sonnet-5`) on rebuilt, leak-checked
packets. Reviewers are models, not humans; with ten scenarios, treat every
number below as descriptive, not conclusive.

| Metric | Baseline | Considered | Comparison | Denominator |
| --- | ---: | ---: | ---: | --- |
| Blinded paired preference | 4 wins | 26 wins | rate 0.87 (0.50 neutral); 0 ties, 0 abstentions | 30 reviews |
| Scenario-level majority | 1 | 9 | considered took the reviewer majority in 9 of 10 | 10 scenarios |
| Mean weighted score (of 140) | 106.8 | 125.8 | +19.0 mean, +17 median | 30 scored pairs |
| Floor pass D1 purpose | 27/30 | 30/30 | | scored outputs |
| Floor pass D2 information architecture | 26/30 | 29/30 | | scored outputs |
| Floor pass D3 hierarchy | 29/30 | 30/30 | | scored outputs |
| Floor pass D7 accessibility | 14/30 | 30/30 | the largest gap on any dimension | scored outputs |
| S1 blocker-free outputs | 25/30 | 29/30 | 7 raw S1 findings vs 1 | reviewed outputs |
| Reviewer gate: Ship / Revise / Redesign | 2 / 28 / 0 | 16 / 14 / 0 | | reviewed outputs |
| Unsupported-claim flags | 5 (0.17) | **11 (0.37)** | **worse for Considered** | reviewed outputs |
| Evidence-complete runs | 10/10 | 10/10 | | successful runs |

The negative finding is as real as the positive ones and gets equal billing: reviewers flagged unsupported material claims **more than twice as often in Considered outputs**. The flagged claims are invented specificity — fabricated member names and email addresses, unsupplied reversibility and account policies, invented durations and capabilities. The structural discipline that makes surfaces richer also gives them more room to fabricate, and the honesty rules in place at `m0-001` did not fully contain that. The `HON-` ruleset (the supplied-fact boundary, `craft.md` §4.7) was distilled from these flagged claims in response. An informal directed re-check (`hon-recheck-001`: the three worst scenarios' considered arms rebuilt with the clamp, audited by the same three reviewer models under a stricter claims-only hunt) found the fabricated-records, invented-mechanism, and invented-definition families eliminated; the residue was invented policy assertions concentrated in confirmation-dialog copy, mostly stated in the risk-averse direction, and the teaching was extended against exactly that. The re-check used a different review protocol than `m0-001`, so its counts are directional evidence, not comparable metrics, and it is not a published evaluation record. The raw `m0-001` descriptions are in the published review forms.

Every harness defect found during collection is recorded in `evals/results/m0-001/environment.json` under `notes`, including two discarded partial review rounds (a packet path leak and a broken capture server, both fixed and committed before any label decode) and an ordering deviation. The binary evidence — screenshots, sanitized sources, and packets, about 78 MB — is not tracked in git; `npm run check` validates the complete local archive, and the tracked JSON core carries the metrics, forms, and label map.

### The second matched evaluation: m0-002

The same protocol, catalog, and reviewer arrangement, re-run against the
v0.4.0 corpus after the honesty and craft programs the first evaluation
provoked. Considered was preferred in **24 of 30** blind pairwise choices
(8–2 scenario majorities, seven unanimous sweeps), with mean weighted score
124.1 vs 113.2 and reviewer Ship gates 16 vs 4 — and the negative finding
from m0-001 closed: **unsupported-claim flags landed 8 vs 9, at parity with
baseline**, where m0-001 ran 11 vs 5 against Considered. The two losses are
recorded and analyzed as findings (`evals/results/m0-002/tally.md`), one of
which minted a new rule (`PERS-11`) the same day. The same honesty standard
applies to the collection itself: the record's `environment.json` notes
every deviation — a pilot that caught three harness defects before the run,
six builds superseded uniformly after a baseline builder encountered prior
evaluation context, one evidence-capture artifact repaired with its packet
re-reviewed by fresh reviewers before any label decode, and a 14-of-30
partial-unblinding rate from method residue in Considered source comments,
recorded per form and answered with a sanitizer hardening. As with m0-001,
the tracked JSON core carries the metrics, all thirty forms, and the label
map; the ~63 MB of binary evidence lives only in the local archive that
`npm run check` validates.

### Blind panels against shipped product design

Beyond the matched evaluation, an informal measurement program ranks
Considered builds blind against screens from shipped products with strong
design reputations, judged by three independent models —
one of them non-Anthropic (OpenAI's Codex CLI) — that choose their own
criteria and are never told any screen is generated. To keep the comparison
about design reasoning rather than aesthetic accident, each build receives
the golds' style register as neutral, categorical "house style" facts
(extracted by a quality-blind schema that bans evaluative language), so the
build cannot win or lose on a different visual costume.

Across the program's rounds, Considered builds have taken blind first place
against shipped screens on analytics, first-run onboarding, and checkout
surfaces — including two unanimous three-judge results — and every loss has
been converted into corpus rules whose effect was then verified in a rebuild.
These rounds are directional evidence, not published claims: one build per
surface, model judges, and each round's tally names its own confounds and
protocol deviations. The round records live in `evals/results/ux-stress-*/`
(working tree archives), with summaries in `CHANGELOG.md`.

## Requirements and installation

Node 20+ enables CLI helpers. The Agent Skill itself has a manual, host-neutral fallback described in `references/workflow.md`.

### Install with the `skills` CLI (recommended)

Considered is published as an [Agent Skills](https://github.com/vercel-labs/skills) repository. Install it straight from GitHub:

```bash
npx skills@latest add Simple-Cortex/considered
```

Target a specific agent, or install globally instead of into the current project:

```bash
npx skills@latest add Simple-Cortex/considered --agent claude-code
npx skills@latest add Simple-Cortex/considered --agent cursor
npx skills@latest add Simple-Cortex/considered --agent codex
npx skills@latest add Simple-Cortex/considered --global
```

By default this installs the root skill, `considered`, which is the router and the only surface a host must understand. To install the eight phase skills as separately triggerable entries, ask for the full tree:

```bash
npx skills@latest add Simple-Cortex/considered --full-depth --skill '*'
```

Inspect before installing, remove, or update later:

```bash
npx skills@latest add Simple-Cortex/considered --list
npx skills@latest list
npx skills@latest update
npx skills@latest remove considered
```

### Install manually

From a checkout, for an exact destination — useful in CI or on a host the `skills` CLI does not know:

```bash
node scripts/install.mjs --dest <your-skill-directory>/considered
```

Or copy `SKILL.md`, `references/`, `assets/`, `scripts/`, and `skills/` into your host's skills directory yourself. There is nothing to build.

### npm package (deprecated)

`considered` was also published to npm with its own install wizard (`npx considered`). That package is **deprecated** in favour of the `skills` CLI above; it still works but will not receive further releases. If you installed that way, remove the destination directory and reinstall with `npx skills@latest add Simple-Cortex/considered`.

Use an explicit destination because skill discovery paths vary by host and version. Do not treat a suggested path as verified support until it works in your installed harness.

| Host style | Typical project-local destination to verify | Capability needed |
| --- | --- | --- |
| Agent Skills-compatible host | its documented project skills directory | reads `SKILL.md` and bundled files |
| Claude Code | `.claude/skills/considered` | skill discovery, shell optional |
| Cursor, Copilot, Codex, Aside | the host's documented skills directory | skill discovery; use manual fallback if unavailable |

To update, rerun with `--force` (or accept the wizard's "replace the existing install?" prompt) after reviewing the release. To uninstall, remove the destination directory. `install` never writes outside the destination you supply.

### Dual surface: one install, two discovery paths

There is one installation. The root `SKILL.md` is the core skill and the router; it works on
every host and is the only surface a host must understand. Alongside it, `skills/<command>/SKILL.md`
carries one small trigger-only skill for each of the eight commands — `init`, `frame`, `structure`,
`compose`, `critique`, `subtract`, `harden`, `document`. Hosts that discover nested skills inside an
installed skill directory will offer those eight directly, so a request like "review this dashboard"
can match `considered-critique` without going through the router first. Hosts that do not discover
nested skills simply ignore the directory and use the root skill plus the command word, exactly as
before. The subskills duplicate no protocol: each names its required input, durable output, the exact
`references/workflow.md` section and templates to read, and its single blocking condition, then
points at the normative files.

## Five-minute workflow

1. Put the skill in your host's skill directory and ask the agent to run `considered init` for a project or `considered frame` for a named surface.
2. The agent writes `.considered/PRODUCT.md` and `.considered/<surface-id>/FRAME.md`.
3. During `structure`, it runs:

   ```bash
   node <skill-dir>/scripts/roll.mjs --mode operate
   ```

   It records the assigned hand and raw contract in `.considered/<surface-id>/`.
4. After composition, gather evidence:

   ```bash
   node <skill-dir>/scripts/lint-contract.mjs .considered/<surface-id>/CONTRACT.md --json > .considered/<surface-id>/contract-lint.json
   node <skill-dir>/scripts/lint-source.mjs src --json > .considered/<surface-id>/source-lint.json
   ```

5. Freeze `REVIEW-PACKET.md`, start a fresh reviewer context, and create `REVIEW.json`. Then run:

   ```bash
   node <skill-dir>/scripts/gate.mjs .considered/<surface-id>/contract-lint.json .considered/<surface-id>/source-lint.json \
     --review .considered/<surface-id>/REVIEW.json
   ```

A missing screenshot does not become a pass. The review is marked source-only or specification-only with its limitation.

## CLI utilities

Run these with `node` from the installed skill directory. There is no `considered` command on your `PATH` — that form required the deprecated npm package.

```text
node scripts/roll.mjs [--mode <mode>] [--key <key>] [--gen <n>] [--json]
node scripts/inventory.mjs [path] [--json]
node scripts/lint-contract.mjs <file> [--json]
node scripts/lint-source.mjs [path] [--json]
node scripts/lint-render.mjs <render-evidence.json> [--json]
node scripts/gate.mjs <report.json> [...report.json] --review <REVIEW.json>
node scripts/validate-assets.mjs
node scripts/test.mjs
node scripts/eval.mjs --condition baseline|considered --evaluation-id <id> --out <directory>
node scripts/check.mjs <evaluation-result-directory>
node scripts/install.mjs --dest <skill-directory> [--force]
```

From a checkout, the `npm run` aliases in `package.json` cover the same set: `roll`, `inventory`, `contract`, `lint`, `render`, `gate`, `validate`, `test`, `eval`, `check`.

Exit codes: `0` success, `1` failed gate or validation, `2` usage or input error, `3` dice deck exhausted.

Run the maintained fixture and package checks:

```bash
npm test
npm run validate
npm run prepack
```

## Evaluation harness

`evals/` contains the frozen scenario catalog: matched dense and persuade/read surfaces plus flow, experience, and essence-matched (`*-house-style`) variants. It records source, screenshots, interaction and state evidence, blind A/B review, unsupported claims, rubric dimensions, and exclusion rules.

```bash
npm run eval -- --condition baseline --evaluation-id m0-001 --out evals/results/m0-001/baseline
npm run eval -- --condition considered --evaluation-id m0-001 --out evals/results/m0-001/considered
```

`eval` freezes the catalog and initializes collection folders. A host-specific runner must perform the matched builds and add evidence (`evals/runner/RUNNER.md` documents the reference runner, including the blind gold-standard panel pipeline in `codex-judge.mjs`). `npm run check -- evals/results/m0-001` verifies a completed result record but never declares that Considered is better.

## Status

**v0.7.0.** The corpus is at 190 rules (167 → 190 since v0.3.0, every mint evidence-driven and documented in `CHANGELOG.md`), the blind gold-standard panel pipeline is included, and the ship gate is calibrated against the first evaluation's review distribution. The workflow, deterministic direction assignment, validators, independent-review packet, fixtures, and matched evaluation harness are included, and both complete matched evaluations have been collected, blind-reviewed, and validated: `m0-001` (the honesty deficit found and named) and `m0-002` (the deficit closed at parity while the preference margin held). Both records — metrics, all sixty anonymized review forms, and every harness incident — are published in `evals/results/m0-001/` and `evals/results/m0-002/`. The Rust accelerator serves validated contract, gate, inventory, lint, asset-validation, roll, lifecycle, and compact-output shapes; malformed legacy grammar stays on the byte-compatible Node fallback. The versioned [engine capability matrix](assets/templates/engine-capabilities.json) is the authority for selection and exceptions.

## Attribution

Considered applies an architecture researched and published by [Impeccable](https://impeccable.style/research): assigned direction indices, commit-first generation, in-artifact contracts, and separate review. This project extends that approach with information architecture, question inventories, priority tiers, action scopes, and an open evaluation protocol. Research claims are treated as design hypotheses until reproduced in this harness.

One rule id departs from the source corpus. The source's `EVAL-04` requires generating at
least two structural options before choosing; this project assigns a single structural hand
by deterministic roll instead, so option generation is intentionally not carried forward. The
id had been reused here for an unrelated repo-native rule about the provenance of that
assignment, which is a collision rather than an extension. That rule is now `ROLL-01`
("Assigned roll provenance") and records `supersedes: "EVAL-04"`. The source corpus has 140
rules; this repo has 190: the 139 carried source rules, `ROLL-01` in place of `EVAL-04`, the
repo-native `CONTRACT-01`, 26 guidance rules minted for the mode catalogs
(`PERS-`/`MODE-`/`READ-`/`EXP-`), whose persuade, read, and experience content is original to
this project rather than distilled from the course, 6 honesty rules (`HON-`) distilled
from measured fabrication findings, 7 rules (`PERS-08`, `DASH-19`, `ACT-20`,
`MODE-08`, `CONT-06`, `EXP-07`, `READ-07`) distilled from comparisons of generated output
against shipped product design, the last two confirmed by two independent blind judges,
2 craft rules (`HIER-13`, `EVAL-17`) distilled from the impeccable, typeset, and
layout skill families against the same blind-judge findings, and 7 rules from
three-judge convergent findings against gold references (`FORM-01`, `FORM-02`,
`FORM-03`, `PERS-09`, `MODE-09` — the general genre-over-direction principle
of which `MODE-08`, `READ-07`, and `EXP-07` are instances — `MODE-10`, its
supplied-register complement, and `PERS-10`, minted unanimous from the
essence-matched rounds), and 1 rule (`PERS-11`) minted unanimous from the
second matched evaluation, where all three blind reviewers independently
named the collapsed-evidence failure on the same surface.

## License

MIT
