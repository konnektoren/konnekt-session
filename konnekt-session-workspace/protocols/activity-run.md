---
title: Activity run
type: protocol
status: draft
tags: [protocol, activity]
---
# Activity run

The host starts a run from a queued activity. The aggregate snapshots required
submitters and reports completion when all have submitted. It cancels the run
when all required submitters are removed or the host cancels. Results already
submitted remain available after either outcome.

The state machine controls transitions after a run starts. Queue management,
result validation, participant membership, and the effects of ending a run
remain outside it. A machine does not authenticate the host or replicate state.
The existing serialized `RunStatus` is retained and restored before transitions.

See [[activity-run-diagram|generated diagram]].

## Remaining work

- [ ] Include active and finished runs in synchronization snapshots.
- [ ] Authenticate and authorize activity commands and events.
- [ ] Define retention and recovery of results across reconnects.
