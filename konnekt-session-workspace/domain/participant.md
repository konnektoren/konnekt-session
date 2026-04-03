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

> A host can be Spectating. A guest can be Active. These are orthogonal.

## Rules

- `LobbyRole::Host` → single; transfers via [[../concepts/host-delegation|Host Delegation]].
- `ParticipationMode` → cannot change during an active activity.
- Only `Active` participants may submit `ActivityResult`.

## See Also

- [[../concepts/participation-modes|Participation Modes]]
- [[../concepts/lobby-role|Lobby Role]]
