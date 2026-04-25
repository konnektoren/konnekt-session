---
title: Guest Connection Flow
type: architecture-note
tags: [architecture, guest, p2p, sync]
source: konnekt-session-p2p/src/application/runtime/session_loop.rs
---

# Guest Connection Flow

## Guest startup path

1. Connect to signalling URL with known `session_id` / lobby id.
2. Wait for local `peer_id`.
3. Start `SessionLoop` as guest.
4. On `PeerConnected`, request full sync from host.
5. Apply snapshot as domain commands.
6. Submit `JoinLobby` to host.
7. Apply signed `GuestJoined` event from host.

## Guest rules

- Guests do not broadcast authoritative domain events.
- Guests send commands to host (`JoinLobby`, `SubmitResult`, ...).
- Guests verify host-signed updates before state mutation.

## Links

- [[overview|Architecture Overview]]
- [[connection-lifecycle|Connection Lifecycle]]
- [[host-connection-flow|Host Connection Flow]]
- [[p2p-flow|P2P Message Flow]]
