---
title: Peer grace period diagram
type: protocol
---

# Peer grace period

```mermaid
stateDiagram-v2
    [*] --> Connected
    TimedOut --> [*]
    Connected --> Disconnected : disconnected
    Disconnected --> Connected : reconnected
    Disconnected --> TimedOut : grace_expired
```
