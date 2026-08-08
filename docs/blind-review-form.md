# Blind review form

Use one completed form per reviewer, scenario, and A/B packet. Do not reveal which output used Considered. Review the scenario and required evidence before choosing a winner. If evidence is insufficient, mark it rather than guessing.

## Packet

```text
Evaluation ID: ____________________
Packet ID: ____________________
Scenario ID: ____________________
Reviewer role: ____________________
Conflict with either build or packet assembly?  Yes / No
Suspect you recognized a treatment or builder?  Yes / No
If yes, why: _________________________________________________
Evidence viewed: desktop / mobile / interactions / states / sanitized source / other
```

## Pairwise decision

The question is: **Which output better enables the stated role to complete the stated job using only the supplied facts?**

```text
Choice:  A / B / Tie / Not assessable

Reason for choice in 1 to 3 concrete observations:
1. ____________________________________________________________
2. ____________________________________________________________
3. ____________________________________________________________

Did either output assert a material fact not supplied by the scenario?
A / B / Both / Neither / Cannot assess
Description and evidence: _____________________________________
```

Do not award a win for a more decorative treatment alone. Do not penalize a result for omitting unsupplied proof. A material unsupported fact includes an invented customer, metric, quote, policy, capability, price, availability claim, or outcome.

## Score output A

Rate each dimension 0 to 4. `NA` means the supplied evidence cannot support a judgment. Add a short observation for any 0, 1, 2, or NA.

| Dimension | Weight | Rating 0-4 or NA | Observation |
| --- | ---: | --- | --- |
| D1 Purpose fidelity | 5 | | |
| D2 Information architecture | 5 | | |
| D3 Visual hierarchy | 5 | | |
| D4 Action clarity | 4 | | |
| D5 Content and context | 4 | | |
| D6 States and resilience | 3 | | |
| D7 Accessibility | 4 | | |
| D8 Craft and consistency | 3 | | |
| D9 Responsiveness | 2 | | |
| **Weighted total** | **140** | | |

## Score output B

| Dimension | Weight | Rating 0-4 or NA | Observation |
| --- | ---: | --- | --- |
| D1 Purpose fidelity | 5 | | |
| D2 Information architecture | 5 | | |
| D3 Visual hierarchy | 5 | | |
| D4 Action clarity | 4 | | |
| D5 Content and context | 4 | | |
| D6 States and resilience | 3 | | |
| D7 Accessibility | 4 | | |
| D8 Craft and consistency | 3 | | |
| D9 Responsiveness | 2 | | |
| **Weighted total** | **140** | | |

Score anchors: 0 absent, 1 fundamentally wrong, 2 partially correct with significant gaps, 3 correct with minor gaps, 4 correct and defensible. Score only what can be supported by the packet. For D7, do not infer keyboard or screen-reader behavior from an image; use provided inspectable evidence or mark NA.

## Findings

Record material findings for each output. At first review, seek at least five total findings across the pair when evidence permits, including at least one S2 or higher. This is a prompt to inspect critically, not a reason to invent defects.

```text
Output: A / B
[Finding ID] [S1|S2|S3|S4] [dimension or rule] Description
Evidence:
Suggested fix:

Output: A / B
[Finding ID] [S1|S2|S3|S4] [dimension or rule] Description
Evidence:
Suggested fix:
```

Severity guide: S1 blocks a correct or accessible decision, including unsafe destructive action; S2 materially slows or misleads the decision; S3 adds friction or inconsistency; S4 is polish.

## Reviewer gate

Complete this separately for A and B after scoring. This is a reviewer judgment, not a statement about the hidden condition.

```text
A: Ship / Revise / Redesign / Not assessable
B: Ship / Revise / Redesign / Not assessable

Working well in A: ____________________________________________
Working well in B: ____________________________________________
Missing evidence or caveats: __________________________________
```
