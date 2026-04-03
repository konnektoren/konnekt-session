---
title: P2P Message Flow
type: architecture
tags: [architecture, p2p, websocket, signing]
source: docs/05-connect.adoc
---

# P2P Message Flow

Every state change follows: **Sign → Broadcast → Verify → Apply**

## Create Lobby & Join

```mermaid
sequenceDiagram
    participant H as Host
    participant M as Matchbox
    participant G as Guest

    H->>M: Connect (WebRTC handshake)
    G->>M: Connect (WebRTC handshake)
    M-->>H: Peer discovered
    M-->>G: Peer discovered
    G->>H: JoinRequest (signed)
    H->>H: Verify signature
    H-->>G: LobbyState snapshot
    H->>G: GuestJoined (broadcast)
```

## Start Activity

```mermaid
sequenceDiagram
    participant H as Host
    participant G as Guest (Active)
    participant S as Guest (Spectating)

    H->>G: ActivityStarted (signed)
    H->>S: ActivityStarted (signed)
    G->>H: ResultSubmitted (signed)
    Note over S: Cannot submit
    H->>G: ActivityCompleted (broadcast)
    H->>S: ActivityCompleted (broadcast)
```

## Host Delegation

```mermaid
sequenceDiagram
    participant H as Host (disconnected)
    participant G1 as Guest (oldest)
    participant G2 as Guest

    H--xG1: heartbeat timeout (10s)
    Note over G1,G2: Wait grace period (30s)
    G1->>G2: HostDelegated (signed by oldest)
    G2->>G2: Verify oldest timestamp
```

## P2PMessage Envelope

All messages use a signed envelope:
- **payload** — the lobby event
- **signature** — Ed25519 signature of payload
- **sender_id** — public key of sender

## See Also

- [[../concepts/p2p-signing|P2P Signing]]
- [[../concepts/host-delegation|Host Delegation]]
- `konnekt-session-core/src/infrastructure/p2p/`
