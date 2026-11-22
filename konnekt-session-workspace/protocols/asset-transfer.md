---
title: Asset transfer
type: protocol
status: draft
tags: [protocol, asset-transfer]
---
# Asset transfer

One machine tracks one transfer between a sender and receiver. It advances
from preparation through chunk transfer and completion acknowledgement, or
ends on timeout or cancellation. Each endpoint owns its own machine.

The caller validates chunk identity, bounds, content and peer identity, tracks
which chunks are present, sends and matches acknowledgements, and decides
retry and timeout policy. On the receiver, completion means the confirmation
was sent; on the sender, it means the matching confirmation was received.
Neither guarantees durable storage or delivery of the confirmation.

[[asset-transfer-diagram|Generated diagram]]

The core application layer provides `AssetSender` and `AssetReceiver` for
bounded in-memory transfers. The default limit is 10 MiB with 4 KiB chunks;
callers may configure both. The caller sends chunks and confirmations and
checks identity and asset contents. The music app currently uses its own
transfer helpers around the same state machine.

## Remaining work

- [x] Connect the music app's MIDI transfer and its existing chunk validation.
- [ ] Adopt the shared sender and receiver in the music app.
- [ ] Define retry, acknowledgement-loss and timeout policies.
- [ ] Add integration and multi-peer transfer scenarios.
