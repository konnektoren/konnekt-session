Feature: Peer key possession
  Connected players can prove possession of their session keys.

  Scenario: Both Players Prove Their Keys
    Given a guest has synchronized with the host
    When both players exchange identity proofs
    Then each player can see the peer's proven key
