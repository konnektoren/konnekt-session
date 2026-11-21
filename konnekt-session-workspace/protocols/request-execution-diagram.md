---
title: Request execution diagram
type: protocol
---

# Request execution

```mermaid
stateDiagram-v2
    [*] --> Pending
    Accepted --> [*]
    Rejected --> [*]
    Pending --> Accepted : accepted
    Pending --> Rejected : rejected
```
