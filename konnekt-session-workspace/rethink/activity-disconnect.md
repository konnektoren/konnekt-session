---
title: Problem — Activity Blocks on Disconnect
type: rethink
tags: [rethink, activity, disconnect, state-machine]
date: 2026-04-25
---

# Problem: Activity Blocks on Disconnect

## Current Rule

Activity completes when **all Active participants** submit a result.

## The Gap

If an Active participant disconnects mid-activity, the activity **never completes**. No ADR or domain doc addresses this.

## Scenarios

```mermaid
stateDiagram-v2
    [*] --> InProgress: ActivityStarted
    InProgress --> Completed: all Active submitted
    InProgress --> Stuck: Active participant disconnects
    Stuck --> [*]: ??? unhandled
```

## Proposed Rules

| Event | Action |
|-------|--------|
| Active participant disconnects during activity | Remove from required submissions |
| All remaining Active have submitted | Activity completes normally |
| All Active participants disconnect | Host cancels activity |

## Mode Change During Activity

Current rule: ParticipationMode cannot change while InProgress.

Clarification needed: does **disconnect** count as switching to Spectating or as removal?

Recommendation: **removal** — cleaner, no ghost participants blocking completion.

## See Also

- [[../domain/activity|Activity]] — state machine
- [[../concepts/participation-modes|Participation Modes]]
- [[../concepts/host-delegation|Host Delegation]] — disconnect detection already exists
