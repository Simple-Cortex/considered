# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries describe user-visible changes. See `docs/changelog-policy.md` for what
belongs here and what stays in the development log.

## [Unreleased]

## [0.7.1] - 2026-09-14

### Added

- `lint` now emits `HON-03` review leads when source text carries instance
  data that is not marked illustrative nearby: email addresses, card
  fragments, invoice or period labels, recency phrases ("last used 3 days
  ago"), name-and-email pairs, and current-value settings such as a timezone
  or date format. Leads, never proof, like the other `COMP` heuristics; the
  critique pass now starts its honesty check from them.
- `PERS-12` "Whole before large": a reproduced work is shown whole and legible
  in the first viewport before any zoomed or cropped view, and a paired text
  column matches its height.
- `PERS-13` "The offer set is visible before the argument": on a pricing or
  plan-selection surface every plan's name and price sits in the first
  viewport, whatever framing precedes it.
- `COMP-19` "Native controls wear the theme": select, input, textarea,
  checkbox, radio and button elements carry the surface's ground, ink, border
  and focus tokens; a user-agent default box on a themed ground is a render
  defect.

### Changed

- `EXP-07` now applies to every surface, not only calming or focus surfaces:
  primary actions and ambient chrome avoid alarm-coded hues unless the action
  is destructive or the surface's job is to warn.
- The canonical CLI in the skill text is `node <skill-dir>/bin/considered.mjs
  <utility>` for `roll`, `contract`, `lint`, and `gate`. The dispatcher runs
  the native engine when `considered-rs` is present and the Node scripts
  otherwise, with identical output; calling `scripts/*.mjs` directly still
  works.
- Skill text is host-neutral: `<skill-dir>` is defined; the contract sidecar
  path is stated as `.considered/<surface-id>/CONTRACT.md`; the no-Node roll
  fallback names the deck files and tier order; headless runs record a
  question and its assumption instead of inventing an answer; a supplied
  surface or route id is preferred over a derived slug; the install table
  covers any host that loads `<skills-dir>/<name>/SKILL.md`; the contract
  template names the safe comment wrapper per file type.
- `references/persuade.md` and `references/workflow.md` gain worked examples
  for `PERS-12` and `HON-06` (the skill's own tier and zone vocabulary is not
  user copy).

### Fixed

- Under the native engine, `validate`, `contract`, `gate`, `roll`,
  `context --experimental`, and `validate-skill` failed with "Cannot find
  project root" whenever the installed skill was invoked from outside its own
  directory. The dispatcher now passes the skill root explicitly (`--root`,
  also accepted directly by each `considered-rs` subcommand); `status` and
  `verify` stay anchored to the caller's project by design. The compact
  contract cache lives under the caller's `.considered/cache`, never inside
  the skill install. A usage error (exit 2) now carries the same
  `usage_error` envelope field on both engines; see the envelope error-code
  table in `docs/engine-capabilities.md`.
- The contract parser (Node and Rust) no longer splits a HIERARCHY row on a
  comma or semicolon inside its reason text, ignores Markdown fence lines,
  and stops the ROLL field correctly when the contract body is indented
  inside a comment. A multi-element row with per-item reasons was never a
  documented shape and now parses as one element plus its reason.
- Every `scripts/*.mjs` command answers `--help`/`-h`; `lint` explains that it
  takes one directory or file; `gate` shows a complete example invocation.


## [0.7.0] - 2026-09-13

### Added

- Native Rust engine (`considered-rs`) for `contract`, `gate`, `inventory`,
  `lint`, `validate`, and `roll`. When the binary is present the CLI selects
  it for supported argument shapes and otherwise runs the Node implementation;
  output is byte-identical between engines. Install with
  `cargo install --path crates/considered-cli`.
- `--format engine-json`: a versioned envelope stating which engine ran and
  why, alongside the unchanged `--json` output.
- Native-only lifecycle commands `validate skill`, `status`, `verify`, and
  `context --experimental`. Without the binary they report
  `manual-unavailable` rather than failing silently.
- Engine capability matrix (`assets/templates/engine-capabilities.json`),
  with `docs/engine-capabilities.md` and `docs/known-limitations.md`
  recording what is tested and what is not.

### Changed

- Install moves to the Agent Skills CLI:
  `npx skills@latest add Simple-Cortex/considered`. The CLI resolves the host's
  skills directory itself; `--agent`, `--global`, and `--full-depth` are
  supported. The manual path — clone and run `node scripts/install.mjs --dest
  <dir>` — is unchanged.
- README examples invoke `node <skill-dir>/scripts/<util>.mjs`. The bare
  `considered <util>` form required a global npm install and is no longer
  documented.
- `runtime-manifest.json` is the single authority for which files an install
  contains.

### Deprecated

- The `considered` npm package. It still installs but receives no further
  releases.

### Fixed

- Development files are no longer included in installs.

## [0.6.0] - 2026-08-04

### Added

- Interactive install wizard for the npm package: run in a terminal, it
  detected a project or global skills directory, confirmed the destination
  before writing, and never silently overwrote an existing install.
  Superseded by the Agent Skills CLI.
- `install --yes` for non-interactive use, proceeding only when a confident
  default exists.

## [0.5.0] - 2026-08-04

### Added

- `PERS-11` ("the argument survives collapse"), bringing the guidance corpus
  to 190 rules.

### Fixed

- Harness reliability fixes, each covered by a regression check.

## [0.4.0] - 2026-08-03

### Added

- A clamp on invented specificity: the skill no longer fabricates concrete
  detail the brief did not supply, with matching builder-side teaching in
  `references/craft.md`.
- Six guidance rules covering operate-mode density, persuade-mode evidence,
  and mode-independent direction expression.
- New frozen scenario: a 390 px mobile guided session surface.

### Fixed

- `capture.mjs`: directory routes served without a trailing slash rendered
  incorrectly.
- `blind.mjs` sanitizer: an over-broad rule stripped content it should have
  preserved.

## [0.3.0] - 2026-07-31

### Added

- The evaluation runner (`evals/runner/`) and the first published evaluation
  record (`evals/results/m0-001/`).

### Fixed

- `eval.mjs` failed when the output directory's parent did not exist.
- `assemble.mjs` joined findings on a field the review forms do not use.

## [0.2.0] - 2026-07-31

### Added

- `SKILL.md` entry point with mode, surface, and command routing.
- `skills/<command>/SKILL.md` for all eight commands (`init`, `frame`,
  `structure`, `compose`, `critique`, `subtract`, `harden`, `document`).
- `references/workflow.md` as the normative command, artifact, and all-host
  fallback specification.
- The brief gate: `frame` scores every brief on an eight-dimension rubric and
  blocks design work on an unscored or failing brief.
- Fifteen reference catalogs covering modes, information architecture,
  hierarchy, actions, forms, states, accessibility, craft, and critique.
- The consequence-to-friction ladder: friction scales to the cost of being
  wrong, not to how destructive an action sounds.
- A required steelman section in the reviewer packet.
- A dashboard-type declaration (`operational | analytical | strategic |
  embedded`).
- Deterministic roll (`scripts/roll.mjs`) assigning three structure laws and
  one visual direction, so the agent cannot pick its comfort zone.
- Contract, source, and render validators plus the `gate` decision script.
- 167 rules across four manifests, with bidirectional coverage validation —
  every rule must be both taught and enforced.
- The ship-gate policy as data (`assets/rules/gate.json`).
- Matched evaluation harness with ten scenarios and a result schema.
- Continuous integration across Node 20 and 22.

### Changed

- Gate thresholds moved from code into `assets/rules/gate.json`.
- `SKILL.md`'s frontmatter description rewritten as trigger conditions only.
- `engines.node` raised from `>=18` to `>=20`.

### Fixed

- `scripts/install.mjs` copied the entire repository root into the
  destination.
- The ship gate failed open on an incomplete review.
- Nineteen rules were enforceable but taught in no reference.
- Accessibility gates `A11Y-01`–`A11Y-12` had inconsistent severity across
  manifests.
- `scripts/test.mjs` under-reported per-test passes.
