# Script test fixtures and contract

This directory is a fixture plan and test contract for the package scripts. It contains no production source and makes no claim that the scripts have already been exercised in this environment.

The future `scripts/test.mjs`, registered as `npm run test`, must load `fixtures/manifest.json`, execute every listed case from a temporary working directory, parse `--json` output where the command supports it, and fail closed on an assertion mismatch. It must not write into `assets/`, `evals/scenarios/`, or the fixtures themselves.

## Fixture inventory

| Script | Positive fixture | Negative or edge fixture | Required test contract |
| --- | --- | --- | --- |
| `lint-contract.mjs` | `contracts/valid-operate.tsx` | `missing-contract.tsx`, `invalid-operate.tsx` | The valid contract exits 0 with no findings. Missing contract and destructive-primary cases exit 1 and include the manifest's required findings. |
| `lint-source.mjs` | `source/clean/` | `source/violations/` | Clean source exits 0 with no findings. The violation sample exits 1 and includes every required S1, S2, and S3 listed in the manifest. |
| `inventory.mjs` | `projects/system/` | `projects/empty/` | Both exit 0 and emit valid JSON. The system fixture identifies existing tokens and `StatusCard`; the empty fixture reports empty collections without crashing. |
| `roll.mjs` | `roll/cases.json` deterministic and chained draws | invalid mode and reroll without key | Enforce reproducibility, no repeated chained deals, and strict argument errors. This fixture deliberately locks desired behavior for known edge cases. |
| future `lint-render.mjs` | Add a documented passing screenshot fixture | Add focal, grayscale, and fold failures | Use generated local images only. Pin browser and image-decoder versions in the test log. |
| future `eval.mjs` and `check.mjs` | Add a minimal valid result directory | Add schema-invalid paths, unpaired conditions, and stale computed metrics | Validate data integrity only; never assert a quality outcome. |

## Assertion rules

- `expected_findings: []` means exactly no findings. `required_findings` is a lower bound, allowing a deliberate checker expansion to add findings only when the fixture contract is updated intentionally.
- A required finding matches both rule ID and severity. Changing a severity is a fixture-contract change and must be reviewed rather than silently accepted.
- Run each linter fixture from a temporary copy or pass an absolute fixture target. Do not depend on the current working directory.
- Normalize absolute paths, timestamps, generated roll keys, and platform line endings before comparing output. Do not normalize rule IDs, severities, selected deck IDs, or exit statuses.
- Keep fixture samples minimal. Each seeded violation should exist for a named assertion, not to inflate finding counts.

## Current scope limits

These fixtures test only behavior actually asserted by a script. They do not prove semantic accessibility, rendered visual hierarchy, Tailwind AST coverage, or design quality. Add a fixture only after documenting what the checker can observe and how false positives or negatives are handled.

The `invalid-mode-rejected` and `reroll-without-key-rejected` roll cases are intended regression contracts. If the current implementation does not satisfy them, the test should expose that gap rather than weaken the case.

## Stable command

Once `scripts/test.mjs` is added and registered, run:

```bash
npm run test
```

Use `npm run check -- evals/results/<id>` for result-record validation. Tests and checks are distinct: tests exercise script behavior; checks validate a collected evaluation record.
