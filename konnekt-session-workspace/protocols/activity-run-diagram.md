---
title: Activity run diagram
type: protocol
---

# Activity run

```mermaid
stateDiagram-v2
    [*] --> InProgress
    Completed --> [*]
    Cancelled --> [*]
    InProgress --> Completed : completed
    InProgress --> Cancelled : cancelled
```
