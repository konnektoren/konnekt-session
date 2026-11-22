---
title: Synchronized start
type: protocol
status: draft
tags: [protocol, timing]
---
# Synchronized start

A sender tracks one start announcement until all required peers acknowledge,
it times out, or it is cancelled. The caller owns announcement and run IDs,
peer membership, clock conversion, delivery, authentication, and timeout
policy. Acknowledgement does not guarantee playback began on time.

[[synchronized-start-diagram|Generated diagram]]

The music app announces an already-started count-in and acknowledges a matching
run and sequence. The sender retries every 500 ms and times out after five
seconds. This does not schedule a common future start.

## Remaining work

- [ ] Specify scheduled start times, matching acknowledgements, retries and deadlines.
- [x] Connect the music live link and test acknowledgement, retries and timeout.
- [x] Cancel pending announcements on disconnect or leaving the run; ignore late acknowledgements.
- [ ] Define scheduling and start-without-all-peers policy.
