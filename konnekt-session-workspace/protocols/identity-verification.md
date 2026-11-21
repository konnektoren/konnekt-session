---
title: Identity verification
type: protocol
status: draft
---

# Identity verification

Either peer can create an `IdentityChallengeVerifier` with a lobby, peer,
monotonic issuance time and validity period. The challenge uses a fresh random
nonce. The participant signs the versioned `signing_bytes`; the verifier checks
the sender, deadline and Ed25519 signature and consumes the challenge. An
invalid proof, expiry or disconnect ends that challenge. A successful result
contains the lobby, peer and verified public key.

`SessionLoopV2` can opt into exchanging challenges using an application-supplied
key and exposes the public key proven by each connected peer. The music adapter
uses an ephemeral key for each session instance; it does not persist identity.
The exchange does not gate admission or commands. Proof of key possession alone
does not authorize an action, establish
trust in the host, provide durable participant identity, sign later messages,
or prevent replay of commands after a host restart. The application must own its key; key
generation, storage and recovery are not implemented here.

Before enabling authenticated guest outcomes, bind verified keys to
participants, authenticate the host and subsequent requests and replies, and
define reconnect, duplicate-key and request-deduplication policies. See the
[generated diagram](identity-verification-diagram.md).
