# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Entries describe user-visible changes. See `docs/changelog-policy.md` for what
belongs here and what stays in the development log.

## [Unreleased]

### Changed

- Install moves to the Agent Skills CLI:
  `npx skills@latest add Simple-Cortex/considered`. The CLI resolves the host's
  skills directory itself; `--agent`, `--global`, and `--full-depth` are
  supported. The manual path — clone and run `node scripts/install.mjs --dest
  <dir>` — is unchanged.
- README examples invoke `node <skill-dir>/scripts/<util>.mjs`. The bare
  `considered <util>` form required a global npm install and is no longer
  documented.

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
