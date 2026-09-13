# Review Cockpit

The review cockpit is a static HTML report that presents the full state of a design review for human inspection. It aggregates findings from contract checks, source linting, and independent review into a single self-contained page.

## What the cockpit shows

The cockpit contains these sections, rendered in order:

### Missing or unreadable artifacts
If any expected JSON files are absent or malformed, this section lists each one with the specific error. The cockpit never crashes on missing input — it reports what is unavailable and continues.

### Gate Outcome
The ship/revise decision from the gate, with reasons. Only shown when a review.json artifact is present with an outcome.

### Decision & P0
The primary design decision and its evidence, from the run record. This is the organizing principle for the review — what the design is trying to achieve and why.

### Assigned Roll
The structure and direction assigned by the deterministic roll. The cockpit shows what was assigned — it does not provide UI for changing the direction or rerolling. Bounded reroll rules are preserved: the cockpit displays the assignment, not alternatives.

### Question-to-Element Traceability
A mapping of design questions to the elements they apply to. This shows which questions the review should answer for each part of the surface.

### Contract / Source Differences
Side-by-side findings from the contract checker and source linter. Contract findings prove internal consistency violations. Source findings are heuristic leads for a reviewer, not proof.

### Review Freshness
Whether the review is stale — i.e., whether the source has changed since the review was performed. A stale review is visually indicated with a red badge and warning text. Staleness is determined by the `reviewValidity.stale` field in the run record.

### Evidence
Links to evaluation artifacts (source files, contracts, screenshots) with their hashes. Provides traceability from findings back to the specific code or artifact they reference.

### Limitations
Known limitations from the project's maintained limitations register. References `docs/known-limitations.md` for the full context.

### Ordered Findings
All findings from all sources (contract, source, review), sorted by severity (S1 → S4), then file, then line. Each finding shows its disposition: accept, reject, defer, or pending.

## How to generate it

```bash
# Generate cockpit.html in the result directory
node scripts/cockpit.mjs <result-directory>

# Generate at a custom output path
node scripts/cockpit.mjs <result-directory> --output <path>
```

### Expected artifacts

The cockpit reads these JSON files from the result directory (all optional):

| File | Source | Content |
|------|--------|---------|
| `run-record.json` | `.considered/<surface>/runs/<id>.json` | Decisions, roll, questions, artifacts, review validity |
| `contract.json` | `considered contract --json` | Contract lint findings |
| `source.json` | `considered lint --json` | Source lint findings |
| `review.json` | `considered gate --review` | Review findings, gate outcome, scores |
| `known-limitations.json` | Structured limitations | Optional structured limitation entries |

The cockpit handles missing or malformed files gracefully — it reports what is unavailable in the warnings section and continues rendering with whatever data is present.

## How to interpret each section

**Gate Outcome**: SHIP means the review passed all thresholds. REVISE means at least one threshold was breached. REVIEW-REQUIRED means the review data is incomplete or uninterpretable.

**Review Freshness**: FRESH means the source has not changed since the review. STALE means the source has changed and the findings may no longer reflect current code. A stale review should be re-run before making ship/revise decisions.

**Finding severity**:
- **S1**: Blocker — must fix before ship
- **S2**: Major — should fix, gate threshold applies
- **S3**: Minor — advisory, gate threshold applies
- **S4**: Informational — no gate impact

**Finding disposition**:
- **accept**: Finding is acknowledged and will be addressed
- **reject**: Finding is disputed or determined to be a false positive
- **defer**: Finding is acknowledged but deferred to a future iteration
- **pending**: No disposition has been recorded yet

**Confidence**: Source lint findings are marked `heuristic` — they are leads for a reviewer, not proof. Contract findings are `high` confidence — they prove internal consistency violations.

## Why live functionality was deferred

The cockpit is intentionally static. Live element selection, variant comparison, and interactive rerolling were considered but not implemented.

**Decision rationale:**
- Live selection was considered for the cockpit
- No recorded evidence demonstrates that chat-based review repeatedly blocks inspection, selection, approval, or recovery
- The static cockpit is sufficient for current inspection requirements
- All information needed for review is present in the generated HTML
- Static output is reproducible, auditable, and has no runtime dependencies

**When to reconsider:**
If evidence emerges that chat-based review repeatedly blocks inspection, selection, approval, or recovery — for example, if reviewers consistently cannot identify which finding a cockpit row refers to in the source, or if the inability to switch between variants causes repeated re-review cycles — live functionality can be added in a future milestone.

Until that evidence exists, the static cockpit is the correct scope.

## Constraints

- **No JavaScript**: The output is pure HTML and CSS. No `<script>` tags, no event handlers, no dynamic behavior.
- **No external dependencies**: All CSS is inlined. The file is fully self-contained.
- **XSS-safe**: All user content is escaped before inclusion in the HTML.
- **Deterministic**: Same input produces identical output. No timestamps, random values, or non-deterministic ordering.
- **Preserves assigned direction**: The cockpit shows what was assigned by the roll. It does not provide UI for changing the direction or rerolling.
