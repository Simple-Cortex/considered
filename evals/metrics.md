# Metric definitions

Metric version: `1.0.0`. Compute all metrics after blind labels are decoded. Never substitute a missing observation with a zero, a tie, or an assumed failure.

## Notation

For each matched, reviewed pair `i`:

- `c_i` is the Considered output and `b_i` is the baseline output.
- `w_i` is pairwise outcome: `1` if Considered wins, `0.5` if tie, `0` if baseline wins. Abstentions are excluded from the denominator and counted separately.
- `S(x)` is the reviewer-weighted rubric score for output `x`, from 0 to 140.
- `F_s(x)` is the count of findings at severity `s` for output `x`.
- `U(x)` is `1` when the reviewer flags one or more unsupported material claims, otherwise `0`.
- `G(x)` is the gate outcome recorded by the reviewer: Ship, Revise, or Redesign.

Aggregate reviewer-level observations only after preserving individual review rows. For a scenario-level summary, use the mean across reviewers who rated both outputs. For a cross-scenario result, each scenario receives equal weight unless the report explicitly registers a different weighting before collection.

## Primary metric

### Blinded paired preference

```text
preference_rate = sum(w_i) / count(non-abstaining matched pairwise reviews)
net_preference = (wins_considered - wins_baseline) / count(non-abstaining matched pairwise reviews)
```

Report both. `preference_rate = 0.50` is neutral under the half-tie convention. Also report raw Considered wins, baseline wins, ties, abstentions, scenarios, and reviewers. Do not label a rate as significant without a stated uncertainty method and its assumptions.

## Secondary metrics

### Rubric score delta

```text
score_delta_i = S(c_i) - S(b_i)
mean_score_delta = mean(score_delta_i)
```

Also report the median delta and the per-dimension deltas D1 through D9. A positive total does not override a regression in a floor-gated dimension.

### Floor-gated dimension rate

For D1, D2, D3, and D7:

```text
floor_pass_rate(condition, dimension) =
  count(score >= 3) / count(scored outputs in that condition)
```

Show numerator and denominator for each dimension and condition.

### Severity delta

```text
severity_delta_s = mean(F_s(c_i) - F_s(b_i))
blocker_free_rate(condition) = count(F_S1(x) = 0) / count(reviewed outputs)
```

Keep S1 through S4 separate. Do not merge a reduction in minor findings with blocker safety.

### Unsupported-claim rate

```text
unsupported_claim_rate(condition) = sum(U(x)) / count(reviewed outputs)
```

A material unsupported claim is a customer, metric, testimonial, capability, policy, price, availability, or outcome not supplied by the scenario. Report the number and a short redacted description of each flagged claim.

### Gate distribution

```text
ship_rate(condition) = count(G(x) = Ship) / count(reviewed outputs)
revise_rate(condition) = count(G(x) = Revise) / count(reviewed outputs)
redesign_rate(condition) = count(G(x) = Redesign) / count(reviewed outputs)
```

These reflect reviewer judgment under the rubric. They are not an automated lint result.

### Required evidence completion

```text
evidence_complete_rate(condition) =
  count(successful runs with all required artifact paths) / count(successful runs)
```

This measures collection integrity, not output quality. Display excluded and failed runs separately.

## Rubric scoring

Use the nine dimensions and weights already defined by `references/critique.md`:

| Dimension | Weight | Floor-gated |
| --- | ---: | --- |
| D1 Purpose fidelity | 5 | Yes |
| D2 Information architecture | 5 | Yes |
| D3 Visual hierarchy | 5 | Yes |
| D4 Action clarity | 4 | No |
| D5 Content and context | 4 | No |
| D6 States and resilience | 3 | No |
| D7 Accessibility | 4 | Yes |
| D8 Craft and consistency | 3 | No |
| D9 Responsiveness | 2 | No |

A reviewer assigns 0 through 4 to every dimension, then the weighted score is the sum of `rating × weight`, maximum 140. `not assessable` is not zero. It requires a missing-evidence flag and exclusion from that metric's denominator.

## Optional uncertainty reporting

If an implementation reports confidence intervals, it must record the method, resampling unit, random seed, confidence level, and treatment of ties. Bootstrap at the scenario level, not screenshot level, to avoid treating repeated views of one build as independent evidence. With only ten scenarios, describe intervals as descriptive uncertainty rather than conclusive proof.

## Required result table

Every report must include:

| Metric | Baseline | Considered | Delta or comparison | Denominator | Notes |
| --- | ---: | ---: | ---: | ---: | --- |
| Blinded paired preference | n/a | n/a | n/a | reviewed pairs | wins / ties / losses / abstentions |
| Mean weighted score | | | | scored pairs | plus median |
| D1/D2/D3/D7 floor-pass rate | | | | scored outputs | one row each |
| S1 blocker-free rate | | | | reviewed outputs | raw blockers too |
| Unsupported-claim rate | | | | reviewed outputs | raw flags too |
| Ship/Revise/Redesign | | | | reviewed outputs | three rows or distribution |
| Evidence-complete rate | | | | successful runs | exclusions separately |

Empty fields are the correct state before an evaluation is run. Do not fill them with predicted values.
