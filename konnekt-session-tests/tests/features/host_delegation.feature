Feature: Host Delegation
  As documented in docs/02-discover.adoc - Host Delegation Protocol

  Automatic host delegation on 30s disconnect timeout.
  Oldest guest becomes host (deterministic election).

  Background:
    Given a lobby exists with a host
    And guest "Alice" joined 10 seconds ago
    And guest "Bob" joined 5 seconds ago

  Scenario: Manual host delegation
    When the host delegates to "Alice"
    Then "Alice" should become the host
    And the original host should become a guest
    And a HostDelegated event should be broadcast with reason "Manual"

  Scenario: Host disconnect with 30s timeout
    Given the host disconnects at time T
    When 10 seconds pass
    Then the host should be marked as "suspected disconnect"
    When 20 more seconds pass
    Then "Alice" should become the host
    And a HostDelegated event should be broadcast with reason "Timeout"

  Scenario: Host reconnects within grace period
    Given the host disconnects at time T
    When 15 seconds pass
    And the host reconnects
    Then the host should retain their role
    And no delegation should occur

  Scenario: Host reconnects after delegation
    Given the host disconnects at time T
    When 30 seconds pass
    And "Alice" becomes the host
    And the original host reconnects
    Then the original host should rejoin as a guest
    And "Alice" should remain the host

  Scenario: Oldest guest election
    Given the host disconnects
    When the 30s timeout expires
    Then "Alice" should become the host
    And not "Bob"

  Scenario: Tie-breaking with identical timestamps
    Given guest "Alice" joined at T
    And guest "Bob" joined at T
    When the host disconnects
    Then the guest with the lowest UUID becomes host

  Scenario: Single guest auto-promotion
    Given a lobby with only the host and 1 guest
    When the host disconnects
    Then the guest should immediately become host

  Scenario: Empty lobby closure
    Given a lobby with only the host
    When the host disconnects
    Then the lobby should close automatically

  Scenario: Delegation during activity
    Given an activity is in progress
    And the host disconnects
    When the 30s timeout expires
    Then "Alice" should become the host
    And the activity should continue
    And "Alice" can manage the activity

  Scenario: No auto-reclaim after delegation
    Given the host was delegated due to timeout
    When the original host reconnects
    Then they rejoin as a guest
    And cannot auto-reclaim the host role
    But the new host can manually delegate back
