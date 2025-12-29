# AGENTS.md  

## Purpose – Why This Project Exists  

Konnekt Session is a **Rust-centric, peer-to-peer multiplayer-lobby library** whose primary ambition is to **eliminate any central game-server bottleneck**.  

* The only external service required is a **Matchbox signalling node** (currently hosted at `match.konnektoren.help`).  
* All **game state lives on the client** – the lobby, activities, scores, and participant roles are fully replicated on each peer.  
* A **host client is the authoritative source** of truth; its updates are signed and broadcast to every peer, guaranteeing a consistent view without consensus protocols.  
* The design is **extensible** (consuming apps provide activities via trait implementation) and **Web-first** – the UI is written in **Yew** (Rust + WebAssembly) and runs entirely in the browser.

### Core Design Pillars  

| Pillar | What it means for the system |
|--------|------------------------------|
| **Domain-Driven Design** | Clear bounded contexts: `Lobby Management`, `Activity Management`, `Participant Management`, `P2P State Sync`. The `Lobby` aggregate is the aggregate root. |
| **Decentralized State** | No persistent server-side game state. Only connection metadata is stored in-memory; the authoritative lobby state is owned by the host and streamed peer-to-peer. |
| **Host-as-Truth** | Host commands (e.g., *StartActivity*, *KickGuest*) are signed with a private key unique to that client. Peers accept updates only from the host identity. |
| **Host Delegation Protocol** | If the host disconnects for >30s, the oldest remaining guest becomes host. Original host can reclaim role by presenting its stored host key upon reconnection. |
| **WebRTC-based P2P** | Matchbox only handles the WebRTC handshake; the actual **data channel** carries signed lobby events between peers. |
| **Participation Modes** | Guests can be **Active** (can play activities) or **Spectating** (view-only). This is independent of the host/guest role. |

---  

## High-Level Architecture (C4 Overview)  

```
+-----------------------------------------------------------+
|                     Client (Browser)                     |
|  ┌─────────────┐   ┌───────────────────────┐   ┌───────┐ |
|  │   Yew SPA   │←→│  WebSocket (Matchbox)  │←→│ Host  │ |
|  │ (UI, State)│   │  (wss://match.…)      │   │ Role  │ |
|  └─────▲───────┘   └───────▲───────▲───────┘   └───────┘ |
|        │                   │       │                      |
|        │   Signed Lobby   │   P2P Data Channel           |
|        └───────────────────┴───────────────────────┘      |
+-----------------------------------------------------------+

+-----------------------------------------------------------+
|                Matchbox Signalling Service                |
|  (only WebRTC signalling – no game state)                 |
+-----------------------------------------------------------+
```

*The diagram above is deliberately high-level – it shows the separation of concerns without diving into concrete command names or binary formats.*

---  

## Agent Playbook – How to Work on Konnekt Session  

### 1. Mindset  
* Treat **every client as a first-class participant** that holds a full copy of the lobby.  
* The **host** is the *only* source of authoritative updates. All other peers are *receivers* that apply those updates.  
* Authentication is **cryptographic** – every message is signed with the sender's `PrivateKey`; verification is mandatory before state change.  
* **Host vs Participation Mode** – Two independent concerns:
  - **Lobby Role** (Host/Guest) – about authority and management
  - **Participation Mode** (Active/Spectating) – about playing vs watching

### 2. Typical Development Flow  
1. **Spin up the signalling endpoint** you want to test (`match.konnektoren.help`, `helsing.studio`, or local `matchbox_server`).  
2. **Start the SPA** (`trunk serve`) with the appropriate `WEBSOCKET_URL`.  
3. **Open multiple browser windows** – first to connect becomes host automatically; others join as guests.  
4. **Interact** – create a lobby, plan activities, start them. Observe how the host-signed events flow to all windows.  
5. **Test host delegation** – close the host window, watch oldest guest get promoted after 30s timeout.  
6. **Test spectator mode** – toggle a guest to Spectating, verify they can't submit activity results.

### 3. Key Artifacts to Touch  

| Artifact | Location | Why it matters |
|----------|----------|----------------|
| `konnekt-session-core/src/domain/` | Domain entities (`Lobby`, `Participant`, `Activity`), value objects (`LobbyRole`, `ParticipationMode`). | Core DDD building blocks. |
| `konnekt-session-core/src/infrastructure/p2p/` | Message envelope (`P2PMessage`), signing/verification logic. | Guarantees authenticity of every broadcast. |
| `konnekt-session-core/src/infrastructure/auth/` | `PrivateKey` generation, storage, host-key handling. | Enables stable identity and host reclaim. |
| `konnekt-session-yew/src/components/` | Yew UI components that render lobby, activities, participant list. | UI reflects the signed state updates. |
| `docs/*.adoc` | Full architecture documentation (DDD process, C4 diagrams, domain message flows). | Reference for architectural decisions. |

### 4. Module Structure (Subdomain Mapping)

```
konnekt-session-core/          # Published library crate
├── domain/                    # Pure business logic (no I/O)
│   ├── lobby.rs              # Lobby aggregate root
│   ├── participant.rs        # Participant entity
│   └── activity.rs           # Activity entity
├── application/               # Use cases/services
│   ├── lobby_service.rs
│   ├── participant_service.rs
│   └── activity_service.rs
├── infrastructure/            # Adapters (P2P, Auth, Storage)
│   ├── p2p/                  # Matchbox integration, sync
│   ├── auth/                 # Ed25519 signing
│   └── storage/              # LocalStorage wrapper
└── traits/                    # Public API for extensibility
    └── activity.rs           # Activity trait for consuming apps

konnekt-session-yew/           # Published UI crate
├── components/                # Reusable Yew components
│   ├── lobby.rs
│   ├── participant_list.rs
│   └── activity_queue.rs
└── hooks/                     # Yew hooks
    ├── use_lobby.rs
    └── use_p2p.rs

consuming-app/                 # Separate repository (example)
├── src/
│   ├── main.rs               # Entry point
│   └── activities/           # App-specific activities
│       ├── trivia.rs         # Implements Activity trait
│       └── drawing.rs
```

### 5. Testing Strategy  
* **Unit tests** for domain types (`Lobby`, `Activity`, `Participant`).  
* **Integration tests** that spin up a simulated P2P network and verify that host-signed events are correctly propagated and applied.  
* **Property-based tests** for domain invariants (e.g., "lobby always has exactly one host").  
* **End-to-end manual tests** using multiple browser tabs:
  - Close the host window to see host delegation (30s timeout)
  - Reopen to see host reclaim
  - Toggle spectator mode during/between activities
  - Test activity completion with mixed participation modes

### 6. Non-Functional Constraints  
* **No server-side game state** – any persistence must be client-side (e.g., `localStorage`).  
* **All state changes must be signed** by the current host before they are considered valid.  
* **Zero-trust network** – any peer may join; the only gatekeeper is the host's signature.  
* **Layered architecture** – domain has zero dependencies, infrastructure depends on domain.  
* **Published library** – core and yew crates published to crates.io, consuming apps in separate repos.

---  

## Key Domain Concepts (Updated Terminology)

| Term | Definition |
|------|------------|
| **Lobby** | Aggregate root representing a game session with one host and multiple guests |
| **Host** | The lobby organizer with management privileges (can kick guests, start activities, delegate role) |
| **Guest** | A regular participant in the lobby |
| **Active Mode** | Participation mode where guest can submit activity results |
| **Spectating Mode** | Participation mode where guest can only watch (cannot submit results) |
| **Participation Mode** | Whether a guest is actively playing (Active) or watching (Spectating) |
| **Host Delegation** | Transfer of host role from current host to a guest (manual or automatic) |
| **Activity** | A game/task with states: Planned → InProgress → Completed |
| **Activity Result** | Data submitted by an Active participant after completing an activity |

---  

## What **Not** to Do  

* **Don't add new server-side storage** – it breaks the decentralized model.  
* **Don't expose raw WebSocket frames** to the UI; always go through the signed `P2PMessage` envelope.  
* **Don't change the host delegation protocol** without consulting the existing protocol (30s timeout, oldest guest election); it is carefully designed to avoid split-brain scenarios.  
* **Don't commit secret keys** – they are derived from `localStorage` and must stay private to the client.  
* **Don't confuse Host/Guest with Active/Spectating** – they are independent concerns:
  - Host/Guest = lobby role (authority)
  - Active/Spectating = participation mode (playing vs watching)
* **Don't implement activities in the library** – activities are provided by consuming applications via the `Activity` trait.

---  

## Domain Message Flow Examples

### Create Lobby & Join

1. Host creates lobby → `LobbyCreated` event
2. Host connects to Matchbox → WebRTC handshake
3. Guest connects to Matchbox → P2P channel established
4. Guest sends `JoinRequest` (signed)
5. Host verifies signature, adds guest → `GuestJoined` broadcast
6. Host sends `LobbyState` snapshot to guest
7. Guest applies state, syncs up

### Start Activity

1. Host plans activity → `ActivityPlanned` broadcast
2. Host starts activity → `ActivityStarted` broadcast
3. Active guests play activity, submit results → `ResultSubmitted` broadcast
4. When all Active guests submitted → `ActivityCompleted` broadcast
5. Spectating guests watch but cannot submit

### Host Disconnect & Delegation

1. Host connection lost
2. All guests detect heartbeat timeout (10s)
3. Guests wait for grace period (30s total)
4. Oldest guest claims host role → `HostDelegated` broadcast
5. Other guests verify claim (oldest timestamp)
6. If original host returns later → can reclaim with stored host key

---  

## Quick Reference (What the Agent Should Remember)  

* **Goal:** Provide a *decentralized* lobby where a host's signed updates are the **single source of truth** for all participants.  
* **Key Actors:** 
  - `Lobby` aggregate root (owned by host client)
  - `Participant` entity (with `LobbyRole` and `ParticipationMode`)
  - `Activity` entity (with state machine: Planned → InProgress → Completed)
* **Message Flow:** `Host → Sign → Broadcast → Verify → Apply` on every peer.  
* **Host Reclaim:** Original host proves ownership with its stored host key.  
* **Testing:** Simulate multiple clients, verify signatures, test host delegation and spectator mode.  
* **Architecture Docs:** See `docs/*.adoc` for full DDD process, EventStorming, Context Maps, Domain Message Flows.

---  

### TL;DR for the Agent  

*The library is a **client-side only** solution that leverages a **sign-and-broadcast** pattern via a **Matchbox signalling server**. The host's cryptographically signed messages drive the whole session, and every peer must verify those signatures before mutating state. All design decisions revolve around **decentralization**, **authenticity**, and **host-driven truth**.*  

**Key architectural principles:**
- Domain-Driven Design with clear bounded contexts
- Hexagonal/Onion architecture (domain → application → infrastructure)
- Published library crates (`konnekt-session-core`, `konnekt-session-yew`)
- Activities provided by consuming apps via trait implementation
- Host/Guest roles independent from Active/Spectating participation modes

Use this context to guide any code you generate, ensuring it respects:
1. The host-authority model (not admin)
2. The P2P signing contract (Ed25519)
3. The DDD boundaries outlined in `docs/*.adoc`
4. The layered architecture (domain has zero dependencies)
5. The separation of concerns (lobby role vs participation mode)

---
