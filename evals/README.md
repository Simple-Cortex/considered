# Considered evaluation harness

This directory defines the M0 measurement harness. It contains test cases and a review procedure, not performance results. No effectiveness conclusion is warranted until matched baseline and Considered runs have been captured, blinded, reviewed, and validated.

## Scenario set 1.0.0

| Track | Scenario IDs | Count |
| --- | --- | --- |
| Dense | `dense-sre-console`, `dense-support-queue`, `dense-settings-23-actions`, `dense-chart-dump-rescue`, `dense-admin-bulk-destructive` | 5 |
| Persuade/read | `persuade-saas-landing`, `persuade-pricing-page`, `read-docs-home`, `persuade-portfolio`, `read-changelog` | 5 |

Each JSON file is the source of truth for the brief, facts, viewport, required interactions, required states, and acceptance checks. Treat facts as a closed set. A run must not add customers, proof, metrics, capabilities, policies, or product behavior that the scenario does not supply.

## Stable command contract

The package exposes these commands. `eval` initializes an immutable condition collection only. A host-specific runner must create the actual UI artifacts and evidence; it must not score its own output.

```bash
npm run test
npm run eval -- --condition baseline --evaluation-id <id> --out evals/results/<id>/baseline
npm run eval -- --condition considered --evaluation-id <id> --out evals/results/<id>/considered
npm run check -- evals/results/<id>
```

| Command | Required future behavior | Exit status |
| --- | --- | --- |
| `npm run test` | Run deterministic fixture tests for every maintained script. | 0 pass, nonzero failure |
| `npm run eval -- ...` | Validate scenario input and initialize an immutable manifest plus per-scenario collection folders for exactly one condition. It does not invoke a model or score output. | 0 initialized, nonzero invalid input or collection failure |
| `npm run check -- <result-dir>` | Validate result JSON against `result.schema.json`, validate artifact paths and required paired conditions, and recompute declared aggregate metrics from recorded reviews. | 0 valid, nonzero invalid or incomplete |

The eventual implementation may add options, but it must not change these names, condition values, or required output records. It must print its effective model, runner, scenario catalog version, and prompt hashes. `check` is a verifier, not a quality gate: a valid result can show no improvement.

## Run sequence

1. Read [protocol.md](protocol.md). Freeze the scenario files before starting an evaluation ID.
2. Build all ten `baseline` runs in clean, identical starter projects. Do not load Considered files, decks, rules, contracts, or prior artifacts.
3. Build all ten `considered` runs under the matched environment using the documented Considered workflow.
4. Use the selected host runner to capture required evidence for each run: source, rendered desktop and mobile screenshots, interaction/state evidence, and run manifest. A Considered run additionally retains its contract and lint outputs, but those remain hidden during blind review.
5. Prepare blinded A/B packets, collect the form in [../docs/blind-review-form.md](../docs/blind-review-form.md), then enter reviews and derived metrics in one result JSON.
6. Run `npm run check -- evals/results/<id>`. Publish the protocol version, raw anonymized review data, exclusions, and outcomes together. Do not replace a run after seeing a score; record a new run ID and explain the exclusion.

## Artifact layout expected from a host-specific runner

```text
evals/results/<evaluation-id>/
  result.json
  baseline/<scenario-id>/<run-id>/
    manifest.json
    source/
    desktop.png
    mobile.png
    evidence/
  considered/<scenario-id>/<run-id>/
    manifest.json
    source/
    desktop.png
    mobile.png
    evidence/
    contract.txt
    lint-source.json
    lint-contract.json
  blind-packets/<packet-id>/
    A/  B/  manifest.json
```

Paths recorded in `result.json` are relative to the result directory and must not escape it. Keep raw runner logs only when they do not expose credentials or unrelated project data.

## Documents

- [protocol.md](protocol.md): matched baseline-versus-Considered procedure.
- [metrics.md](metrics.md): metric formulas and reporting rules.
- [result.schema.json](result.schema.json): machine-validatable result record.
- [../docs/blind-review-form.md](../docs/blind-review-form.md): human review worksheet.
- [../tests/README.md](../tests/README.md): fixture and script-test contract.
