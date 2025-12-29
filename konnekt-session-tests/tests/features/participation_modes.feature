Feature: Participation Modes
  As documented in docs/02-discover.adoc - Participation Mode

  Guests can toggle between Active and Spectating modes.
  Spectators can watch but cannot submit results.

  Background:
    Given a lobby exists with a host
    And guest "Carol" has joined in Active mode

  Scenario: Guest toggles to Spectating before activity
    Given no activity is running
    When "Carol" toggles to Spectating mode
    Then "Carol" should be in Spectating mode
    And a ParticipationModeChanged event should be broadcast

  Scenario: Guest cannot toggle during activity
    Given an activity is in progress
    When "Carol" tries to toggle to Spectating mode
    Then the toggle should be rejected
    And the error should be "Cannot change mode during activity"

  Scenario: Spectator cannot submit results
    Given "Carol" is in Spectating mode
    And an activity is in progress
    When "Carol" tries to submit a result
    Then the submission should be rejected
    And the error should be "Spectators cannot submit results"

  Scenario: Host forces guest to Spectating mode
    Given "Carol" is in Active mode
    When the host forces "Carol" to Spectating mode
    Then "Carol" should be in Spectating mode
    And a ParticipationModeChanged event should be broadcast with forced=true

  Scenario: Toggle back to Active after activity
    Given "Carol" is in Spectating mode
    And the previous activity has completed
    When "Carol" toggles to Active mode
    Then "Carol" should be in Active mode
    And can participate in the next activity

  Scenario: Host can be in Spectating mode
    Given the host is in Active mode
    When the host toggles to Spectating mode
    Then the host should be in Spectating mode
    And can still manage the lobby

  Scenario: New guests join in Active mode by default
    Given a lobby exists
    When a new guest joins
    Then the guest should be in Active mode

  Scenario: Activity completion excludes spectators
    Given an activity is in progress
    And host is Active (submitted)
    And "Alice" is Active (submitted)
    And "Bob" is Active (not submitted)
    And "Carol" is Spectating
    When "Bob" submits
    Then the activity should complete

  Scenario: All guests spectating
    Given 3 guests in the lobby
    And all guests are in Spectating mode
    When the host starts an activity
    Then only the host can submit
    And the activity completes when host submits
