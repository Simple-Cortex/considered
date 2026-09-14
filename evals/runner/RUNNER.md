# Host runner: Claude Code, matched builds via executor subagents

This is the host-specific runner `evals/README.md` requires. It implements protocol
`1.0.0` on the Claude Code harness. It performs matched builds with executor subagents,
captures evidence with a pinned Playwright, assembles blinded packets, and records one
result JSON per evaluation id. It never scores its own output.

## Environment freeze (recorded per evaluation in `environment.json`)

| Setting | Value |
| --- | --- |
| Builder model | one model id for every build in both conditions, recorded verbatim |
| Reviewer models | three distinct model ids, recorded verbatim |
| Harness | Claude Code executor subagents, one invocation per run, no retries after inspecting either condition |
| Starter | `evals/starter/` at the recorded repo commit, copied fresh per run |
| Working directory | fresh scratchpad directory per run, outside the repository; baseline directories contain no reference to this repository or the skill |
| Screenshots | `evals/runner/capture.mjs`; resolved Playwright version, module source, and Chromium build recorded per run in `capture.json` |
| Viewports | scenario-declared desktop viewport plus 390 px mobile |
| Temperature / seed | not exposed by the harness; recorded `null`; outputs are stochastic samples |
| Web access | not mechanically disabled; both conditions receive the same instruction to use only supplied facts and local assets; recorded as an instruction-level control |
| Time budget | one executor invocation per build, no mid-build coaching |
| Browser | each builder launches its own Playwright/Chromium from a script inside its working directory; the session-shared browser tool (Playwright MCP tab) is never used by a builder because concurrent builders share one tab |

## Condition procedure

Baseline run:
1. `node evals/runner/stage.mjs --scenario <id> --condition baseline --evaluation-id <id>` copies the starter into a fresh scratchpad directory and writes `prompt.txt` — the neutral envelope from `protocol.md` verbatim with the frozen scenario JSON substituted.
2. One executor subagent receives `prompt.txt` content and the working directory path. Nothing else. It must not be told about Considered, this repository, or any style target.

Considered run:
1. `stage.mjs --condition considered` additionally installs the skill into `<workdir>/skill/` with `scripts/install.mjs`, prunes `evals/`, `docs/`, and `tests/` from the installed copy — the shipped package carries the evaluation harness, and a builder that finds its own brief in the skill's scenario catalog knows it is being measured; the treatment is the skill surface, not the measurement apparatus — and prepends the fixed line: "An Agent Skill is installed at ./skill. Read ./skill/SKILL.md and follow its workflow for this task." ahead of the same neutral envelope.
2. One executor subagent receives that prompt and the working directory path. It retains `.considered/` artifacts, `CONTRACT.md`, and lint outputs in the working directory.

Both conditions use the same builder model, envelope wording, starter, and working-directory
layout. The only difference is the installed skill and its one-line pointer.

The executor invocation carries, for both conditions identically, one confinement
sentence after the working-directory pointer: work entirely inside the working
directory; do not read or use files, archives, or skills outside it beyond what
prompt.txt itself directs; do not consult any prior implementations. The m0-002
pilot phase demonstrated why: a baseline builder running as a session subagent
inherits the session's repository cwd and skill listing, and one such builder
read a prior evaluation's archives and reported treatment awareness. The sentence
names no treatment and is part of the harness, not the task envelope; prompt.txt
and its recorded sha are unchanged by it.

The invocation carries a fourth line, identical for both conditions, after the
confinement sentence: "If you need a browser to verify your work, start your own
Playwright or Chromium instance from a script in the working directory; do not
use a shared browser tool." This line names no treatment either and is part of
the harness like the confinement sentence, the same way the
spawn-permission line ("You may spawn subagents where the task's own workflow
calls for an independent reviewer.") is a harness line applied identically to
every build regardless of condition — it governs how the builder may operate,
not what it builds. If `stage.mjs` or the protocol text ever embeds these
invocation lines directly, keep this file and that code in step; as of this
writing neither `stage.mjs` nor `protocol.md` embeds them, so only this file
carries the wording, and `prompt.txt` content itself must not change to
include them.

## Evidence procedure (identical for both conditions)

1. `capture.mjs --run <workdir> --scenario <scenario-file> --out <run-dir>` serves the
   working directory on an ephemeral port and captures `desktop.png` at the scenario
   viewport and `mobile.png` at 390 px, both full page.

   The repository stays zero-dependency, so `capture.mjs` resolves an already-installed
   Playwright rather than installing one. `npx --no-install playwright` is not usable here:
   it resolves a version that is not in the local cache and refuses to fetch it. The
   working mechanism is ordinary Node module resolution — `require('playwright')` finds
   Playwright 1.58.2 installed above this checkout — with two escape hatches for other
   machines: `CONSIDERED_PLAYWRIGHT` to name a playwright package directory, and
   `CONSIDERED_CHROMIUM` to name a browser executable when the revision Playwright expects
   is absent from the browser cache. Whatever combination ran is recorded in `capture.json`
   and must be copied into `environment.json` for the evaluation.

   State-matched capture: `--prep <script.mjs>` names a module default-exporting an
   `async (page)` function that runs after load and settle, before each screenshot —
   used when the reference screens for a blind panel were shot mid-session (a tab
   open, a step reached) rather than at idle. The prep path and sha256 are recorded
   in `capture.json`; a failing prep aborts the capture rather than silently falling
   back to idle. Both panel screens and golds must be state-matched or neither —
   never prep the candidate into a richer state than the golds show.
2. An evidence executor reads the built source and writes `evidence/capture-plan.mjs` —
   a replayable Playwright script that drives each required interaction and required
   state, saving one labeled screenshot per item to `evidence/`. The same executor
   procedure, prompt template, and model run for both conditions. If a state cannot be
   reached, it records `unreachable-<state>.txt` with the reason instead of a screenshot.
3. Considered runs additionally copy `.considered/<surface-id>/CONTRACT.md` to `contract.txt` and record
   `lint-contract.json` and `lint-source.json` using the repo scripts.
4. Artifacts are copied into `evals/results/<evaluation-id>/<condition>/<scenario-id>/run-001/`
   and `manifest.json` is completed with times, prompt sha256, and status.

## Order, blinding, review

- `evals/runner/order.mjs --seed <n>` emits the per-scenario condition order before any
  build; the seed is recorded in the evaluation manifest.
- `evals/runner/blind.mjs` assembles `blind-packets/<packet-id>/A|B` per scenario:
  rendered screenshots, evidence directory, and a sanitized source copy with
  `.considered/`, contract wrappers, lint output, and treatment-revealing filenames or
  comments removed. The A/B label map is written to `label-map.json` at the result root;
  reviewers never receive that path. Sanitization reaches source and evidence text only:
  a screenshot that renders the word on screen cannot be scrubbed, so a build that names
  the skill in visible copy unblinds itself. Treat that as a build defect and record it.
  Comments naming either condition — "considered" anywhere, "baseline" when a script
  annotates its run — are dropped uniformly in both arms (code lines keep both words;
  CSS `baseline` survives), as are comments carrying rule-code tokens (`HON-04`,
  `A11Y-07` — uppercase, exactly two digits, so `UTF-8`/`ISO-8601` survive): 14 of 30
  m0-002 forms flagged that residue class as a pipeline fingerprint. A block-aware
  tripwire fails the packet build loudly if either class survives in any comment,
  including block-comment interiors. Known residual: method attributes in shipped
  markup (`data-zone`, `data-el`) are behavior and cannot be scrubbed — a build that
  fingerprints itself in live DOM is the visible-copy case; record it.
- Three reviewer executors per scenario packet, three distinct models, fresh contexts.
  Each receives the scenario brief, packet A, packet B, and `docs/blind-review-form.md`,
  and returns the completed form. Reviewer role and conflict status are recorded;
  builders and the packet assembler do not review.
- Labels are decoded only after all forms for the evaluation id are recorded.
  `evals/runner/assemble.mjs --results <result-root>` merges runs, reviews, and decoded
  labels into `result.json` and computes the declared metrics per `metrics.md`. It reads
  `<root>/environment.json` (the filled-in skeleton from `stage.mjs`), each
  `<root>/<condition>/<scenario-id>/<run>/manifest.json`, `<root>/label-map.json`, and one
  completed review form per file at `<root>/reviews/<packet-id>/<reviewer-id>.json`. Each
  form carries `pairwise_choice`, `choice_reason`, `unsupported_claim_label`,
  `gate_by_label`, `findings[]`, and `scores.A`/`scores.B` as D1–D9 ratings; the weighted
  total is computed, never transcribed. Artifact paths are derived from what is on disk,
  including a `source.tar.gz` archived from each run's `source/`.
- `npm run check -- evals/results/<evaluation-id>` is the final structural gate.

## Pilot rule

Mechanics are proven on a throwaway evaluation id (`pilot-*`, one scenario, both
conditions, one reviewer) before any `m0-*` id is started. Pilot records never merge
into a real evaluation and are deleted rather than published. A defect found during an
`m0-*` run supersedes the full matched pair per `protocol.md` §6; it does not patch a
single condition in place.

## Blind gold-standard judging (`codex-judge.mjs`)

The token-efficient stress harness for informal rounds. Deterministic Node
assembles an anonymized panel — the candidate capture top-cropped, resampled,
and re-encoded to be format-identical to `gold-N.jpg` reference screens, then
shuffled among them by seed — and a non-Anthropic judge (the Codex CLI) ranks
every screen choosing its own criteria. The judge is never told a screen is
generated, which one is the candidate, or that a skill exists; the decode map
is written beside the panel, never inside it.

    node evals/runner/codex-judge.mjs --shot <capture.png> \
      --gold <dir with gold-1..N.jpg> --category "<judging label>" \
      --out <dir> --seed <n> [--codex-model <m>]

Outputs `verdict.json` (our rank, winner, per-screen scores), `codex-raw.json`
(verbatim response), `panel/`, and `map.json`. Gold reference sets are curated
independently (selectors must never see candidate builds) and reused across
rounds. Host requirements: macOS `sips`, `codex` on PATH. Same-seed runs
replay the same shuffle. These verdicts are directional benchmarks, never
`m0-*` evidence.

## Landmines and notes

- **Shared browser tab.** Three of seven builders across two rounds reported
  that the session-shared Playwright MCP browser tab was navigated away by
  another concurrent builder mid-capture, because every concurrent builder in
  a session shares that one tab. Each affected builder recovered by checking
  `document.title` (or an equivalent page identity check) against what it
  expected before trusting a screenshot or DOM read, then re-navigated and
  retried. The rule that prevents the incident, not just the recovery from
  it, is the browser line in the Environment freeze table and the condition
  procedure above: a builder that needs a browser starts its own
  Playwright/Chromium instance from a script in its own working directory
  and never touches the shared browser tool.
