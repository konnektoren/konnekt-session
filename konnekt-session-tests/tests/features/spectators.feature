Feature: Joining as a spectator
  A guest can join a lobby as a spectator (observer). Spectators follow the
  lobby and its runs, are never required to submit results, and never become
  host, neither by delegation nor by automatic election or failover.

  Background:
    Given a lobby exists with a host

  Scenario: A guest joins as a spectator
    When guest "Olivia" joins as a spectator
    Then "Olivia" should be in Spectating mode
    And "Olivia" should not be the host

  Scenario: Guests still join as players by default
    When guest "Bob" joins as a player
    Then "Bob" should be in Active mode

  Scenario: Spectators are not required to submit results
    Given guest "Bob" joined as a player
    And guest "Olivia" joined as a spectator
    When the host starts an activity
    Then "Bob" should be a required submitter
    And "Olivia" should not be a required submitter

  Scenario: The host cannot hand over to a spectator
    Given guest "Olivia" joined as a spectator
    When the host delegates to "Olivia"
    Then the delegation should be rejected
    And the error should be "Cannot delegate host to a spectator"

  Scenario: Automatic host election skips spectators
    Given guest "Olivia" joined as a spectator
    And guest "Bob" joined as a player
    When the host role is assigned automatically
    Then the automatic host should be "Bob"

  Scenario: No automatic host when only spectators remain
    Given guest "Olivia" joined as a spectator
    When the host role is assigned automatically
    Then no automatic host should be found

  Scenario: Spectators are not failover candidates
    Given guest "Olivia" joined as a spectator
    And guest "Bob" joined as a player
    Then the failover candidates should be "Bob"
