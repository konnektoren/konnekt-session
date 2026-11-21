---
title: Protocols
type: index
tags: [protocol, index]
---

# Protocols

Multi-step protocols between players and services. Each protocol is a sans-IO state machine built with the `state-machines` crate ([[../adr/0025|ADR-0025]]). Its Mermaid diagram should be generated from the `state_machine!` definition once it is implemented.

| Protocol | Status | Crate | Summary |
|---|---|---|---|
| [[rating-match\|Rating Match]] | draft | `konnekt-session-rating` | Two players co-sign every round and the match record; disconnect, forfeit, resume |
| Matchmaking Search | planned | `konnekt-session-matchmaking` | Search, widening window, match found / no match / ghost offer, expiry |

## Conventions

- One file per protocol: scope, inputs, messages, state machine, states, events, flows, invariants, open questions.
- Events are named after what happened (`round_finished`, `opponent_left`), not after commands.
- Guards go in square brackets in diagrams: `round_signed [last round]`.
- Actions (sign, send, store) run in the protocol service **after** a transition, never inside the machine.

<!-- generated-protocol-diagrams:start -->
## Generated state machines

Run `just protocol-diagrams` to update these diagrams.

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
