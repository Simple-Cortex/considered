# Considered review packet v1

This packet is prepared by the build context and consumed by a reviewer that did not see the build reasoning. It must contain evidence, not a defense of the design. Copy the required snapshots into this file or attach them with stable workspace-relative paths. A path alone is not sufficient context for a reviewer that cannot access it.

## Reviewer instructions

Review the artifact adversarially: find reasons it fails. Judge the artifact and evidence, not presumed intent. Do not request the builder's reasoning. Run the pass order and gate in `references/critique.md`. On a first review, produce at least five findings including one S2 or higher unless the review scope is explicitly blocked. Fill the steelman section below. Name what works so later fixes do not regress it.

## Review identity

**Surface id:**
**Review pass:** initial | follow-up
**Fresh context:** true | false, with reason
**Review scope:** render-and-source | source-only | specification-only
**Artifact under review:** `<workspace-relative path or URL>`
**Preview or screenshot evidence:** `<path, URL, or unavailable with reason>`
**Contract placement:** wrapper | sidecar | both

## Frozen inputs

Paste the current values or an exact snapshot below. Do not include draft rationale, self-critique, or a list of desired verdicts.

### Product context

```text
<paste relevant PRODUCT.md sections>
```

### Frame

```text
<paste decision, mode, question inventory, object model, assumptions, and blockers>
```

### Structure

```text
<paste assigned hand, zone map, hierarchy table, action table, layout and state plan, and deletions>
```

### Canonical contract

```text
CONSIDERED-CONTRACT v1
<paste complete raw contract>
```

### Evidence and limitations

| Evidence | Location or observation | Limitation |
| --- | --- | --- |
| Render or screenshot |  |  |
| Source target |  |  |
| Contract lint |  |  |
| Source lint |  |  |
| Accessibility or responsive check |  |  |

### Component and library evidence (include only when fit is relevant)

Freeze the minimum selected facts the reviewer needs to assess component fit. Prefer copying the relevant facts below into this packet. An attached snapshot must be immutable and content-addressed (include its digest); a cache path or other mutable pointer alone is not frozen evidence. Do not attach the full cache or builder selection rationale. If component fit is irrelevant to this review, write `not applicable` and do not load the design-system reference.

```text
Package and resolved version/revision:
Selected component(s):
Documented purpose and relevant API/variants:
Relevant constraints and accessibility behavior:
Project-local wrapper/theme overrides affecting behavior:
Provenance (source paths/URLs for these claims):
Fingerprints (package/source, wrappers, and themes as relevant):
Snapshot digest or immutable snapshot location, if used:
Known coverage gaps, unknowns, and freshness limitations:
```

The reviewer evaluates fit independently against the frozen surface need and evidence. Library documentation informs the judgment; it does not require a recommendation to use that component. Mark unavailable or inconclusive evidence as a limitation, not as proof of fit or defect.

## Steelman (`EVAL-16`)

The builder leaves this empty. The reviewer fills it after the structure pass: state the strongest structurally different alternative to the shipped hand, the case for it in its own best terms, and why the shipped structure still wins on this brief. If that case cannot be made, the alternative was never genuinely considered and the structure finding is at least S2.

```text
<strongest rejected alternative, its best case, and why the shipped structure survives it>
```

## Required reviewer output

Write `.considered/<surface-id>/REVIEW.json` from `assets/templates/REVIEW.json`. Each finding must include ID, severity, rule, description, evidence, exact fix, and effort. End with the gate, weighted score if scorable, ordered fix list, working elements, and scope limitations.
