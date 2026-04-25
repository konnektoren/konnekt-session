---
title: Connection Lifecycle
type: architecture-note
tags: [architecture, p2p, signalling, lifecycle]
source: konnekt-session-p2p/src/infrastructure/connection.rs
---

# Connection Lifecycle

Short reference for how peers connect and become session participants.

## Steps

1. Client connects to `wss://match.konnektoren.help/<lobby-id>`.
2. Matchbox assigns a local `peer_id`.
3. Peers exchange SDP/ICE through Matchbox.
4. WebRTC data channel is established.
5. Guest requests sync, host sends snapshot/events.
6. Guest sends `JoinLobby`, host broadcasts signed `GuestJoined`.

## Invariants

- Matchbox relays signalling only.
- Host is authoritative for domain events.
- Peers verify host-signed updates before applying.

## Links

- [[overview|Architecture Overview]]
- [[p2p-flow|P2P Message Flow]]
- [[host-connection-flow|Host Connection Flow]]
- [[guest-connection-flow|Guest Connection Flow]]
- [[../concepts/p2p-signing|P2P Signing]]
