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

The music adapter waits for queue acceptance before submitting a tracked
`StartNextRunIf` command. That command checks the expected activity ID at
execution time, so it rejects rather than starting an unrelated queued activity.

V2 holds received guest commands until the host's domain processes them. It
broadcasts accepted commands and does not broadcast rejected ones. This does
not send an outcome back to the requesting guest or authenticate the sender.

The standalone `RequestLedger` reserves request IDs per participant, rejects
conflicting command reuse, and retains pending or completed results until the
ledger is dropped. It refuses new entries at capacity. It is not wired into V2,
does not authenticate participants, and does not survive a host restart; a retry
after restart cannot be assumed safe.

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
