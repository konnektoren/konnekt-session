---
title: Event log reconciliation
type: protocol
status: draft
tags: [protocol, event-log, recovery]
---
# Event log reconciliation

An attempt verifies authority, compares histories and, if they diverge, finds
a common ancestor, selects a history, validates a merge plan and applies it.
It can reject untrusted or invalid input, stop when history is unavailable, or
be cancelled. Matching histories can complete after authority is verified.

[[event-log-reconciliation-diagram|Generated diagram]]

The machine tracks one local attempt. Its caller must verify authority and
history, define and validate fork selection and replay, apply changes safely,
and communicate results. A local completed state does not establish agreement
with another peer. The existing bounded event log has no signed hash chain or
fork comparison. `ReconciliationAttempt` in the application layer accepts an
explicit authority-verification outcome and compares complete caller-verified
logs. Identical histories finish locally; different histories wait for ancestor
discovery and incomplete histories stop as insufficient. V2 can manage one
caller-initiated attempt for a connected peer and cancels it on disconnect.
Applications can offer a history to one peer and request it during an attempt;
requests and responses are signed by the sending peer and bound to a lobby,
peer IDs, the current connection challenge and request ID. Callers must pin
expected keys and complete the existing key-possession exchange. Histories are
limited to 100 events and 256 KiB of JSON; offers are cleared on disconnect.
Key possession and a signed response do not establish host authority or prove
individual events are valid. The receiving caller must verify the history
before comparison. V2 does not automatically verify received histories, choose
a fork or apply a merge. Size limits are checked after the underlying transport
has parsed the incoming message.

Version 1 signed event envelopes and local chain verification are available in
`application::signed_history`. They sign the exact payload bytes and link
envelopes with SHA-256. Verification requires caller-trusted author keys and,
for a truncated history, a caller-trusted checkpoint. V2 hosts can opt in
before lobby creation (including through the V2 builder) to record a bounded
signed chain. The recorder retains the last evicted hash as a checkpoint and
stops after an error; applications must persist the events and checkpoint.
A valid chain does not decide which author had host authority, and the current
lobby-created event is not a complete replayable snapshot. V2 can offer a
snapshot of its recorded signed segment over the authenticated history
exchange. The received checkpoint is untrusted: verify using an independently
trusted checkpoint and author keys. Legacy history exchange remains available.

## Remaining work

- [x] Exchange explicitly offered, bounded histories between connected peers.
- [x] Define and verify version 1 signed event envelopes and hash links.
- [x] Record a bounded signed history in newly created V2 host sessions.
- [x] Exchange bounded signed history segments over the authenticated channel.
- [ ] Bind participant identity and persist trusted checkpoints; integrate verification and comparison.
- [ ] Define snapshots and common-ancestor discovery.
- [ ] Specify fork choice and replay rules, including unavailable history.
- [ ] Implement atomic merge application and peer coordination.
- [ ] Connect reconciliation to recovery and host failover.
- [ ] Test diverging histories, invalid authority and truncated logs across peers.
