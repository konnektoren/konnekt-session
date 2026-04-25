---
title: Architecture Overview
type: architecture
tags: [architecture, c4, ddd, bounded-contexts, p2p, connection]
source: docs/README.adoc
---

# Architecture Overview

## System Context (C4 Level 1)

```mermaid
graph TD
    User -->|uses| Browser
    Browser -->|loads app| KS[Konnekt Session Client]
    KS -->|SDP/ICE signalling only| Matchbox[Matchbox Signalling]
    Matchbox -->|relay only| KS
```

Matchbox never sees lobby or game state. It only relays signalling frames.

## Connection Planes

```mermaid
flowchart LR
    Host[Host Client] <-->|SDP/ICE via WebSocket| Matchbox[Matchbox]
    Guest[Guest Client] <-->|SDP/ICE via WebSocket| Matchbox
    Host <-->|Signed lobby events + commands| Guest
```

- **Signalling plane:** host/guest ↔ Matchbox (`wss://.../<lobby-id>`)
- **Data plane:** host ↔ guest (WebRTC data channel, signed payloads)

## Connection Handshake (Host + Guest)

```mermaid
sequenceDiagram
    autonumber
    participant H as Host Client
    participant M as Matchbox
    participant G as Guest Client

    H->>M: Connect to wss://match.../<lobby-id>
    M-->>H: Assign peer_id(H)

    G->>M: Connect to same lobby URL
    M-->>G: Assign peer_id(G)

    H->>M: Offer + ICE candidates
    M-->>G: Relay offer + ICE candidates
    G->>M: Answer + ICE candidates
    M-->>H: Relay answer + ICE candidates

    H-->>G: WebRTC data channel established
```

## Join + Initial Sync

```mermaid
sequenceDiagram
    autonumber
    participant G as Guest
    participant H as Host

    G->>H: FullSyncRequest
    H-->>G: FullSyncResponse(snapshot, events)
    G->>H: JoinLobby command
    H-->>G: GuestJoined event (signed)
    G->>G: Verify host signature, apply event
```

For full protocol detail: [[p2p-flow|P2P Message Flow]].

## Bounded Contexts

| Context | Responsibility |
|---------|----------------|
| **Session Management** | Core domain — `Lobby`, `Participant`, `Activity` |
| **P2P Networking** | WebRTC connections, broadcasting, signature verification |
| **Authentication** | Private keys, identity proofs, host-key handling |
| **Signalling** (external) | Matchbox — WebRTC handshake only |

## Crate Structure

```
konnekt-session-core/     ← published library
├── domain/               ← pure business logic, zero dependencies
├── application/          ← use cases / services
├── infrastructure/       ← P2P, Auth, Storage adapters
└── traits/               ← Activity trait for extensibility

konnekt-session-yew/      ← published UI library
├── components/           ← Lobby, ParticipantList, ActivityQueue
└── hooks/                ← use_lobby, use_p2p
```

## Layered Architecture

```mermaid
graph BT
    Domain[Domain]
    Application[Application]
    Infrastructure[Infrastructure]
    UI[Yew UI / CLI / TUI]

    Domain --> Application
    Application --> Infrastructure
    Infrastructure --> UI
```

Domain has **zero** external dependencies.

## Quick Notes

- [[connection-lifecycle|Connection Lifecycle]]
- [[host-connection-flow|Host Connection Flow]]
- [[guest-connection-flow|Guest Connection Flow]]

## See Also

- [[domain-model|Domain Model]]
- [[p2p-flow|P2P Message Flow]]
- [[../concepts/p2p-signing|P2P Signing]]
- [[../concepts/host-delegation|Host Delegation]]
- [[../adr/index|ADR Index]]
- `docs/README.adoc` — full C4 diagrams
