---
title: Join session
type: protocol
status: draft
tags: [protocol, join, admission, spectator]
---
# Join session

A V2 guest connects, asks the host for admission and synchronizes the lobby
before it is `Ready`. The machine is `konnekt-session-p2p/src/protocol/join.rs`.

[[join-session-diagram|Generated diagram]]

## Admission request

The guest builds its own `Participant` once and sends it as
`DomainCommand::AddParticipant` until the host admits it. The ID stays the same
across retries and reconnects, so the host links a returning connection instead
of adding a new participant.

The participant carries its requested
[[../concepts/participation-modes|participation mode]]:

| Call | Joins as |
|---|---|
| `SessionLoopV2::join(name)` | `Active` (player) |
| `SessionLoopV2::join_as(name, ParticipationMode::Spectating)` | `Spectating` (observer) |

The mode is part of the participant, so it needs no extra message. It survives
snapshots, so peers that join later see the same mode. A spectator follows the
lobby and its runs, is never a required submitter and is never given the host
role (see [[host-failover]]).

The legacy `DomainCommand::JoinLobby` (CLI, Yew, Bevy) still always joins as
`Active`.

## Invariants

- The requested mode is applied as sent; the host doesn't change it on admission.
- Admission doesn't depend on the mode. Seat limits are not enforced yet.

## Remaining work

- [ ] Add a lobby policy (`max_active`, `max_spectators`, `spectators_allowed`,
  `spectators_may_switch`). Turn an `Active` request beyond `max_active` into a
  rejection or a spectator admission, so the host can lock a room once its seats
  are filled.
- [ ] Prevent spectators from switching to `Active` when the policy forbids it
  (today `ToggleParticipationMode` allows it between activities).
- [ ] Bind seats to verified keys (see [[identity-verification]]), so only the
  expected players can take an `Active` seat.
- [ ] Add the requested mode to the legacy `JoinLobby` command.
