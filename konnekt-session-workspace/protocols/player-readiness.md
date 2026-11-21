---
title: Player readiness
type: protocol
status: draft
tags: [protocol, readiness]
---
# Player readiness

A machine tracks whether one player is ready for the current activity. Repeated
ready and not-ready messages are idempotent. The caller resets readiness when
the selected activity changes or a run starts.

[[player-readiness-diagram|Generated diagram]]

## Remaining work

- [x] Use one machine per player in the music app lobby.
- [ ] Integrate readiness into a shared session API.
- [x] Bind music app readiness messages to a host-issued selection revision.
- [ ] Add ordering or replay protection for queue announcements and same-revision updates.
- [ ] Define compatibility behavior for peers without revision support.
- [ ] Define membership, disconnect, authentication and host start policy.
