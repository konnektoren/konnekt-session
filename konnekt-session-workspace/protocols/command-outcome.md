---
title: Command outcome
type: protocol
status: draft
---

# Command outcome

Tracks one caller's command request. Sending only moves the request to `Pending`;
the host must process it before an accepted or rejected outcome is reported.
If a reply is lost, `OutcomeUnknown` allows a late reply or retry. `Abandoned`
means the caller stopped tracking the request; it does not cancel execution.

`SessionLoopV2::submit_tracked_command` tracks host-local commands through
domain execution. `command_outcome` reports pending, accepted or rejected and
includes the rejection reason; `forget_command_outcome` releases retained
results without cancelling execution. Guest tracked commands are not supported.
Acceptance confirms host-side execution, not delivery or application by peers.
The existing `submit_command` API only queues or sends a command; a successful
return is not acceptance.

Before exposing real outcomes:

- Assign one request ID per guest command and retain it across retries.
- Correlate host execution results with that ID and return accepted or rejected
  outcomes only after domain processing. Queue failures are separate errors.
- Deduplicate retries at the host and return the recorded result without
  executing the command again; define retention and behavior after eviction.
- Authenticate and authorize requests and replies, bind each ID to its sender
  and command, and reject conflicting reuse of an ID.
- Define timeout, reconnect, late-response and abandonment policies; a timeout
  must never be presented as proof that the host did not execute the command.

The [generated diagram](command-outcome-diagram.md) comes from the state machine.
