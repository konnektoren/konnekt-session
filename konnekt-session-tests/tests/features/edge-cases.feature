Feature: Edge Cases and Error Handling

  Scenario: All guests switch to Spectating
    Given 3 guests in Active mode
    When all guests toggle to Spectating
    And the host tries to start an activity
    Then a warning should be shown: "No active participants"
    But the activity can still start
    And only the host can submit

  Scenario: Host and all guests disconnect
    Given a lobby with 3 guests
    When the host disconnects
    And all guests disconnect within the grace period
    Then the lobby should close automatically

  Scenario: New guest joins during host delegation timeout
    Given the host disconnected at T
    And 15 seconds have passed
    When a new guest tries to join
    Then the join should succeed
    But the new guest is not eligible for host election
    And the oldest original guest becomes host

  Scenario: Simultaneous host delegation attempts
    Given the host disconnects
    And two guests detect the timeout simultaneously
    When both try to claim host role
    Then the oldest guest wins
    And the other guest's claim is rejected

  Scenario: Activity result submission after activity ends
    Given an activity has completed
    When a late participant tries to submit
    Then the submission should be rejected
    And the error should be "Activity already completed"

  Scenario: Guest leaves during activity
    Given an activity is in progress with 3 active participants
    When one guest leaves
    Then the activity can still complete with 2 participants

  Scenario: Host kicks guest mid-activity
    Given an activity is in progress
    When the host kicks a participating guest
    Then the guest is removed
    And their partial result is discarded
    And the activity continues

  Scenario: Empty lobby after all guests kicked
    Given a lobby with 3 guests
    When the host kicks all guests
    Then the lobby remains open
    And the host is alone
    And can still invite new guests

  Scenario: Password change while guests are in lobby
    Given a lobby with 3 guests
    When the host changes the lobby password
    Then existing guests remain in the lobby
    But new guests must use the new password
