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

## Steelman (`EVAL-16`)

The builder leaves this empty. The reviewer fills it after the structure pass: state the strongest structurally different alternative to the shipped hand, the case for it in its own best terms, and why the shipped structure still wins on this brief. If that case cannot be made, the alternative was never genuinely considered and the structure finding is at least S2.

```text
<strongest rejected alternative, its best case, and why the shipped structure survives it>
```

## Required reviewer output

Write `.considered/<surface-id>/REVIEW.json` from `assets/templates/REVIEW.json`. Each finding must include ID, severity, rule, description, evidence, exact fix, and effort. End with the gate, weighted score if scorable, ordered fix list, working elements, and scope limitations.
