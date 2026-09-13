# Script test fixtures and contract

This directory contains fixture data and test contracts for the package scripts. All scripts listed as implemented are exercised by `scripts/test.mjs` via `npm run test`.

`scripts/test.mjs`, registered as `npm run test`, loads `fixtures/manifest.json`, executes every listed case, parses `--json` output, and fails closed on an assertion mismatch. It does not write into `assets/`, `evals/scenarios/`, or the fixtures themselves.

## Fixture inventory

| Script | Status | Positive fixture | Negative or edge fixture | Required test contract |
| --- | --- | --- | --- | --- |
| `lint-contract.mjs` | implemented | `contracts/valid-operate.tsx` | `missing-contract.tsx`, `invalid-operate.tsx` | The valid contract exits 0 with no findings. Missing contract and destructive-primary cases exit 1 and include the manifest's required findings. |
| `lint-source.mjs` | implemented | `source/clean/` | `source/violations/` | Clean source exits 0 with no findings. The violation sample exits 0 (heuristic leads are not gate-blocking by default) and includes every required S1, S2, and S3 listed in the manifest. |
| `inventory.mjs` | implemented | `projects/system/` | `projects/empty/` | Both exit 0 and emit valid JSON. The system fixture identifies existing tokens and `StatusCard`; the empty fixture reports empty collections without crashing. |
| `roll.mjs` | implemented | `roll/cases.json` deterministic and chained draws | invalid mode and reroll without key | Enforce reproducibility, no repeated chained deals, and strict argument errors. This fixture deliberately locks desired behavior for known edge cases. |
| `validate-assets.mjs` | implemented | Repository's own assets | N/A | Exits 0 when all manifests, decks, and gate policy are valid. |
| `gate.mjs` | implemented | `gate/contract.json` + `review-ship.json` | `review-revise.json` | Ship review exits 0. Revise review exits 1. |
| `eval.mjs` | implemented | N/A (evaluation assembly) | N/A | Assembles evaluation records. |
| `check.mjs` | implemented | N/A (result validation) | N/A | Validates result directory structure. |
| `lint-render.mjs` | future | Add a documented passing screenshot fixture | Add focal, grayscale, and fold failures | Use generated local images only. Pin browser and image-decoder versions in the test log. |

## Rust accelerator

Selected deterministic scripts have Rust equivalents in the `crates/` workspace. The Rust engine is invoked via `considered-rs <command>`. `tests/compat/run.mjs` freezes the public Node contract (34 cases); `tests/compat/shadow.mjs` compares Node, direct Rust, and public native dispatch for validated shapes, file effects, ordering, lifecycle commands, and repeated-run determinism (23 exact JSON/error cases, 6 human-output cases, 7 native feature cases, 4 lifecycle commands).

`fixtures/source/real-admin-surface/` is a generated admin surface (four source files: `app.js`, `index.html`, `README.md`, `styles.css`) retained as a realistic lint and inventory input; copied 2026-09-13. Both `lint` and `inventory` shadow cases against it are exact-parity (Node and Rust byte-identical).

## Assertion rules

- `expected_findings: []` means exactly no findings. `required_findings` is a lower bound, allowing a deliberate checker expansion to add findings only when the fixture contract is updated intentionally.
- A required finding matches both rule ID and severity. Changing a severity is a fixture-contract change and must be reviewed rather than silently accepted.
- Run each linter fixture from a temporary copy or pass an absolute fixture target. Do not depend on the current working directory.
- Normalize absolute paths, timestamps, generated roll keys, and platform line endings before comparing output. Do not normalize rule IDs, severities, selected deck IDs, or exit statuses.
- Keep fixture samples minimal. Each seeded violation should exist for a named assertion, not to inflate finding counts.

## Current scope limits

These fixtures test only behavior actually asserted by a script. They do not prove semantic accessibility, rendered visual hierarchy, Tailwind AST coverage, or design quality. Add a fixture only after documenting what the checker can observe and how false positives or negatives are handled.

The `invalid-mode-rejected` and `reroll-without-key-rejected` roll cases are intended regression contracts. If the current implementation does not satisfy them, the test should expose that gap rather than weaken the case.

## Running tests

```bash
npm run test
```

Use `npm run check -- evals/results/<id>` for result-record validation. Tests and checks are distinct: tests exercise script behavior; checks validate a collected evaluation record.
