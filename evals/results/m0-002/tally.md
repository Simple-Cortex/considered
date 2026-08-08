# m0-002 tally — second full matched evaluation (protocol 1.0.0, catalog 1.0.0)

Skill v0.4.0 (189 rules, calibrated gate 120/140) vs unassisted baseline. Builder
claude-sonnet-5 both arms; reviewers claude-opus / claude-fable-5 / claude-sonnet-5,
blind, three per scenario. Seed 20260803. Staged at fb2edb2/29466bb. Complete record:
`result.json` (22 runs: 20 completed + 2 excluded), 10 packets, 30 forms, check green.

## Headline

| Metric | m0-002 | m0-001 |
| --- | --- | --- |
| Blind pairwise preference (considered) | **24–6** (80%), 0 ties | 26–4 (87%) |
| Scenario majorities | **8–2** (7 unanimous sweeps) | — |
| Mean weighted score (considered vs baseline) | **124.1 vs 113.2** (median 129 vs 112) | — |
| Reviewer Ship gates | **16 vs 4** | — |
| Unsupported-claim flags | **8 vs 9** (par) | 11 vs 5 **against** considered |
| D7 accessibility floor pass | **29/30 vs 18/30** | — |

The honesty result is the story: m0-001's headline preference win came with a 2:1
fabrication deficit; after the honesty-clamp program (HON-01..06 and kin), the
considered arm now fabricates at baseline rate while winning preference and craft.

## Per-scenario (C = considered chosen)

| Scenario | Votes | Majority |
| --- | --- | --- |
| dense-admin-bulk-destructive | CCC | considered |
| dense-chart-dump-rescue | CCC | considered |
| dense-settings-23-actions | bbb | **baseline** |
| dense-sre-console | CCC | considered |
| dense-support-queue | CCC | considered |
| persuade-portfolio | bCb | **baseline** |
| persuade-pricing-page | CCC | considered |
| persuade-saas-landing | CCC | considered |
| read-changelog | CCC | considered |
| read-docs-home | CbC | considered |

Losses: dense-settings (unanimous — the considered build that shipped with all render
checks self-marked blocked under browser contention; reviewers found real interaction
defects the builder could not see) and persuade-portfolio (2–1 — the persistent weak
genre; see the trace below).

## HIER-13 / EVAL-17 causal trace

Both rules trace **positive** in this round (correcting an earlier draft of this
section that misattributed the portfolio finding to EVAL-17):

- **HIER-13** (adjacent tiers separate decisively): cited once in a build decision
  trail (read-docs-home considered REVIEW.json, "HIER-02/HIER-13") — that packet won
  2–1 — and its failure class ("everything reads at one weight") appears in **zero**
  of the 30 forms against considered outputs.
- **EVAL-17** (the polish pass): no artifact citations, but its failure class
  (off-scale spacing, same-role styling drift, tracking/widow defects) also appears in
  **zero** forms against considered outputs. The criticism family both rules were
  minted from in the r4 era did not recur anywhere in the matched evaluation.

The portfolio criticism that DID recur (all three reviewers, independently: "case
modules hold no evidence until interacted with"; "cards expose no project content
beyond title and kicker") belongs to neither rule — it is an evidence-at-rest gap no
persuade rule covered: PERS-02 adjacency was technically satisfied by a one-line
kicker. Minted **PERS-11 — the argument survives collapse** (unanimous 3/3 on one
surface meets the same-day bar): the resting state must carry claim + one decisive
supplied evidence element; disclosure defers depth, never the case. Corpus 189 → 190.

## Deviations and limitations (full detail in environment.json notes)

- Pilot-first per RUNNER; pilot caught and fixed three harness defects pre-run.
- Six early builds superseded uniformly after a baseline builder self-reported reading
  m0-001 archives (session-context leak); confinement sentence added to the invocation
  (RUNNER 29466bb); two excluded run records retained.
- Pricing baseline evidence was re-captured and its packet re-reviewed by fresh
  reviewers after an evidence-server artifact produced unstyled screenshots; all three
  affected forms discarded undecoded (protocol §6; m0-001 precedent).
- **Partial unblinding, recorded not acted on: 14/30 forms** flagged considered-arm
  method residue (rule-code comments, data-zone attributes, renamed design-chain
  references in README prose). Blindness reduced but not guaranteed, exactly as
  protocol anticipates; choice reasons are behavior-grounded. Sanitizer hardening for
  prose/attribute residue is the top harness follow-up.
- Builder debriefs are self-reports; all clean of content contamination, tooling-only
  exceptions recorded per run manifest.

## Settings-loss autopsy

Every S1/S2 the three reviewers logged against the settings considered build is a
render-only defect: `.unsaved-bar` and `.token-form` CSS rules set `display: flex`
unconditionally, overriding the `hidden` attribute (a permanently visible
unsaved-changes bar and a pre-expanded token form); the "YOUR TASK" badge — the one
wayfinding element for the stated job — renders at illegible contrast; the
delete-workspace dialog's focus trap fails in a live keyboard run. The build's own
REVIEW.json marked renderInspection/responsiveInspection/accessibilityInspection
**blocked** (shared-browser contention) and self-gated SHIP anyway. Two consequences
worth encoding: (1) a blocked render check should cap the self-gate at REVISE rather
than permit SHIP-with-caveat — the caveat did not survive contact with reviewers; (2)
the display-override class is detectable statically (reviewer-3 found it by source
inspection) — a lint-source heuristic for unconditional `display` rules on elements
carrying `hidden` would have caught both S2s without a browser. Both are candidates,
not mints — n=1 surface.

## Verdict

A complete, structurally valid matched record. Directionally: the skill now wins
preference (24–6), craft scores (+10.9 mean), and gates (16 vs 4 Ship) without the
fabrication cost that undercut m0-001. PERS-11 (minted from the portfolio finding) awaits
a validation round; the settings candidates await a second signal; unblinding-residue
hardening shipped post-run (blind.mjs rule-code comment drop + tripwire).
