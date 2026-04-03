---
title: Konnekt Session
type: home
tags: [konnekt-session, index]
---

# Konnekt Session

Rust-based, decentralized multiplayer-lobby library. No central game server — all state lives on the client.

## Core Concepts

- [[domain/lobby|Lobby]] — aggregate root, single source of truth per session
- [[domain/participant|Participant]] — has a [[concepts/lobby-role|Lobby Role]] and a [[concepts/participation-modes|Participation Mode]]
- [[domain/activity|Activity]] — state machine: Planned → InProgress → Completed

## Architecture

- [[architecture/overview|Architecture Overview]] — C4 context, bounded contexts
- [[architecture/domain-model|Domain Model]] — entities, aggregates, value objects
- [[architecture/p2p-flow|P2P Message Flow]] — sign → broadcast → verify → apply

## Key Concepts

- [[concepts/host-delegation|Host Delegation]] — 30s timeout, oldest guest elected
- [[concepts/p2p-signing|P2P Signing]] — Ed25519, every message signed by host
- [[concepts/participation-modes|Participation Modes]] — Active vs Spectating (independent of role)

## Decisions

- [[adr/index|ADR Index]] — 23 architecture decision records

## Links

- Source: `konnekt-session-core/`, `konnekt-session-yew/`
- Docs: `docs/*.adoc`, `docs/adr/`
