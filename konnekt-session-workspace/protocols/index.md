---
title: Protocols
type: index
tags: [protocol, index]
---

# Protocols

Multi-step protocols between players and services. Each protocol is a sans-IO state machine built with the `state-machines` crate ([[../adr/0025|ADR-0025]]). Its Mermaid diagram should be generated from the `state_machine!` definition once it is implemented.

| Protocol | Status | Crate | Summary |
|---|---|---|---|
| [[synchronized-start\|Synchronized start]] | partial | `konnekt-session-core` | Music retries and acknowledges count-in announcements; shared scheduling remains open |
| [[clock-probe\|Clock probe]] | partial | `konnekt-session-core` | Music live link matches and validates one outstanding probe |
| [[player-readiness\|Player readiness]] | partial | `konnekt-session-core` | Music lobby binds per-player readiness to a selection revision; shared API and ordering remain open |
| [[activity-run\|Activity run]] | partial | `konnekt-session-core` | Complete or cancel started runs; queued activities remain outside the machine |
| [[command-outcome\|Command outcome]] | partial | `konnekt-session-p2p` | Track host-local execution; guest outcomes remain planned |
| [[peer-grace\|Peer grace period]] | partial | `konnekt-session-p2p` | Track disconnect, reconnect, and timeout in the legacy peer registry |
| [[identity-verification\|Identity verification]] | partial | `konnekt-session-p2p` | Exchange peer-bound proofs; admission and authorization are not enforced |
| State synchronization | partial | `konnekt-session-p2p` | Observe initial snapshot, detected gaps, recovery, and disconnects in V2 |
| [[rating-match\|Rating Match]] | draft | `konnekt-session-rating` | Two players co-sign every round and the match record; disconnect, forfeit, resume |
| Matchmaking Search | planned | `konnekt-session-matchmaking` | Search, widening window, match found / no match / ghost offer, expiry |

V2 requests a snapshot while awaiting initial synchronization. When it detects a
sequence gap, it requests the missing range and retries while recovery is in
progress. Retries are currently based on session polls, not elapsed time. If the
host no longer has the missing messages, recovery can remain pending: snapshot
fallback is not yet safe because snapshots do not contain complete run state.
The synchronization machine does not authenticate host messages.

## Conventions

- One file per protocol: scope, inputs, messages, state machine, states, events, flows, invariants, open questions.
- Events are named after what happened (`round_finished`, `opponent_left`), not after commands.
- Guards go in square brackets in diagrams: `round_signed [last round]`.
- Actions (sign, send, store) run in the protocol service **after** a transition, never inside the machine.

<!-- generated-protocol-diagrams:start -->
## Generated state machines

Run `just protocol-diagrams` to update these diagrams.

### Asset transfer

```mermaid
stateDiagram-v2
    [*] --> Prepared
    Completed --> [*]
    TimedOut --> [*]
    Cancelled --> [*]
    Prepared --> Transferring : started
    Transferring --> AwaitingAcknowledgement : chunks_complete
    AwaitingAcknowledgement --> Completed : acknowledged
    Transferring --> TimedOut : timed_out
    AwaitingAcknowledgement --> TimedOut : timed_out
    Prepared --> Cancelled : cancelled
    Transferring --> Cancelled : cancelled
    AwaitingAcknowledgement --> Cancelled : cancelled
```

### Synchronized start

```mermaid
stateDiagram-v2
    [*] --> Prepared
    Acknowledged --> [*]
    TimedOut --> [*]
    Cancelled --> [*]
    Prepared --> AwaitingAcknowledgements : announced
    AwaitingAcknowledgements --> Acknowledged : acknowledged
    AwaitingAcknowledgements --> TimedOut : timed_out
    Prepared --> Cancelled : cancelled
    AwaitingAcknowledgements --> Cancelled : cancelled
```

### Clock probe

```mermaid
stateDiagram-v2
    [*] --> Prepared
    Sampled --> [*]
    TimedOut --> [*]
    Cancelled --> [*]
    Prepared --> AwaitingReply : sent
    AwaitingReply --> Sampled : sample_validated
    AwaitingReply --> TimedOut : timed_out
    Prepared --> Cancelled : disconnected
    AwaitingReply --> Cancelled : disconnected
```

### Player readiness

```mermaid
stateDiagram-v2
    [*] --> NotReady
    NotReady --> Ready : ready
    Ready --> Ready : ready
    Ready --> NotReady : not_ready
    NotReady --> NotReady : not_ready
    Ready --> NotReady : reset
    NotReady --> NotReady : reset
```

### Activity run

```mermaid
stateDiagram-v2
    [*] --> InProgress
    Completed --> [*]
    Cancelled --> [*]
    InProgress --> Completed : completed
    InProgress --> Cancelled : cancelled
```

### Command outcome

```mermaid
stateDiagram-v2
    [*] --> Prepared
    Accepted --> [*]
    Rejected --> [*]
    Abandoned --> [*]
    Prepared --> Pending : sent
    Pending --> OutcomeUnknown : response_lost
    OutcomeUnknown --> Pending : retried
    Pending --> Accepted : accepted
    OutcomeUnknown --> Accepted : accepted
    Pending --> Rejected : rejected
    OutcomeUnknown --> Rejected : rejected
    Prepared --> Abandoned : abandoned
    Pending --> Abandoned : abandoned
    OutcomeUnknown --> Abandoned : abandoned
```

### Request execution

```mermaid
stateDiagram-v2
    [*] --> Pending
    Accepted --> [*]
    Rejected --> [*]
    Pending --> Accepted : accepted
    Pending --> Rejected : rejected
```

### Identity verification

```mermaid
stateDiagram-v2
    [*] --> AwaitingChallenge
    Verified --> [*]
    Rejected --> [*]
    Expired --> [*]
    Cancelled --> [*]
    AwaitingChallenge --> AwaitingProof : challenge_issued
    AwaitingProof --> Verified : proof_verified
    AwaitingProof --> Rejected : proof_rejected
    AwaitingProof --> Expired : timed_out
    AwaitingChallenge --> Cancelled : disconnected
    AwaitingProof --> Cancelled : disconnected
```

### Peer grace period

```mermaid
stateDiagram-v2
    [*] --> Connected
    TimedOut --> [*]
    Connected --> Disconnected : disconnected
    Disconnected --> Connected : reconnected
    Disconnected --> TimedOut : grace_expired
```

### Join session

```mermaid
stateDiagram-v2
    [*] --> Connecting
    Connecting --> Joining : connected
    Joining --> Synchronizing : admitted
    Joining --> AwaitingAdmission : snapshot_applied
    Synchronizing --> Ready : synchronized
    AwaitingAdmission --> Ready : admission_confirmed
    Connecting --> Failed : failed
    Joining --> Failed : failed
    Synchronizing --> Failed : failed
    AwaitingAdmission --> Failed : failed
    Joining --> Disconnected : disconnected
    Synchronizing --> Disconnected : disconnected
    AwaitingAdmission --> Disconnected : disconnected
    Ready --> Disconnected : disconnected
    Failed --> Connecting : retry
    Disconnected --> Connecting : retry
```

### State synchronization

```mermaid
stateDiagram-v2
    [*] --> AwaitingSnapshot
    AwaitingSnapshot --> Current : snapshot_applied
    Recovering --> Current : snapshot_applied
    Current --> Recovering : gap_detected
    Recovering --> Current : gap_recovered
    Recovering --> AwaitingSnapshot : snapshot_required
    AwaitingSnapshot --> Disconnected : disconnected
    Current --> Disconnected : disconnected
    Recovering --> Disconnected : disconnected
    Disconnected --> AwaitingSnapshot : reconnected
```

### Rating match

```mermaid
stateDiagram-v2
    [*] --> ExchangingSnapshots
    Rated --> [*]
    Forfeited --> [*]
    Disagreement --> [*]
    Aborted --> [*]
    ExchangingSnapshots --> Playing : snapshots_verified
    ExchangingSnapshots --> Aborted : snapshot_rejected
    ExchangingSnapshots --> Aborted : snapshot_timeout
    Playing --> SigningRound : round_finished
    SigningRound --> Playing : round_signed [rounds_left]
    SigningRound --> SigningMatch : round_signed
    SigningRound --> Disagreement : round_mismatch
    SigningMatch --> Rated : match_signed
    SigningMatch --> Disagreement : match_mismatch
    Active --> WaitingForOpponent : opponent_left
    SigningRound --> WaitingForOpponent : signature_timeout
    SigningMatch --> WaitingForOpponent : signature_timeout
    WaitingForOpponent --> Active : opponent_rejoined
    WaitingForOpponent --> Forfeited : rejoin_timeout [has_signed_round]
    WaitingForOpponent --> Aborted : rejoin_timeout

    state Active {
        [*] --> Playing
        Playing
        SigningRound
        SigningMatch
    }
```
<!-- generated-protocol-diagrams:end -->
