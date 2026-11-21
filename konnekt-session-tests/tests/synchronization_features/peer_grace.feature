Feature: Peer disconnect grace period
  A disconnected peer is given time to reconnect before it is reported gone.

  Scenario: Reconnect before timeout
    Given a connected peer enters the grace period
    When the peer reconnects before the grace period is checked
    Then the peer remains connected

  Scenario: Report a peer after the grace period
    Given a connected peer enters the grace period
    When the disconnected peer's grace period expires
    Then the peer is reported timed out once
