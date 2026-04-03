---
title: Lobby Role
type: concept
tags: [concept, host, guest, authority]
---

# Lobby Role

Determines a participant's **authority** within the [[../domain/lobby|Lobby]].

## Roles

| Role | Capabilities |
|------|-------------|
| `Host` | Kick guests, start/stop activities, delegate role, sign state changes |
| `Guest` | Join, play activities, toggle participation mode |

## Rules

- Exactly **one** host at all times.
- Host is the **only** source of signed, authoritative state updates.
- Role transfer via [[host-delegation|Host Delegation]] (manual or automatic).

## Independent from Participation Mode

See [[participation-modes|Participation Modes]] — a host can be Spectating, a guest can be Active.

## See Also

- [[../domain/participant|Participant]]
- [[host-delegation|Host Delegation]]
