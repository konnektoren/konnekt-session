---
title: Participant
type: domain-entity
tags: [domain, entity, ddd]
source: konnekt-session-core/src/domain/participant.rs
---

# Participant

Entity representing a connected player in a [[lobby|Lobby]].

## Fields

| Field | Type | Values |
|-------|------|--------|
| `id` | UUID | — |
| `lobby_role` | `LobbyRole` | `Host` \| `Guest` |
| `participation_mode` | `ParticipationMode` | `Active` \| `Spectating` |

## Two Independent Concerns

```mermaid
graph TD
    P[Participant]
    P --> LR[LobbyRole: Host / Guest]
    P --> PM[ParticipationMode: Active / Spectating]
    LR -->|authority| Mgmt[Can kick, start, delegate]
    PM -->|participation| Play[Can submit results]
```

> A host can be Spectating. A guest can be Active. The two are independent,
> with one exception: the host role is only *given* to an Active participant.
> A host who later switches to Spectating stays host.

## Rules

- `LobbyRole::Host` → single; transfers via [[../concepts/host-delegation|Host Delegation]], only to `Active` participants.
- `ParticipationMode` → chosen on join (`Participant::with_participation_mode`); default `Active`.
- `ParticipationMode` → cannot change during an active activity.
- Only `Active` participants may submit `ActivityResult`.

## See Also

- [[../concepts/participation-modes|Participation Modes]]
- [[../concepts/lobby-role|Lobby Role]]
