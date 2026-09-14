<!--
CONSIDERED-CONTRACT v1
  THESIS:     Everything is ordered by the queue that needs a human next, not by component type.
              Refuses the category default: a grid of equal metric cards.
  DECISION:   A shift lead decides where to move an agent within 60 seconds; being wrong costs breached response commitments.
  MODE:       operate
  QUESTIONS:
    Q1 [P0] Which queue is about to breach? -> A
    Q2 [P1] How long until the next breach? -> A
    Q3 [P1] Is the backlog growing or shrinking? -> B
    Q4 [P1] Which queue is understaffed? -> B
    Q5 [P2] What is driving volume? -> C
  ZONES:
    A "Needs attention now" :: Q1,Q2 :: breach-count,at-risk-list,time-to-breach
    B "Where the load is going" :: Q3,Q4 :: backlog-trend,load-vs-staffing,priority-split
    C "Floor overview" :: Q5 :: all-queues-strip,channel-volume,agents-online
  HIERARCHY:
    P0 breach-count - it is the only value that changes the lead's next-minute allocation
    P1 at-risk-list, backlog-trend, load-vs-staffing
    P2 time-to-breach, priority-split, all-queues-strip, channel-volume, agents-online
  ACTIONS:
    S1 primary "Reassign agents" safe
    S1 secondary "Export snapshot" safe
    S1 overflow "Alert settings" safe
    S4 tertiary "Move here" reversible
    S4 overflow "Snooze 30 min" reversible
  DELETED:    CSAT - no allocation decision follows; agent leaderboard - answers no ranked question
  ROLL:       urgency-triage-board,overview-and-drill,exception-first,du-bois-paris-data-portraits,cns-k7f2m9x1/0
-->

export const Foo = () => <div>markup that follows the contract comment; must never leak into ROLL</div>;
