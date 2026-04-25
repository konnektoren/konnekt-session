---
title: Host Connection Flow
type: architecture-note
tags: [architecture, host, p2p, session]
source: konnekt-session-p2p/src/application/runtime/runtime_builder.rs
---

# Host Connection Flow

## Host startup path

1. Create lobby and host participant in domain.
2. Connect signalling socket to `wss://.../<lobby-id>`.
3. Wait until local `peer_id` is assigned.
4. Start `SessionLoop` as host.
5. On `PeerConnected`, send full snapshot to that peer.
6. Process incoming guest commands and broadcast signed domain events.

## Host responsibilities

- Keep canonical lobby state.
- Assign event sequence numbers.
- Broadcast signed events.
- Trigger delegation protocol when host changes.

## Links

- [[overview|Architecture Overview]]
- [[connection-lifecycle|Connection Lifecycle]]
- [[guest-connection-flow|Guest Connection Flow]]
- [[../concepts/host-delegation|Host Delegation]]
