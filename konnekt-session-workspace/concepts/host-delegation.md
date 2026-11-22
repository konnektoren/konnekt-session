---
title: Host Delegation
type: concept
tags: [concept, host, p2p, protocol]
source: docs/02-discover.adoc
---

# Host Delegation

Mechanism ensuring the [[../domain/lobby|Lobby]] always has exactly one host.

## Automatic Delegation (Disconnect)

```mermaid
flowchart TD
    A[Host disconnects] --> B[Peers detect heartbeat timeout — 10s]
    B --> C[Grace period — 30s total]
    C --> D[Oldest active guest broadcasts HostDelegated]
    D --> E[Peers verify: oldest active join timestamp]
    E --> F[New host established]
```

## Manual Delegation

Host can explicitly transfer role to any **active** guest via `DelegateHost` command. Delegating to a spectator fails with `CannotDelegateToSpectator`.

## Host Reclaim

Original host can reclaim role after reconnection by presenting its **stored host key**:

```
Original Host → present host key → verified by peers → role restored
```

## Constraints

- Grace period: **30 seconds** (do not change without consulting the protocol design).
- Election: **oldest active guest** by join timestamp wins, ties broken by participant ID — deterministic, no voting needed.
- Spectators are never elected or delegated to. If only spectators remain, there is no candidate (`LobbyError::EmptyLobby`).
- Stored key must match the key used when the lobby was created.

## See Also

- [[../architecture/p2p-flow|P2P Message Flow]] — HostDelegated sequence
- [[../domain/participant|Participant]] — LobbyRole
- [[participation-modes|Participation Modes]] — why spectators never become host
- [[../protocols/host-failover|Host failover]] — candidate order
- `konnekt-session-core/src/infrastructure/auth/`
