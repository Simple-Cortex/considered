# Forms and task flows

Use this branch for a bounded multi-step task, data entry, creation flow, checkout, setup, or settings change. A flow is still one mode. Most flows are `operate`; do not call a marketing prelude part of the same surface merely because it precedes the task.

## Frame a flow

Record these facts in the active `FRAME.md`:

| Need | Capture |
| --- | --- |
| Entry | trigger, prior context, and whether it can resume |
| Primary job | the one result the person is trying to create or change |
| Steps | what each step decides, validates, and unlocks |
| Exit | success destination, next action, and recovery path |
| Risk | destructive, financial, privacy, legal, or irreversible effects |
| Draft behavior | autosave, explicit save, loss warning, and ownership |

Treat missing field copy, a tentative default, or unconfirmed optional data as assumptions when the conservative path is reversible. Ask only if the unknown changes consent, irreversible submission, legal eligibility, access, payment, or the required delivery target.

## Structure a flow

- Give the overall flow one P0: the current required decision or the active object state.
- Use P1 for the essential fields or evidence that completes the current step.
- Put explanation, optional inputs, and examples at P2 or behind disclosure.
- Put history, raw metadata, and advanced settings at P3.
- Keep navigation, progress indicators, help, and utility controls at P4.
- Use one primary action per step scope. A step action must not outweigh a flow-level commitment.
- Write direct exit paths: cancel, save draft, back, and recovery are alternatives, not accidental primary actions.

`FORM-01` (one ask per mobile step): on a mobile flow, each step carries one primary ask. Two credential fields on one screen doubles the typing, forces a keyboard-mode switch, and reads heavier than every shipped signup; three independent blind reviews ranked exactly that step last against gold flows. Splitting a step is cheaper than losing the user inside it — bias toward more, lighter steps on small screens.

`FORM-02` (no step is a dead end): every step of a flow renders a visible way back or out — back control, close, or an explicit statement of what leaving preserves. A first step with no exit traps the user at the moment their commitment is lowest. Back must honor the preservation the flow promises (state what going back keeps).

`FORM-03` (inputs look like inputs): text inputs render a visible boundary or fill, a visible focus state, and a tap target the eye can find before the thumb needs it. Underline-only fields fail all three on mobile; three blind judges flagged the same build's fields as ambiguous. Minimal input styling is a desktop editorial affordance, not a mobile form one.

## Validation and recovery

Specify for each field or step:

| Item | Requirement |
| --- | --- |
| Entry state | default, existing value, or safe prefill with provenance |
| Validation | trigger, plain-language rule, field-level location, and focus behavior |
| Error | how the person fixes it without losing completed work |
| Async work | pending label preserves width and prevents double submission |
| Disabled state | visible same-screen reason, or keep enabled and explain failure on activation |
| Save behavior | optimistic only when safely reversible; otherwise wait for confirmation |
| Exit | confirmation that names the changed object and next available action |

Never hide a required condition behind a disabled primary action. Never use `Submit`, `OK`, or `Continue` when a verb and object can name the real outcome.

## Flow review additions

During `critique` and `harden`, verify step order, keyboard sequence, error return path, focus visibility, draft preservation, back behavior, duplicate-submit prevention, and narrow-width behavior. If a flow needs two equally consequential success signals, split it into separate surfaces or stages.
