# Baseline-versus-Considered evaluation protocol

Protocol version: `1.0.0`.

## Objective

Measure whether a matched run using Considered produces a more usable implementation than an unassisted baseline on this fixed scenario set. This protocol measures outputs, not the perceived quality of a prompt or a reviewer's familiarity with the project. It is not a claim of universal model performance.

## Conditions

| Condition | Allowed context | Disallowed context |
| --- | --- | --- |
| `baseline` | Scenario JSON, identical starter project, normal framework documentation already available in the runner, and the common build envelope below. | `SKILL.md`, `references/`, `assets/decks/`, `assets/rules/`, contract template, lint outputs, prior output, and any summary of Considered instructions. |
| `considered` | The same scenario JSON and starter project, plus the installed Considered skill and its normal workflow. | Baseline output, blind-review feedback, a prior run's artifacts, or any scenario-specific hint not also in the scenario JSON. |

Do not describe the baseline as inferior or ask the builder to imitate a named style. The only treatment difference is access to Considered.

## Common build envelope

Freeze these settings in the evaluation manifest before any run:

- Same exact model identifier, model revision when exposed, harness, starter-project commit, and dependency lockfile.
- Same tool permissions, network policy, time budget, token budget, viewport, operating system, browser version, and screenshot routine.
- Fresh working directory per run. No reusable component, cache, or artifact may cross condition boundaries.
- Temperature and seed, if the runner exposes them. If it does not, record `null` and treat outputs as stochastic samples rather than controlled replicas.
- Web access disabled unless a future scenario explicitly changes that field for both conditions. Scenarios currently provide all source facts.
- Each build implements one route using local code and assets only. It must render at the scenario's desktop viewport and 390 px width.

Use the neutral task envelope below verbatim, replacing only `<scenario-json>` with the frozen file content:

```text
Implement one frontend surface in the provided starter project from the following scenario.

<scenario-json>

Use only the supplied facts. Do not invent claims, customers, metrics, policies,
capabilities, or links. Build the required route and interactions, include the stated
states, and leave a runnable local implementation. Capture the required evidence when done.
```

For the Considered condition, invoke the normal skill workflow before this common envelope. Do not append a hidden scoring rubric to either builder prompt.

## Collection procedure

1. Assign a new `evaluation_id`, then copy all ten scenario files into the result directory or record SHA-256 digests for each. A scenario changed after the first run creates a new catalog version and evaluation ID.
2. Generate a randomized condition order per scenario before building. Store the randomization seed in the evaluation manifest. Do not always run baseline first.
3. For each scenario and condition, create a clean starter-project copy, run the condition, and record start/end times, effective settings, prompt hashes, and tool failures.
4. Perform the same evidence script for each successful run:
   - desktop screenshot at the declared viewport;
   - mobile screenshot at 390 px width;
   - short evidence capture for every required interaction and state;
   - source archive and build/run command;
   - source-lint output where applicable.
5. Mark a failed, unavailable, or non-rendering run `excluded` with a concrete reason. Do not silently retry a condition after inspecting the other condition. A predeclared retry policy may replace a failed run only before reviewing either output.
6. Preserve successful runs unchanged. If a defect in the harness requires rebuilding, supersede the full matched pair and retain the prior records as excluded.

## Blind-review procedure

1. A packet assembler assigns opaque labels A and B independently for every scenario. The label map is retained outside reviewer access.
2. Hide condition names, prompt transcripts, contracts, lint output, directory names, and comments or filenames that disclose the treatment. When source must be inspected, provide a sanitized copy that removes only treatment metadata, not behavior.
3. Give reviewers the scenario brief, the two rendered packages, interaction/state evidence, and the blind form. The scenario facts are available so reviewers can flag unsupported claims.
4. Each reviewer completes the pairwise choice before seeing any weighted scores or other reviewers' choices. Collect a reason and factuality flag with the choice.
5. Use at least three independent reviewers for a report intended to make a directional claim. If fewer reviewers participate, report the count and describe the result as exploratory.
6. A reviewer who built, edited, or assembled a packet may not rate that packet. Record reviewer role and conflict status, not personal identity, in the result record.
7. After the pairwise choice, reviewers may score each output and record findings. Accessibility and interaction claims require inspectable evidence, not inference from a still image.
8. Decode condition labels only after all forms for an evaluation ID are locked. Store the label map with access limited until that point.

Blindness reduces expectation bias but does not guarantee that a reviewer cannot infer a treatment from an output. Record any suspected unblinding in the form and report it.

## Validity and reporting

- Analyze matched pairs only. Do not compare a baseline from one scenario or environment with a Considered run from another.
- Keep `excluded`, `failed`, and `incomplete` runs in the published result record. Report their counts by condition and scenario.
- Report raw counts alongside every rate, and report ties and abstentions separately.
- Separate mechanical compliance results from human quality judgments. A contract lint pass only shows internal consistency against that checker, not design success.
- Do not call a result an improvement solely because a mean is positive. State the sample size, reviewer count, uncertainty method if used, exclusions, and the specific metric.
- Treat unexpected factual inventions as a separate safety or credibility outcome even when reviewers prefer the layout.

## Minimum completion record

An evaluation may be marked complete only when it has ten matched scenario records, both conditions for every non-excluded scenario, evidence paths for each successful run, blinded review packets, decoded review records, computed metrics, and a passing schema/check validation. A complete record may still be inconclusive.
