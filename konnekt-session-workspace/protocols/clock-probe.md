---
title: Clock probe
type: protocol
status: draft
tags: [protocol, clock]
---
# Clock probe

A clock probe tracks one request from preparation through a validated sample,
timeout or disconnect. A fresh probe uses a fresh machine. The caller matches
replies to outstanding requests, validates timestamps, calculates samples and
chooses when to refresh. [[clock-probe-diagram|Generated diagram]]

The music app allows one outstanding probe, refreshes every second, and expires
it after one second; it retains the eight most recent samples.

## Remaining work

- [x] Use the machine in the music live link and match sequence and send time.
- [ ] Define refresh, timeout and sample retention for multiple games.
- [x] Test delayed, duplicate, unsolicited, malformed and out-of-order replies.
- [ ] Define authentication and synchronized-start behavior separately.
