# AGENTS.md  

## Purpose – Why This Project Exists  

Konnekt Session is a **Rust‑centric, peer‑to‑peer multiplayer‑lobby library** whose primary ambition is to **eliminate any central game‑server bottleneck**.  

* The only external service required is a **Matchbox signalling node** (currently hosted at `match.konnektoren.help`).  
* All **game state lives on the client** – the lobby, activities, scores, and player roles are fully replicated on each participant.  
* An **admin client is the authoritative source** of truth; its updates are signed and broadcast to every peer, guaranteeing a consistent view without consensus protocols.  
* The design is **extensible** (separate crates for game‑specific logic) and **Web‑first** – the UI is written in **Yew** (Rust + WebAssembly) and runs entirely in the browser.

### Core Design Pillars  

| Pillar | What it means for the system |
|--------|------------------------------|
| **Domain‑Driven Design** | Clear bounded contexts: `SessionManagement`, `ActivityManagement`, `UserManagement`, `Signalling`. The `Lobby` aggregate is the aggregate root. |
| **Decentralised State** | No persistent server‑side game state. Only connection metadata is stored in‑memory; the authoritative lobby state is owned by the admin and streamed peer‑to‑peer. |
| **Admin‑as‑Truth** | Admin commands (e.g., *StartActivity*, *KickPlayer*) are signed with a private key unique to that client. Peers accept updates only from the admin identity. |
| **Admin Election & Reclaim** | If the admin disconnects, the oldest remaining peer becomes *temporary* admin. When the original admin returns it can reclaim the role by presenting its stored admin key. |
| **WebRTC‑based P2P** | Matchbox only handles the WebRTC handshake; the actual **data channel** carries signed lobby events between peers. |

---  

## High‑Level Architecture (C4 Overview)  

```
+-----------------------------------------------------------+
|                     Client (Browser)                     |
|  ┌─────────────┐   ┌───────────────────────┐   ┌───────┐ |
|  │   Yew SPA   │←→│  WebSocket (Matchbox)  │←→│ Admin │ |
|  │ (UI, State)│   │  (wss://match.…)      │   │ Role  │ |
|  └─────▲───────┘   └───────▲───────▲───────┘   └───────┘ |
|        │                   │                   │
|        │   Signed Lobby   │   P2P Data Channel   │
|        └───────────────────┴───────────────────────┘
+-----------------------------------------------------------+

+-----------------------------------------------------------+
|                Matchbox Signalling Service                |
|  (only WebRTC signalling – no game state)                 |
+-----------------------------------------------------------+
```

*The diagram above is deliberately high‑level – it shows the separation of concerns without diving into concrete command names or binary formats.*

---  

## Agent Playbook – How to Work on Konnekt Session  

### 1. Mindset  
* Treat **every client as a first‑class participant** that holds a full copy of the lobby.  
* The **admin** is the *only* source of authoritative updates. All other peers are *receivers* that apply those updates.  
* Authentication is **cryptographic** – every message is signed with the sender’s `PrivateKey`; verification is mandatory before state change.  

### 2. Typical Development Flow  
1. **Spin up the signalling endpoint** you want to test (`match`, `helsing`, or a local `matchbox_server`).  
2. **Start the SPA** (`trunk serve`) with the appropriate `WEBSOCKET_URL`.  
3. **Open multiple browser windows** – one will become the admin automatically; the others will join as players.  
4. **Interact** – create a lobby, add activities, start them. Observe how the admin‑signed events flow to all windows.  

### 3. Key Artifacts to Touch  
| Artifact | Location | Why it matters |
|----------|----------|----------------|
| `src/model/` | Domain entities (`Lobby`, `Player`, `Activity`), traits (`Identifiable`, `Scorable`). | Core DDD building blocks. |
| `src/p2p/` | Message envelope (`P2PMessage`), signing/verification logic. | Guarantees authenticity of every broadcast. |
| `src/auth/` | `PrivateKey` generation, storage, admin‑key handling. | Enables stable identity and admin reclaim. |
| `src/components/` | Yew UI components that render lobby, activities, player list. | UI reflects the signed state updates. |
| `design.adoc` | Full C4 diagrams, context map, state‑sync explanation. | Reference for architectural decisions. |

### 4. Testing Strategy  
* **Unit tests** for domain types (`Lobby`, `Activity`).  
* **Integration tests** that spin up a simulated P2P network and verify that admin‑signed events are correctly propagated and applied.  
* **End‑to‑end manual tests** using multiple browser tabs – close the admin window to see temporary‑admin election, then reopen to see reclaim.  

### 5. Non‑Functional Constraints  
* **No server‑side game state** – any persistence must be client‑side (e.g., `localStorage`).  
* **All state changes must be signed** by the current admin before they are considered valid.  
* **Zero‑trust network** – any peer may join; the only gatekeeper is the admin’s signature.  

---  

## What **Not** to Do  

* **Don’t add new server‑side storage** – it breaks the decentralised model.  
* **Don’t expose raw WebSocket frames** to the UI; always go through the signed `P2PMessage` envelope.  
* **Don’t change the admin election algorithm** without consulting the existing election protocol; it is carefully designed to avoid dead‑locks.  
* **Don’t commit secret keys** – they are derived from `localStorage` and must stay private to the client.  

---  

## Quick Reference (What the Agent Should Remember)  

* **Goal:** Provide a *decentralised* lobby where an admin’s signed updates are the **single source of truth** for all participants.  
* **Key Actor:** `Lobby` aggregate root, owned by the admin client.  
* **Message Flow:** `Admin → Sign → Broadcast → Verify → Apply` on every peer.  
* **Admin Reclaim:** Original admin proves ownership with its stored admin key.  
* **Testing:** Simulate multiple clients, verify signatures, and watch the admin election/reclaim behaviour.  

---  

### TL;DR for the Agent  

*The library is a **client‑side only** solution that leverages a **sign‑and‑broadcast** pattern via a **Matchbox signalling server**. The admin’s cryptographically signed messages drive the whole session, and every peer must verify those signatures before mutating state. All design decisions revolve around **decentralisation**, **authenticity**, and **admin‑driven truth**.*  

Use this context to guide any code you generate, ensuring it respects the admin‑authority model, the P2P signing contract, and the DDD boundaries outlined in `design.adoc`.  

---
