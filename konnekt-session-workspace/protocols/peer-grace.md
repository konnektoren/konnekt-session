---
title: Peer grace period
type: protocol
status: draft
tags: [protocol, reconnect]
---
# Peer grace period

The legacy P2P peer registry gives a disconnected transport peer a grace period.
It reports timeout once, or accepts a new connection before timeout is processed.
The registry currently creates fresh peer metadata on connection. Duplicate
notifications do not restart the period.

[[peer-grace-diagram|Generated diagram]]

This machine does not restore participant membership, prove identity, synchronize
state, or transfer host authority. The V2 runtime does not use this registry;
full rejoin and host failover remain planned.
