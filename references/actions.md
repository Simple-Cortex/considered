# Action Definition Reference

Operating rules for scoping, tiering, labeling, and stating actions in a design. Cross-reference rule IDs against the reviewer agent and scripts.

Contents: consequence-to-friction ladder, scope ladder, tier assignment procedure, tie-break scoring, emphasis ladder, placement/order/grouping, labels, states, destructive actions, Action Table artifact.

---

## 0. Consequence dictates friction

Friction is a design material, not a safety default. Every action carries a consequence of error, and the friction between intent and effect must be proportional to it. Add friction deliberately where the consequence is high; remove it aggressively where the consequence is low. Uniform friction is a failure in both directions: it trains users to dismiss confirmations they should read, and it taxes the routine actions they take all day.

| Consequence of error | Required friction |
| --- | --- |
| Trivial and reversible | None. Apply optimistically with an undo affordance |
| Moderate, reversible | Inline confirm, or an undo toast with adequate duration |
| Significant, reversible | Explicit confirmation naming the object and the effect |
| Irreversible or externally visible | Confirmation requiring typed or explicit re-affirmation, plus a preview of the effect |

This ladder is assigned before tier and treatment, and it is orthogonal to both: consequence sets the friction, scope sets the emphasis. A frequent low-consequence action at S1 keeps a filled button and no confirmation. A rare irreversible action at S4 sits in an overflow menu and still demands re-affirmation. Section 7 applies the ladder's bottom two rows to destructive actions specifically; the rows above it govern every other action on the surface.

## 1. Scope before priority

"One primary button per screen" is wrong. Correct rule: one primary action per scope, with emphasis decreasing as scope narrows. The error is not having several primaries; it's rendering them at the same weight so a row-level action shouts as loud as a page-level commitment.

### Scope ladder

| Level | Scope | Example | Max high-emphasis actions | Ceiling treatment |
| --- | --- | --- | --- | --- |
| S0 | Application shell | Global "Create" | 1 | Filled accent, or FAB |
| S1 | Page or view | "Publish report" | 1 | Filled accent |
| S2 | Zone, panel, section | "Add widget" | 1 | Outlined or tonal |
| S3 | Card or object block | "View details" | 1 | Subtle or text |
| S4 | Table row / list item | "Assign", "Archive" | 0 | Icon, text link, or overflow menu |
| S5 | Inline field or cell | "Edit", "Copy" | 0 | Icon on hover, with accessible name |

`ACT-03` (monotonic emphasis decay): the highest-emphasis action at level Sn must be strictly less than the highest-emphasis action at level Sn-1. A card's primary can never be a filled accent button if the page's primary also is.

`ACT-01`: at most one filled, high-emphasis action per scope; at most 2 filled high-emphasis actions visible in any single viewport regardless of scope count.

`ACT-04`: if a scope contains more than 3 actions, actions beyond the top 2 move into an overflow menu, sorted by frequency.

## 2. Tier assignment procedure

Run per action, mechanically, first match wins. Assign the consequence of error and its friction (section 0) alongside the tier; neither determines the other.

```
STEP 1: Is it navigation (moves user to a different context, changes nothing)?
        -> LINK, not a button. [ACT-07] Exit.

STEP 2: Is it destructive or irreversible?
        -> Mark risk = DESTRUCTIVE. Continue to step 3 for tier, but apply
           danger treatment/confirmation (section 5). May NOT become PRIMARY
           unless destruction is the scope's entire purpose. [ACT-08]

STEP 3: Completes the scope's main job, AND safe/reversible, AND most
        frequently taken path?
        -> PRIMARY. Max one per scope. [ACT-01] Exit.

STEP 4: Meaningful alternative to primary, regular supporting action, or
        escape from primary flow (Cancel, Back, Discard)?
        -> SECONDARY. Exit.

STEP 5: Optional, occasional, contextual, or scoped to a single object
        rather than the whole view?
        -> TERTIARY. Exit.

STEP 6: Used in <~5% of sessions, or advanced/admin/config?
        -> OVERFLOW (menu). [ACT-04] Exit.

STEP 7: Otherwise -> shouldn't be on this surface. Remove or move to
        settings/detail context. [ACT-09]
```

### Tie-breaking score

Use when two actions in the same scope both plausibly claim primary. Equal scores means neither is primary; both become secondary.

| Factor | Weight | 0 | 1 | 2 | 3 |
| --- | --- | --- | --- | --- | --- |
| Job completion | x3 | Unrelated to scope job | Tangential | Advances it | Completes it |
| Frequency | x2 | Rare | Occasional | Common | Every session |
| Workflow position | x2 | Off-path | Optional branch | On-path | Terminal step |
| Reversibility | x2 | Irreversible | Hard to undo | Undoable w/ effort | Trivially undoable |
| User expectation | x1 | Surprising | Neutral | Expected | Strongly expected |

`primary_score = 3*job + 2*frequency + 2*position + 2*reversibility + 1*expectation` (max 30).

Interpretation: 24+ strong primary. 16-23 weak primary, verify no other action scores higher. Below 16, should not be primary.

## 3. Emphasis ladder

| Rank | Treatment | Tier served | Typical spec |
| --- | --- | --- | --- |
| 1 | Filled accent | Primary (S0, S1) | Solid accent bg, white label |
| 2 | Filled tonal | Primary (S2) or strong secondary | Low-sat accent bg, accent label |
| 3 | Outlined | Secondary | Transparent, 1px border, text-primary label |
| 4 | Subtle / ghost | Secondary or tertiary | Transparent, no border, hover bg |
| 5 | Text button | Tertiary | Label only, accent/primary text color |
| 6 | Link | Navigation, tertiary | Underlined or accent text, inline |
| 7 | Icon-only | Tertiary (S4, S5) | Requires accessible name + tooltip |
| 8 | Overflow menu | Everything else | Kebab or "More" trigger |

`ACT-10` (adjacent-rank pairing): actions placed side by side should be at least 1 rank apart and no more than 3. Filled next to filled = Primary Inflation. Filled next to link hides the alternative (acceptable for Cancel, not for a genuine alternative path).

`ACT-11` (secondary weight caution): outlined secondary reads heavier than expected. In dense views, prefer subtle/text treatments for secondaries so primary retains dominance.

**Dense-surface exception:** in toolbars, table rows, filter bars, card footers, no action should use ranks 1 or 2. Use ranks 4-8. A filled button on a dense surface usually signals a misplaced page-level action.

## 4. Placement, order, grouping

`ACT-12` (placement by container):

| Container | Primary position | Rationale |
| --- | --- | --- |
| Page header | Right end of header row | Title stays left-aligned |
| Full-page form | Left, end of form flow | Follows reading path to completion |
| Modal / dialog footer | Right, secondary to its left | Conventional confirm position |
| Drawer / side panel footer | Right, or full-width stacked if narrow | Follows dialog convention |
| Card footer | Left, low emphasis | Reads as content continuation |
| Table row | Right end, icon or overflow | Keeps data columns scannable |
| Empty state | Center, below explanatory text | Action is the point of the state |

Mirror horizontal positions in RTL locales.

`ACT-13` (ordering): order by workflow sequence when one exists, else descending frequency. Never alphabetical, never by label length.

`ACT-14` (grouping): group actions only when they operate on the same object at the same scope. A false visual group is an IA error, not a styling error.

`ACT-15` (destructive separation): separate destructive from constructive actions with a divider, distinct position, or overflow placement. Never place destructive immediately adjacent to the primary confirm action without separation.

## 5. Action labels

`ACT-05`: verb plus object, three words or fewer, user's vocabulary.

### Banned / bad label patterns

| Bad | Good | Why |
| --- | --- | --- |
| Submit | Send invitation | Names the outcome |
| OK | Save changes | Ambiguous confirmation is the classic dialog failure |
| Yes / No | Delete project / Keep project | Answer must restate the action, not the question |
| Continue | Review order | Tells the user where they land |
| Manage | Edit permissions | "Manage" is a category, not an action |
| Click here | View report | Never describe the input device |
| Update | Apply filters | Describe effect on the user's world |

Additional rules:
- Sentence case, never title case or all caps (hurts scan speed and screen-reader pronunciation).
- Dialog confirm label must restate the specific action (users skip body text).
- Never write a label that only makes sense after reading a paragraph above it.
- Loading labels describe the in-progress state ("Saving..."), never a spinner that fully replaces the label (causes layout shift, loses context).

## 6. Action states

`ACT-16`: every action must define all states below; an action with only a default state is a specification defect.

| State | Requirement |
| --- | --- |
| Default | Meets contrast requirements, at rest |
| Hover | Visible bg/border change; pointer devices only |
| Focus | Visible focus ring, min 3:1 contrast, never removed |
| Active / pressed | Distinct from hover, immediate feedback |
| Loading | In-place, preserves button width, disables re-entry, announces to assistive tech |
| Disabled | Only when reason is discoverable (see `ACT-17`) |
| Success | Optional; brief in-place confirmation for low-consequence actions |
| Error | Inline message adjacent to action, cause + remedy |

`ACT-17` (never disable silently): choose one: (1) keep enabled, explain failure on activation (preferred), or (2) disable with tooltip/adjacent text explaining exactly what's required to enable it. Never disable a control whose enabling condition isn't visible on the same screen.

`ACT-18` (optimistic vs pessimistic): trivially reversible actions apply optimistically with undo affordance. Consequential actions show pending state, confirm on server response. Never show optimistic success for an irreversible action.

`ACT-20` (toggles are state, flows get setup affordances): render an action as an on/off toggle only when flipping it takes complete effect immediately. An action that requires configuration steps — connecting a provider, verifying a domain, any multi-field setup — renders as a setup affordance: a card or button labeled with the action ("Configure", "Verify") plus its current status. A toggle on a configuration flow asserts a completion the flip cannot deliver; shipped settings surfaces use setup cards with explicit status for exactly this reason.

## 7. Destructive actions

Risk is orthogonal to tier: an action has both a tier (prominence) and a risk level (treatment/friction). This table is the section 0 ladder with the visual treatment attached; the friction column must agree with the consequence assigned there.

| Risk | Treatment | Friction |
| --- | --- | --- |
| Safe, reversible | Normal tier treatment | None |
| Reversible, moderate impact | Normal treatment | Undo toast, min 8s (15s if multi-select) |
| Hard to reverse | Danger color at tier's emphasis level | Confirmation dialog naming the object |
| Irreversible | Danger treatment | Confirmation requiring explicit re-affirmation + consequence/scope statement |

`ACT-08`: a destructive action is never the default primary of a scope unless destruction is that scope's entire purpose (e.g. "Delete" is correctly primary inside a delete confirmation dialog; "Delete project" is tertiary in a separated danger zone on a settings page).

`ACT-19` (confirmation content) must state:
1. The exact object, by name.
2. The exact effect, including scope and cascade (e.g. "This will also remove 14 linked tasks").
3. Whether it is reversible, and how.
4. Button labels that restate the action, never "Yes"/"No".

## 8. Artifact A6: Action Table

Required columns: ID, Action, Scope, Tier, Treatment, Risk, Label, Disabled rule, Loading, Placement.

Validation checks:
- Every row's friction matches its consequence of error, and no two rows at different consequences carry identical friction (section 0).
- At most one Primary per distinct scope value (`ACT-01`).
- Filled accent treatments in one viewport: at most 2 (`ACT-01`).
- Emphasis rank decreases monotonically as scope level increases (`ACT-03`).
- No destructive action carries Primary tier outside a destruction-purposed scope (`ACT-08`).
- Every row's label matches verb + object (`ACT-05`).
- Every row specifies a disabled rule and a loading behavior (`ACT-16`, `ACT-17`).
- Any scope with more than 3 actions places the surplus in overflow (`ACT-04`).
