---
title: Host failover and reclaim
type: protocol
status: draft
tags: [protocol, host, recovery]
---
# Host failover and reclaim

Each peer observes host loss, the grace period, candidate selection and
synchronization. If no candidate is available, election can be retried. A
returning host may request reclaim; accepted reclaim also requires
synchronization. The current host can return before a transfer is selected.

[[host-failover-diagram|Generated diagram]]

The machine is local control state. A transition does not authenticate a host,
elect a candidate, grant authority or establish agreement with other peers.
The caller must establish these conditions before reporting events. Candidate
loss during synchronization returns to waiting for an election.

V2 guests track the peer that supplied their first applied snapshot. A
disconnect starts a grace period (30 seconds by default, configurable before
the first snapshot); expiry enters waiting for a
candidate, without promoting anyone. A later applied snapshot from the same
connected peer restores the observation. This peer is not authenticated, and
reconnections with a new peer ID cannot yet be recognized as the same host.
Callers can inspect `SessionLoopV2::host_failover_state()`; progress depends on
polling the session loop.

## Remaining work

- [ ] Specify deterministic eligibility, tie-breaking and split-brain handling.
- [ ] Verify participant identity and define reclaim authorization and policy.
- [ ] Transfer authority and synchronize complete state and history in V2.
- [ ] Define the treatment of active runs and reconnecting hosts during synchronization.
- [ ] Add multi-peer failover and reclaim scenarios.
