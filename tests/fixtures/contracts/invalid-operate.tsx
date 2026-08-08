// CONSIDERED-CONTRACT v1
// THESIS:     A collection of settings.
// DECISION:   An administrator decides a setting within a minute; being wrong costs disruption.
// MODE:       operate
// QUESTIONS:
//   Q1 [P0] What needs attention? -> A
//   Q2 [P1] What can be changed? -> B
//   Q3 [P1] What else is available? -> B
// ZONES:
//   A "Overview" :: Q1 :: status
//   B "Metrics" :: Q3 :: status
// HIERARCHY:
//   P0 status, setting
//   P1 another-setting
// ACTIONS:
//   S0 secondary "View audit" safe
//   S1 primary "Delete account" destructive
//   S1 secondary "Update" safe
//   S2 primary "Archive records now today" safe
// DELETED:    none
// ROLL:       triage-board,plain-direction,no-replay-key

export const fixture = 'intentionally invalid contract';
